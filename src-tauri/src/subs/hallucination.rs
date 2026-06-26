/// 是否為已知「字幕組署名 / 訂閱呼籲」boilerplate 幻覺（高精準、近零誤刪取向）。
/// 純函式、無狀態。比對前 trim；amara.org 走小寫比對（不可用裸 Amara → 誤中 Tamara/Samara）。
pub fn is_boilerplate(text: &str) -> bool {
    let t = text.trim();
    // 字幕署名類
    if t.contains("字幕志愿者") || t.contains("字幕志願者") || t.contains("字幕提供") || t.contains("字幕由") {
        return true;
    }
    if t.contains("字幕") && (t.contains("组") || t.contains("組")) {
        return true;
    }
    // 李宗盛：僅與「字幕」同現才算（避免誤殺真的提到李宗盛的內容）
    if t.contains("字幕") && t.contains("李宗盛") {
        return true;
    }
    // amara.org（小寫比對）
    if t.to_lowercase().contains("amara.org") {
        return true;
    }
    // 訂閱呼籲類
    if t.contains("请订阅") || t.contains("請訂閱") {
        return true;
    }
    if (t.contains("订阅") || t.contains("訂閱"))
        && (t.contains("点赞") || t.contains("點讚") || t.contains("转发") || t.contains("轉發"))
    {
        return true;
    }
    // 知名 whisper-zh boilerplate「明镜与点点栏目」。維持 明镜+点点：明镜+订阅/支持 太寬，會誤殺
    // 「他支持明镜周刊」「我订阅了明镜周刊」等真句（明镜周刊=Der Spiegel 為真實媒體）。
    // 實測的「明镜…欢迎订阅明镜」由下方 欢迎订阅 規則接，不靠此規則。
    if t.contains("明镜") && t.contains("点点") {
        return true;
    }
    // 影片結尾 sign-off 幻覺（whisper-zh 高頻；這些完整片語在日常對話極少出現，contains 取向夠安全）
    if t.contains("感谢观看") || t.contains("感謝觀看")
        || t.contains("谢谢观看") || t.contains("謝謝觀看")
        || t.contains("感谢收看") || t.contains("感謝收看")
    {
        return true;
    }
    // 訂閱呼籲：欢迎订阅 promo
    if t.contains("欢迎订阅") || t.contains("歡迎訂閱") {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_known_boilerplate() {
        assert!(is_boilerplate("字幕志愿者 李宗盛")); // 實測幻覺原文
        assert!(is_boilerplate("字幕志願者")); // 繁體變體
        assert!(is_boilerplate("字幕提供"));
        assert!(is_boilerplate("字幕由○○提供"));
        assert!(is_boilerplate("字幕组 提供")); // 字幕 + 组
        assert!(is_boilerplate("请订阅 转发"));
        assert!(is_boilerplate("請訂閱"));
        assert!(is_boilerplate("Amara.org")); // 小寫比對命中
        assert!(is_boilerplate("明镜与点点栏目"));
        assert!(is_boilerplate("  字幕提供  ")); // 前後空白 trim
        // sign-off / promo 幻覺（whisper-zh 高頻；e2e log 實測）
        assert!(is_boilerplate("感谢观看"));
        assert!(is_boilerplate("好 感谢观看")); // 混合行含 sign-off → 整行丟（既定取向）
        assert!(is_boilerplate("感謝觀看")); // 繁體
        assert!(is_boilerplate("谢谢观看，下次再见"));
        assert!(is_boilerplate("感谢收看"));
        assert!(is_boilerplate("欢迎订阅"));
        assert!(is_boilerplate("歡迎訂閱本頻道")); // 繁體
        assert!(is_boilerplate("明镜需要您的支持 欢迎订阅明镜")); // e2e 實測幻覺原文（由 欢迎订阅 規則接）
    }

    #[test]
    fn does_not_match_real_content() {
        assert!(!is_boilerplate("走一步向回望,")); // 《醉》真歌詞
        assert!(!is_boilerplate("那聲音早已經轉身繞過花牆。"));
        assert!(!is_boilerplate("這首歌是李宗盛寫的")); // 李宗盛 但無字幕
        assert!(!is_boilerplate("Tamara, wait for me")); // 守 D2：裸 Amara 不可中人名
        assert!(!is_boilerplate("我們今天來聊聊音樂"));
        assert!(!is_boilerplate(""));
        // 守住誤刪邊界：含「观看」但非「感谢观看」、「明镜」無 promo 詞、常見真道謝/告別
        assert!(!is_boilerplate("我们今天来观看比赛")); // 「观看」非「感谢观看」
        assert!(!is_boilerplate("明镜高悬")); // 明镜 但無 promo 詞
        assert!(!is_boilerplate("谢谢大家")); // 真道謝，非完整 sign-off 片語
        assert!(!is_boilerplate("好 拜拜"));
        // 明镜 是真實媒體（明镜周刊/Der Spiegel）：明镜+支持/订阅 屬真句，不可誤刪（守住 明镜 規則邊界）
        assert!(!is_boilerplate("他支持明镜周刊的报道"));
        assert!(!is_boilerplate("我订阅了明镜周刊"));
    }
}
