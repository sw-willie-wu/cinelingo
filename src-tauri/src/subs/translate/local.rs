use super::{build_translate_messages, is_untranslated, postprocess_translation, source_lang_name, target_lang_name, Translator};
use super::llama::LlamaServer;
use async_trait::async_trait;
use std::path::Path;

pub struct LocalLlmTranslator {
    server: LlamaServer,
    http: reqwest::Client,
}

pub fn parse_chat_content(json: &serde_json::Value) -> Result<String, String> {
    json.get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("bad chat response: {json}"))
}

#[allow(dead_code)]
impl LocalLlmTranslator {
    pub async fn new(exe: &Path, gguf: &Path) -> Result<Self, String> {
        let server = LlamaServer::start(exe, gguf).await?;
        Ok(Self {
            server,
            http: reqwest::Client::new(),
        })
    }

    /// &self → 可從 Arc 呼叫（Manager::shutdown 經 Arc deref）。
    pub async fn shutdown(&self) {
        self.server.kill().await;
    }
}

#[async_trait]
impl Translator for LocalLlmTranslator {
    async fn translate(
        &self,
        text: &str,
        source_lang: Option<&str>,
        target_lang: &str,
    ) -> Result<String, String> {
        if text.trim().is_empty() {
            return Ok(String::new());
        }
        let name = target_lang_name(target_lang);
        let src_name = source_lang.map(source_lang_name).unwrap_or_default(); // 串流恆 Some(ISO)
        let body = serde_json::json!({
            "model": "local",
            "temperature": 0.1,
            "max_tokens": 256,
            "stream": false,
            "messages": build_translate_messages(text, &src_name, &name),
        });
        let resp = self
            .http
            .post(format!("{}/v1/chat/completions", self.server.base_url()))
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let out = postprocess_translation(&parse_chat_content(&json)?);
        if is_untranslated(text, &out, target_lang) {
            return Ok(String::new()); // 仍 echo → 回空 → stream.rs Ok(_) 不設 target_text → 只顯原文
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_openai_content() {
        let j = serde_json::json!({"choices":[{"message":{"content":"你好"}}]});
        assert_eq!(parse_chat_content(&j).unwrap(), "你好");
    }

    #[test]
    fn errors_on_empty_choices() {
        let j = serde_json::json!({"choices":[]});
        assert!(parse_chat_content(&j).is_err());
    }

    #[tokio::test]
    #[ignore] // 手動：需 llm/ 內已下載 llama-server.exe + gemma gguf（環境變數給路徑）
    async fn integration_translate_en_to_trad() {
        let exe = std::path::PathBuf::from(std::env::var("LLAMA_EXE").unwrap());
        let gguf = std::path::PathBuf::from(std::env::var("LLAMA_GGUF").unwrap());
        let tr = LocalLlmTranslator::new(&exe, &gguf).await.unwrap();
        let out = tr
            .translate("Wait, that doesn't make sense.", Some("en"), "zh-Hant")
            .await
            .unwrap();
        assert!(!out.is_empty() && !out.contains("<src>"));
        tr.shutdown().await;
    }
}
