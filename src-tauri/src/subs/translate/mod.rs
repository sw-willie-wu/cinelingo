pub mod llama;
pub mod local;
use async_trait::async_trait;

#[allow(dead_code)]
#[async_trait]
pub trait Translator: Send + Sync {
    async fn translate(&self, text: &str, source_lang: Option<&str>, target_lang: &str) -> Result<String, String>;
}

/// Target-language code → English name for injection into prompts.
/// Covers common values from the LANGS list; unknown codes pass through unchanged.
#[allow(dead_code)]
pub fn target_lang_name(code: &str) -> String {
    match code {
        "zh-Hant" => "Traditional Chinese",
        "zh-Hans" => "Simplified Chinese",
        "ja" => "Japanese",
        "en" => "English",
        "ko" => "Korean",
        "es" => "Spanish",
        "fr" => "French",
        "de" => "German",
        "ru" => "Russian",
        "pt" => "Portuguese",
        "it" => "Italian",
        "nl" => "Dutch",
        "ar" => "Arabic",
        "hi" => "Hindi",
        "tr" => "Turkish",
        "vi" => "Vietnamese",
        "th" => "Thai",
        "id" => "Indonesian",
        "pl" => "Polish",
        "uk" => "Ukrainian",
        other => other,
    }
    .to_string()
}

/// whisper 來源 ISO 碼 → 英文名（給 prompt）。zh 特判 Chinese（target_lang_name 只有 zh-Hant/zh-Hans），
/// 其餘委派 target_lang_name（ja/en/ko… 已涵蓋），未知 pass-through。純函式。
#[allow(dead_code)]
pub fn source_lang_name(code: &str) -> String {
    match code {
        "zh" => "Chinese".to_string(),
        other => target_lang_name(other),
    }
}

/// Build TranslateGemma's raw `/completion` prompt (reproduces the model's embedded
/// chat template for text translation). Target name is passed explicitly so 繁/簡/other
/// are correct even though the model's own map collapses zh variants to "Chinese".
pub fn build_translate_prompt(
    text: &str,
    source_name: &str,
    source_code: &str,
    target_name: &str,
    target_code: &str,
) -> String {
    format!(
        "<bos><start_of_turn>user\n\
You are a professional {source_name} ({source_code}) to {target_name} ({target_code}) translator. \
Your goal is to accurately convey the meaning and nuances of the original {source_name} text while \
adhering to {target_name} grammar, vocabulary, and cultural sensitivities.\n\
Produce only the {target_name} translation, without any additional explanations or commentary. \
Please translate the following {source_name} text into {target_name}:\n\n\n\
{text}<end_of_turn>\n<start_of_turn>model\n"
    )
}

/// Source-language-agnostic prompt（用於載入的字幕檔：來源語言未知時）。
/// 不指名來源語（避免硬套 English），仍顯式命名 target（繁/簡正確）。純函式。
pub fn build_translate_prompt_auto(text: &str, target_name: &str, target_code: &str) -> String {
    format!(
        "<bos><start_of_turn>user\n\
You are a professional translator. Translate the text below into {target_name} ({target_code}), \
accurately conveying the meaning and nuances of the original while adhering to {target_name} \
grammar, vocabulary, and cultural sensitivities.\n\
Produce only the {target_name} translation, without any additional explanations or commentary:\n\n\n\
{text}<end_of_turn>\n<start_of_turn>model\n"
    )
}

/// Strip `<think>…</think>` blocks, stray `<src>` wrappers, and surrounding quotes
/// from a raw LLM translation response.
#[allow(dead_code)]
pub fn postprocess_translation(raw: &str) -> String {
    let mut s = raw.trim().to_string();
    while let (Some(a), Some(b)) = (s.find("<think>"), s.find("</think>")) {
        if b > a {
            s.replace_range(a..b + "</think>".len(), "");
        } else {
            break;
        }
    }
    let s = s.trim();
    let s = s
        .strip_prefix("<src>")
        .unwrap_or(s)
        .strip_suffix("</src>")
        .unwrap_or(s)
        .trim();
    s.trim_matches(|c| c == '"' || c == '\u{300C}' || c == '\u{300D}' || c == '\u{201C}' || c == '\u{201D}')
        .trim()
        .to_string()
}

