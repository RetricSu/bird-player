/// Audio playback state machine
///
/// 使用状态模式重构音频线程，每个状态独立封装自己的逻辑
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use symphonia::core::codecs::Decoder;
use symphonia::core::formats::FormatReader;

use super::loader;
use super::utils;

use crate::audio::output::AudioOutput;
use crate::AudioEvent;

/// 状态转换结果
pub enum Transition {
    /// 保持当前状态
    Stay,
    /// 转换到新状态
    To(Box<dyn State>),
}

/// 音频上下文 - 所有状态共享的数据
pub struct AudioContext {
    pub engine: AudioEngineState,
    pub decoder: Option<Box<dyn Decoder>>,
    pub volume: f32,
    pub current_track_path: Option<PathBuf>,
    pub ui_tx: Sender<AudioEvent>,
    pub timer: std::time::Instant,
    pub last_ts: u64,
}

/// 音频引擎状态
pub struct AudioEngineState {
    pub reader: Option<Box<dyn FormatReader>>,
    pub audio_output: Option<Box<dyn AudioOutput>>,
    pub track_num: Option<usize>,
    pub seek: Option<SeekPosition>,
    pub decode_opts: Option<symphonia::core::codecs::DecoderOptions>,
    pub track_info: Option<PlayTrackOptions>,
    pub duration: u64,
    pub timebase: u64,
}

#[derive(Copy, Clone)]
pub struct PlayTrackOptions {
    pub track_id: u32,
    pub seek_ts: u64,
}

pub enum SeekPosition {
    Timestamp(u64),
}

/// 状态 trait - 所有播放状态必须实现
pub trait State: Send {
    /// 进入状态时调用（可选）
    fn enter(&mut self, _ctx: &mut AudioContext) {}

    /// 状态更新 - 每帧调用，返回状态转换
    fn update(&mut self, ctx: &mut AudioContext) -> Transition;

    /// 离开状态时调用（可选）
    fn exit(&mut self, _ctx: &mut AudioContext) {}

    /// 获取状态名称（用于调试）
    fn name(&self) -> &'static str;
}

// ==================== 具体状态实现 ====================

/// 未开始状态
pub struct UnstartedState;

impl State for UnstartedState {
    fn update(&mut self, _ctx: &mut AudioContext) -> Transition {
        std::thread::sleep(std::time::Duration::from_millis(100));
        Transition::Stay
    }

    fn name(&self) -> &'static str {
        "Unstarted"
    }
}

/// 停止状态
pub struct StoppedState;

impl State for StoppedState {
    fn enter(&mut self, ctx: &mut AudioContext) {
        // 刷新音频输出缓冲区
        if let Some(audio_output) = ctx.engine.audio_output.as_mut() {
            tracing::info!("Audio Thread Stopped - flushing output");
            audio_output.flush();
        }
    }

    fn update(&mut self, ctx: &mut AudioContext) -> Transition {
        if let Some(ref current_track_path) = ctx.current_track_path {
            // 完成当前解码器
            if let Some(decoder) = ctx.decoder.as_mut() {
                _ = utils::do_verification(decoder.finalize());
            }

            if let Some(audio_output) = ctx.engine.audio_output.as_mut() {
                audio_output.flush();
            }

            ctx.engine.audio_output = None;

            loader::load_file(current_track_path, &mut ctx.engine, &mut ctx.decoder, 0, &ctx.ui_tx);

            ctx.ui_tx
                .send(AudioEvent::CurrentTimestamp(0))
                .expect("Failed to send timestamp to ui thread");

            return Transition::To(Box::new(UnstartedState));
        }

        Transition::Stay
    }

    fn name(&self) -> &'static str {
        "Stopped"
    }
}

/// 暂停状态
pub struct PausedState;

impl State for PausedState {
    fn update(&mut self, _ctx: &mut AudioContext) -> Transition {
        std::thread::sleep(std::time::Duration::from_millis(50));
        Transition::Stay
    }

    fn name(&self) -> &'static str {
        "Paused"
    }
}

