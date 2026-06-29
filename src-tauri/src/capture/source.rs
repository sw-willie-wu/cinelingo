use serde::{Deserialize, Serialize};

/// 即時字幕的音源來源（系統輸出 loopback／某程式 loopback／麥克風 capture）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AudioSource {
    System,
    Process { pid: u32 },
    InputDevice { id: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn system_serializes_tagged() {
        let j = serde_json::to_string(&AudioSource::System).unwrap();
        assert_eq!(j, r#"{"kind":"system"}"#);
    }
    #[test]
    fn process_round_trip() {
        let s = AudioSource::Process { pid: 4321 };
        let j = serde_json::to_string(&s).unwrap();
        assert_eq!(j, r#"{"kind":"process","pid":4321}"#);
        assert!(matches!(serde_json::from_str::<AudioSource>(&j).unwrap(), AudioSource::Process { pid: 4321 }));
    }
    #[test]
    fn input_device_round_trip() {
        let j = r#"{"kind":"inputDevice","id":"dev-1"}"#;
        assert!(matches!(serde_json::from_str::<AudioSource>(j).unwrap(), AudioSource::InputDevice { ref id } if id == "dev-1"));
    }
}
