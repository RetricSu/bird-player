use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimedTextCue {
    pub text: String,
    pub start_time_ms: Option<u64>,
    pub end_time_ms: Option<u64>,
}

impl TimedTextCue {
    pub fn is_active_at(&self, timestamp_ms: u64) -> bool {
        match (self.start_time_ms, self.end_time_ms) {
            (Some(start), Some(end)) => timestamp_ms >= start && timestamp_ms < end,
            (Some(start), None) => timestamp_ms >= start,
            _ => false,
        }
    }
}

pub fn parse_lrc(content: &str) -> Vec<TimedTextCue> {
    let mut cues = Vec::new();

    for line in content.lines() {
        let Some((timestamp_text, text)) = line.split_once(']') else {
            continue;
        };
        let Some(timestamp_text) = timestamp_text.strip_prefix('[') else {
            continue;
        };
        let Some(timestamp_ms) = parse_lrc_timestamp(timestamp_text) else {
            continue;
        };

        cues.push(TimedTextCue {
            text: text.trim().to_string(),
            start_time_ms: Some(timestamp_ms),
            end_time_ms: None,
        });
    }

    cues.sort_by_key(|cue| cue.start_time_ms);
    for index in 0..cues.len().saturating_sub(1) {
        cues[index].end_time_ms = cues[index + 1].start_time_ms;
    }

    cues
}

fn parse_lrc_timestamp(timestamp: &str) -> Option<u64> {
    let (minutes, seconds) = timestamp.split_once(':')?;
    let minutes = minutes.parse::<u64>().ok()?;
    let seconds = seconds.parse::<f64>().ok()?;
    if !seconds.is_finite() || seconds.is_sign_negative() {
        return None;
    }

    Some(minutes * 60_000 + (seconds * 1_000.0) as u64)
}

#[cfg(test)]
mod tests {
    use super::{parse_lrc, TimedTextCue};

    #[test]
    fn parse_lrc_builds_ordered_cues_and_end_times() {
        let cues = parse_lrc("[00:02.50]Second\n[00:01.00]First\n[00:04]Last");

        assert_eq!(cues.len(), 3);
        assert_eq!(cues[0].text, "First");
        assert_eq!(cues[0].start_time_ms, Some(1_000));
        assert_eq!(cues[0].end_time_ms, Some(2_500));
        assert_eq!(cues[2].end_time_ms, None);
    }

    #[test]
    fn cue_activity_uses_end_time_as_exclusive_boundary() {
        let cue = TimedTextCue {
            text: "Caption".to_string(),
            start_time_ms: Some(1_000),
            end_time_ms: Some(2_000),
        };

        assert!(!cue.is_active_at(999));
        assert!(cue.is_active_at(1_000));
        assert!(!cue.is_active_at(2_000));
    }
}