/// 播放状态
pub struct PlayingState;

impl State for PlayingState {
    fn update(&mut self, ctx: &mut AudioContext) -> Transition {
        let result: std::result::Result<(), symphonia::core::errors::Error> = 'decode: {
            // 检查是否有有效的 reader 和 track info
            let reader = match ctx.engine.reader.as_mut() {
                Some(reader) => reader,
                None => {
                    tracing::warn!(
                        "AudioThread Playing - No reader available, switching to stopped"
                    );
                    ctx.ui_tx
                        .send(AudioEvent::AudioFinished)
                        .expect("Failed to send audio finished to ui thread");
                    return Transition::To(Box::new(StoppedState));
                }
            };

            let play_opts = match ctx.engine.track_info {
                Some(opts) => opts,
                None => {
                    tracing::warn!(
                        "AudioThread Playing - No track info available, switching to stopped"
                    );
                    ctx.ui_tx
                        .send(AudioEvent::AudioFinished)
                        .expect("Failed to send audio finished to ui thread");
                    return Transition::To(Box::new(StoppedState));
                }
            };

            // 获取下一个数据包
            let packet = match reader.next_packet() {
                Ok(packet) => packet,
                Err(_err) => {
                    tracing::warn!("couldn't decode next packet");
                    ctx.ui_tx
                        .send(AudioEvent::AudioFinished)
                        .expect("Failed to send audio finished to ui thread");
                    return Transition::To(Box::new(StoppedState));
                }
            };

            // 检查数据包是否属于当前音轨
            if packet.track_id() != play_opts.track_id {
                tracing::warn!("packet track id doesn't match track id");
                break 'decode Ok(());
            }

            // 发送时间戳更新
            let current_time = ctx.timer.elapsed();
            if current_time > std::time::Duration::from_millis(100)
                && (packet.ts > ctx.last_ts + 100 || packet.ts < ctx.last_ts)
            {
                let timestamp_ms = if ctx.engine.timebase > 0 {
                    ((packet.ts() as f64 / ctx.engine.timebase as f64) * 1000.0) as u64
                } else {
                    packet.ts()
                };

                ctx.ui_tx
                    .send(AudioEvent::CurrentTimestamp(timestamp_ms))
                    .expect("Failed to send timestamp to ui thread");

                ctx.timer = std::time::Instant::now();
                ctx.last_ts = packet.ts();
            }

            // 解码数据包
            match ctx.decoder.as_mut().unwrap().decode(&packet) {
                Ok(decoded) => {
                    // 如果音频输出未打开，尝试打开它
                    if ctx.engine.audio_output.is_none() {
                        let spec = *decoded.spec();
                        let duration = decoded.capacity() as u64;
                        ctx.engine
                            .audio_output
                            .replace(crate::audio::output::try_open(spec, duration).unwrap());
                    }

                    // 写入解码后的音频样本
                    if packet.ts() >= play_opts.seek_ts {
                        if let Some(audio_output) = ctx.engine.audio_output.as_mut() {
                            audio_output.write(decoded, ctx.volume).unwrap();
                        }
                    }

                    Ok(())
                }
                Err(symphonia::core::errors::Error::DecodeError(err)) => {
                    tracing::warn!("decode error: {}", err);
                    break 'decode Ok(());
                }
                Err(err) => break 'decode Err(err),
            }
        };

        // 处理错误
        if let Err(err) = utils::ignore_end_of_stream_error(result) {
            tracing::error!("Fatal error in playing state: {}", err);
        }

        Transition::Stay
    }

    fn name(&self) -> &'static str {
        "Playing"
    }
}

/// 加载文件状态
pub struct LoadFileState {
    path: PathBuf,
}

impl LoadFileState {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl State for LoadFileState {
    fn enter(&mut self, ctx: &mut AudioContext) {
        tracing::info!("AudioThread Loading File");

        // 停止当前播放
        if let Some(audio_output) = ctx.engine.audio_output.as_mut() {
            tracing::info!("AudioThread Loading File - Flushing output");
            audio_output.flush();
        }

        // 完成当前解码器
        if let Some(decoder) = ctx.decoder.as_mut() {
            _ = utils::do_verification(decoder.finalize());
        }

        ctx.engine.audio_output = None;
    }

