/// Matches JavaScript String.prototype.trim(), including Wekan schema cleaning.
/// JavaScript preserves U+0085 and trims U+FEFF, unlike Rust's Unicode whitespace.
pub(crate) fn trim_javascript_whitespace(value: &str) -> &str {
    value.trim_matches(|character| {
        matches!(
            character,
            '\u{0009}'..='\u{000D}'
                | ' '
                | '\u{00A0}'
                | '\u{1680}'
                | '\u{2000}'..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
        )
    })
}
