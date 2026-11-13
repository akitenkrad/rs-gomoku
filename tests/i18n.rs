use rs_gomoku::i18n::{Lang, Msg};

#[test]
fn language_defaults_toggles_and_parses_case_insensitively() {
    assert_eq!(Lang::default(), Lang::En);
    assert_eq!(Lang::En.toggle(), Lang::Ja);
    assert_eq!(Lang::Ja.toggle(), Lang::En);
    assert_eq!(Lang::from_str("en"), Some(Lang::En));
    assert_eq!(Lang::from_str("EN"), Some(Lang::En));
    assert_eq!(Lang::from_str("Ja"), Some(Lang::Ja));
    assert_eq!(Lang::from_str("fr"), None);
    assert_eq!(Lang::from_str(""), None);
}

#[test]
fn every_message_is_non_empty_and_translated() {
    let mut seen = std::collections::HashSet::new();
    for message in Msg::ALL {
        assert!(seen.insert(message as usize), "duplicate message in Msg::ALL: {message:?}");
        let en = message.text(Lang::En);
        let ja = message.text(Lang::Ja);
        assert!(!en.is_empty(), "missing English text for {message:?}");
        assert!(!ja.is_empty(), "missing Japanese text for {message:?}");
        assert_ne!(en, ja, "untranslated message: {message:?}");
    }
    assert_eq!(seen.len(), 60, "Msg::ALL must contain every variant exactly once");
}

#[test]
fn language_supplies_localized_names_and_formatted_messages() {
    assert_eq!(Lang::En.default_human_name(), "You");
    assert_eq!(Lang::Ja.default_human_name(), "あなた");
    assert_eq!(Lang::En.move_line(15, "Alice", "Black", "i10"), "Move 15: Alice Black i10");
    assert_eq!(Lang::Ja.move_line(15, "Alice", "黒", "i10"), "手数 15: Alice 黒 i10");
    assert_eq!(Lang::En.result_winner("Alice"), "Result: Alice wins");
    assert_eq!(Lang::Ja.result_winner("Alice"), "結果: Alice の勝ち");
}