    fn update(&mut self, ctx: &mut AudioContext) -> Transition {
        ctx.current_track_path = Some(self.path.clone());
        loader::load_file(&self.path, &mut ctx.engine, &mut ctx.decoder, 0, &ctx.ui_tx);

        // 检查加载是否成功
        if ctx.engine.reader.is_some() && ctx.engine.track_info.is_some() {
            ctx.ui_tx
                .send(AudioEvent::TotalTrackDuration(ctx.engine.duration))
                .expect("Failed to send track duration to ui thread");

            Transition::To(Box::new(PlayingState))
        } else {
            tracing::warn!("Failed to load audio file: {:?}", self.path);
            ctx.ui_tx
                .send(AudioEvent::AudioFinished)
                .expect("Failed to send audio finished to ui thread");
            ctx.current_track_path = None;
            Transition::To(Box::new(StoppedState))
        }
    }

    fn name(&self) -> &'static str {
        "LoadFile"
    }
}

/// Seek 状态
pub struct SeekToState {
    timestamp: u64,
}

impl SeekToState {
    pub fn new(timestamp: u64) -> Self {
        Self { timestamp }
    }
}

impl State for SeekToState {
    fn enter(&mut self, ctx: &mut AudioContext) {
        tracing::info!("AudioThread Seeking to {}", self.timestamp);

        // 停止当前播放
        if let Some(audio_output) = ctx.engine.audio_output.as_mut() {
            audio_output.flush();
        }

        ctx.engine.audio_output = None;
    }

    fn update(&mut self, ctx: &mut AudioContext) -> Transition {
        if let Some(ref current_track_path) = ctx.current_track_path {
            loader::load_file(
                current_track_path,
                &mut ctx.engine,
                &mut ctx.decoder,
                self.timestamp,
                &ctx.ui_tx,
            );

            // 检查加载是否成功
            if ctx.engine.reader.is_some() && ctx.engine.track_info.is_some() {
                ctx.ui_tx
                    .send(AudioEvent::PlaybackStateChanged(true))
                    .expect("Failed to send playback state to ui thread");

                return Transition::To(Box::new(PlayingState));
            } else {
                tracing::warn!("Failed to reload audio file during seek");
                ctx.ui_tx
                    .send(AudioEvent::AudioFinished)
                    .expect("Failed to send audio finished to ui thread");
                return Transition::To(Box::new(StoppedState));
            }
        }

        Transition::Stay
    }

    fn name(&self) -> &'static str {
        "SeekTo"
    }
}

/// 状态机 - 管理状态转换
pub struct StateMachine {
    current_state: Box<dyn State>,
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl StateMachine {
    pub fn new() -> Self {
        Self {
            current_state: Box::new(UnstartedState),
        }
    }

    /// 运行一次状态更新
    pub fn update(&mut self, ctx: &mut AudioContext) {
        match self.current_state.update(ctx) {
            Transition::Stay => {
                // 保持当前状态
            }
            Transition::To(new_state) => {
                // 状态转换
                tracing::debug!(
                    "State transition: {} -> {}",
                    self.current_state.name(),
                    new_state.name()
                );

                self.current_state.exit(ctx);
                self.current_state = new_state;
                self.current_state.enter(ctx);
            }
        }
    }

    /// 从外部命令触发状态转换
    pub fn transition_to(&mut self, new_state: Box<dyn State>, ctx: &mut AudioContext) {
        tracing::debug!(
            "External state transition: {} -> {}",
            self.current_state.name(),
            new_state.name()
        );

        self.current_state.exit(ctx);
        self.current_state = new_state;
        self.current_state.enter(ctx);
    }

    /// 获取当前状态名称
    pub fn current_state_name(&self) -> &'static str {
        self.current_state.name()
    }
}
