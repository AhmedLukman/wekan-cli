use crate::client::cards::CardCopyPosition;
use clap::Args;
use serde_json::{Number, Value};

pub(crate) fn identifier(value: &str) -> Result<String, String> {
    let value = crate::client::trim_javascript_whitespace(value);
    if value.is_empty() || matches!(value, "." | "..") {
        Err("ID must be nonblank and cannot be . or ..".into())
    } else {
        Ok(value.to_owned())
    }
}

fn finite_number(value: &str) -> Result<Number, String> {
    let number: Number = value
        .parse()
        .map_err(|_| "position must be a number".to_owned())?;
    if number.as_f64().is_some_and(f64::is_finite) {
        Ok(number)
    } else {
        Err("position must be finite".into())
    }
}

#[derive(Debug, Args)]
pub struct PositionArgs {
    /// Zero-based destination position. Wekan floors fractions and clamps negatives.
    #[arg(long, value_parser = finite_number, allow_hyphen_values = true, conflicts_with = "position_json")]
    pub position: Option<Number>,
    /// Exact JSON position for Wekan's Number coercion; null skips repositioning.
    #[arg(long, value_name = "JSON", value_parser = json_position, allow_hyphen_values = true, conflicts_with = "position")]
    pub position_json: Option<Value>,
}
impl PositionArgs {
    pub(crate) fn into_value(self) -> Option<Value> {
        self.position_json
            .or_else(|| self.position.map(Value::Number))
    }
}
fn json_position(value: &str) -> Result<Value, String> {
    serde_json::from_str(value).map_err(|_| "expected a JSON position".into())
}

#[derive(Debug, Args)]
pub struct CardPositionArgs {
    /// Zero-based destination position. Wekan floors fractions and clamps negatives.
    #[arg(long, value_parser = finite_number, allow_hyphen_values = true, conflicts_with = "position_json")]
    pub position: Option<Number>,
    /// Exact JSON number, string, or null; null positions a card at the top.
    #[arg(long, value_name = "JSON", value_parser = card_position, allow_hyphen_values = true, conflicts_with = "position")]
    pub position_json: Option<CardCopyPosition>,
}
impl CardPositionArgs {
    pub(crate) fn into_value(self) -> Option<CardCopyPosition> {
        self.position_json
            .or_else(|| self.position.map(CardCopyPosition::Number))
    }
}
fn card_position(value: &str) -> Result<CardCopyPosition, String> {
    serde_json::from_str(value).map_err(|_| "expected a JSON number, string, or null".into())
}