/// 假名「字母」判定（排除 ・U+30FB、ー U+30FC 等標點，避免誤殺中文音譯名）。
fn is_kana_letter(c: char) -> bool {
    matches!(c, '\u{3041}'..='\u{3096}' | '\u{30A1}'..='\u{30FA}')
}

/// 判翻譯輸出「其實沒翻」：中文 target 時含日文假名字母，或與來源完全相同（非空）。純函式。
#[allow(dead_code)]
pub fn is_untranslated(src: &str, output: &str, target_lang: &str) -> bool {
    let o = output.trim();
    if o.is_empty() {
        return false;
    }
    if o == src.trim() {
        return true;
    }
    if target_lang.starts_with("zh") && o.chars().any(is_kana_letter) {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_maps_trad() {
        assert_eq!(target_lang_name("zh-Hant"), "Traditional Chinese");
    }

    #[test]
    fn name_unknown_passthrough() {
        assert_eq!(target_lang_name("xx"), "xx");
    }

    #[test]
    fn prompt_has_translategemma_format_and_explicit_target() {
        let p = build_translate_prompt("これは", "Japanese", "ja", "Traditional Chinese", "zh-Hant");
        assert!(p.starts_with("<bos><start_of_turn>user\n"));
        assert!(p.ends_with("<end_of_turn>\n<start_of_turn>model\n"));
        assert!(p.contains("professional Japanese (ja) to Traditional Chinese (zh-Hant) translator"));
        assert!(p.contains("これは"));
        // 繁/簡 both via explicit name
        let s = build_translate_prompt("x", "English", "en", "Simplified Chinese", "zh-Hans");
        assert!(s.contains("to Simplified Chinese (zh-Hans) translator"));
    }

    #[test]
    fn name_maps_simp() {
        assert_eq!(target_lang_name("zh-Hans"), "Simplified Chinese");
    }

    #[test]
    fn postprocess_strips_think_and_quotes() {
        assert_eq!(postprocess_translation("<think>reason</think> 你好"), "你好");
        assert_eq!(postprocess_translation("「你好」"), "你好");
        assert_eq!(postprocess_translation("<src>你好</src>"), "你好");
    }

    #[test]
    fn source_lang_name_maps() {
        assert_eq!(source_lang_name("ja"), "Japanese");
        assert_eq!(source_lang_name("zh"), "Chinese");   // 來源中文用 zh
        assert_eq!(source_lang_name("en"), "English");
        assert_eq!(source_lang_name("xx"), "xx");         // 未知 pass-through
    }

    #[test]
    fn auto_prompt_has_no_source_lang_and_names_target() {
        let p = build_translate_prompt_auto("これは", "Traditional Chinese", "zh-Hant");
        assert!(p.starts_with("<bos><start_of_turn>user\n"));
        assert!(p.ends_with("<end_of_turn>\n<start_of_turn>model\n"));
        assert!(p.contains("これは"));
        assert!(p.contains("Traditional Chinese"));
        assert!(!p.contains("(en)"));           // 不得誤指來源為英文
        assert!(!p.contains("English (en)"));
    }

    #[test]
    fn is_untranslated_cases() {
        // 中文 target：output 含假名 → 未翻
        assert!(is_untranslated("これは", "これは日本語", "zh-Hant"));
        assert!(is_untranslated("だよな", "だよな", "zh-Hant"));       // exact echo（也含假名）
        // 乾淨繁中 → 已翻
        assert!(!is_untranslated("hello", "你好世界", "zh-Hant"));
        assert!(!is_untranslated("10月", "十月最高", "zh-Hant"));       // 數字+中文、無假名
        // 音譯名用中黑點 ・(U+30FB)、長音 ー(U+30FC) → 不得誤判
        assert!(!is_untranslated("x", "阿尼亞・佛傑ー", "zh-Hant"));
        // target 是日文：假名合法 → 不判未翻
        assert!(!is_untranslated("anime", "アニメ", "ja"));
        // 非 exact、無假名（英→中乾淨）
        assert!(!is_untranslated("Wait", "等等", "zh-Hant"));
        // 空 guard
        assert!(!is_untranslated("", "", "zh-Hant"));
    }
}
