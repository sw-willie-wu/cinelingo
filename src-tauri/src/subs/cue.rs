use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Cue {
    pub id: String,
    pub session_id: String,
    pub start_sec: f64,
    pub end_sec: f64,
    pub source_text: String,
    pub lang: Option<String>,
    pub status: String,
}

/// 決定性鍵：絕對起始毫秒。與前端 deriveCueId 一致。
pub fn derive_cue_id(start_ms: i64) -> String {
    format!("{start_ms}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_deterministic() {
        assert_eq!(derive_cue_id(1500), derive_cue_id(1500));
        assert_ne!(derive_cue_id(1500), derive_cue_id(1600));
    }

    #[test]
    fn serializes_camel_case() {
        let c = Cue {
            id: "a".into(),
            session_id: "s".into(),
            start_sec: 0.0,
            end_sec: 1.0,
            source_text: "t".into(),
            lang: None,
            status: "final".into(),
        };
        let j = serde_json::to_string(&c).unwrap();
        assert!(j.contains("\"sessionId\"") && j.contains("\"startSec\""));
    }
}
