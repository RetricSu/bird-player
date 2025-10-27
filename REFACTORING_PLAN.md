# App mod.rs 重构计划

## 当前问题

1. **文件过大**: 1100+ 行代码，难以维护
2. **职责过多**: App 结构体包含30+字段，职责不清晰
3. **方法过长**: 多个方法超过50行
4. **紧耦合**: 数据库、UI、播放器逻辑混在一起

## 重构目标

将代码按职责分离到不同的模块中，保持功能不变的同时提高可维护性。

## 已完成的工作

### 1. 创建状态管理模块 (`src/app/state/`)

- ✅ `app_state.rs` - 核心应用状态
- ✅ `player_state.rs` - 播放器状态管理
- ✅ `ui_state.rs` - UI 状态管理
- ✅ `persistence.rs` - 状态持久化

### 2. 创建服务层 (`src/app/services/`)

- ✅ `library_import.rs` - 库导入服务（从 `import_library_paths` 方法提取）
- ✅ `player_restore.rs` - 播放器恢复服务（从 `restore_player_state` 方法提取）
- ✅ `metadata_editor.rs` - 元数据编辑服务（从 `update_track_metadata` 方法提取）
- ✅ `lyrics_manager.rs` - 歌词管理服务（从歌词相关方法提取）

### 3. 重构 App 结构体

- ✅ 移除重复的字段定义
- ✅ 使用新的状态管理器
- ⏳ 更新所有方法以使用新结构

## 待完成的工作

### Phase 1: 完成 App 核心方法重构

1. **重构 `load_heavy_data` 方法**
   - 使用 `StatePersistence` 服务
   - 委托给 `AppState` 的方法
   - 简化逻辑

2. **重构 `save_state` 方法**
   - 使用新的状态结构
   - 使用 `StatePersistence` 服务

3. **重构 `restore_player_state` 方法**
   - 使用 `PlayerRestoreService`
   - 更新相关字段访问

4. **重构 `import_library_paths` 方法**
   - 使用 `LibraryImportService`

5. **重构 `update_track_metadata` 方法**
   - 使用 `MetadataEditor`

6. **重构歌词相关方法**
   - 使用 `LyricsManager`
   - `fetch_lyrics_for_current_track`

### Phase 2: 更新 app_impl.rs

`app_impl.rs` 中的代码需要更新以适应新的 App 结构：

1. 更新字段访问（如 `self.lyrics_fetch_state` -> `self.ui_state.lyrics_fetch_state`）
2. 更新方法调用
3. 测试 UI 功能

### Phase 3: 更新组件

更新 `components/` 目录下的所有组件以适应新的 App 结构：

1. `player_component.rs`
2. `playlist_table.rs`
3. `lyrics_component.rs`
4. `library_component.rs`
5. 其他组件...

## 迁移映射表

### App 字段迁移

| 旧字段 | 新位置 |
|--------|--------|
| `last_track_path` | `player_state.last_track_path` |
| `last_position` | `player_state.last_position` |
| `last_playback_mode` | `player_state.last_playback_mode` |
| `last_volume` | `player_state.last_volume` |
| `was_playing` | `player_state.was_playing` |
| `playlist_idx_to_remove` | `ui_state.playlist_idx_to_remove` |
| `playlist_being_renamed` | `ui_state.playlist_being_renamed` |
| `is_maximized` | `ui_state.is_maximized` |
| `lib_config_selections` | `ui_state.lib_config_selections` |
| `is_library_cfg_open` | `ui_state.is_library_cfg_open` |
| `show_library_and_playlist` | `ui_state.show_library_and_playlist` |
| `show_about_dialog` | `ui_state.show_about_dialog` |
| `show_lyrics_panel` | `ui_state.show_lyrics_panel` |
| `should_fetch_lyrics_on_init` | `ui_state.should_fetch_lyrics_on_init` |
| `lyrics_fetch_state` | `ui_state.lyrics_fetch_state` |
| `lyrics_service` | `lyrics_manager.lyrics_service` |
| `current_lyrics` | `lyrics_manager.current_lyrics` |
| `pending_lyrics_rx` | `lyrics_manager.pending_lyrics_rx` |

### 方法迁移

| 旧方法 | 新位置 |
|--------|--------|
| `import_library_paths()` | `LibraryImportService::import_library_path()` |
| `restore_player_state()` | `PlayerRestoreService::restore_player_state()` |
| `update_track_metadata()` | `MetadataEditor::update_track_metadata()` |
| `fetch_lyrics_for_current_track()` | `LyricsManager::fetch_lyrics_for_track()` |
| `load_heavy_data()` | 简化，使用 `StatePersistence` |
| `save_state()` | 简化，使用 `StatePersistence` |

## 执行步骤

### Step 1: 更新 mod.rs 中的核心方法（当前步骤）

```bash
# 需要修改的方法：
- load_heavy_data
- save_state  
- restore_player_state
- update_player_persistence
- import_library_paths
- update_track_metadata
- fetch_lyrics_for_current_track
- update_track_lyrics
```

### Step 2: 更新 app_impl.rs

```bash
# 需要更新：
- 字段访问模式
- 歌词相关逻辑
- UI 状态更新
```

### Step 3: 更新所有组件

```bash
# 逐个更新 components/ 下的文件
- 修改字段访问
- 测试编译
- 测试功能
```

### Step 4: 测试和验证

```bash
# 完整测试
cargo build
cargo test
# 手动测试所有功能
```

## 注意事项

1. **向后兼容**: 确保序列化/反序列化仍然工作
2. **渐进式迁移**: 一次修改一个模块，保持编译通过
3. **保留原有API**: 可以添加过渡方法来保持兼容
4. **测试覆盖**: 每次修改后都要测试相关功能

## 下一步行动

选择以下之一：

A. 继续自动迁移 - 我会逐步更新所有代码
B. 仅完成核心方法 - 保留组件部分手动迁移
C. 查看具体某个方法的重构方案
D. 暂停，审查当前进度

你希望如何继续？
