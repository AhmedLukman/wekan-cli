pub(super) fn push_optional(lines: &mut Vec<String>, label: &str, value: Option<&str>) {
    if let Some(value) = value {
        lines.push(format!("{label}: {}", escape_terminal_controls(value)));
    }
}

pub(super) fn push_optional_bool(lines: &mut Vec<String>, label: &str, value: Option<bool>) {
    if let Some(value) = value {
        lines.push(format!("{label}: {}", if value { "yes" } else { "no" }));
    }
}

pub(super) fn render_named_value(
    lines: &mut Vec<String>,
    indent: usize,
    name: &str,
    value: &serde_json::Value,
) {
    let padding = " ".repeat(indent);
    match value {
        serde_json::Value::Array(values) => {
            lines.push(format!("{padding}{name}: [{}]", values.len()));
            for (index, value) in values.iter().enumerate() {
                render_named_value(lines, indent + 2, &format!("[{index}]"), value);
            }
        }
        serde_json::Value::Object(values) => {
            lines.push(format!("{padding}{name}:"));
            for (name, value) in values {
                if !value.is_null() {
                    render_named_value(lines, indent + 2, name, value);
                }
            }
        }
        serde_json::Value::String(value) => lines.push(format!(
            "{padding}{name}: {}",
            escape_terminal_controls(value)
        )),
        other => lines.push(format!("{padding}{name}: {other}")),
    }
}

pub(super) fn render_named_value_complete(
    lines: &mut Vec<String>,
    indent: usize,
    name: &str,
    value: &serde_json::Value,
) {
    let padding = " ".repeat(indent);
    match value {
        serde_json::Value::Array(values) => {
            lines.push(format!("{padding}{name}: [{}]", values.len()));
            for (index, value) in values.iter().enumerate() {
                render_named_value_complete(lines, indent + 2, &format!("[{index}]"), value);
            }
        }
        serde_json::Value::Object(values) => {
            lines.push(format!("{padding}{name}:"));
            for (name, value) in values {
                render_named_value_complete(lines, indent + 2, name, value);
            }
        }
        serde_json::Value::String(value) => lines.push(format!(
            "{padding}{name}: {}",
            escape_terminal_controls(value)
        )),
        other => lines.push(format!("{padding}{name}: {other}")),
    }
}

pub(super) fn two_column_table(
    left_header: &str,
    right_header: &str,
    rows: &[(String, String)],
) -> String {
    let left_width = rows
        .iter()
        .map(|(left, _)| left.len())
        .max()
        .unwrap_or(left_header.len())
        .max(left_header.len());
    let mut lines = vec![format!("{left_header:<left_width$}  {right_header}")];
    lines.extend(
        rows.iter()
            .map(|(left, right)| format!("{left:<left_width$}  {right}")),
    );
    lines.join("\n")
}

pub(super) fn escape_terminal_controls(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if character.is_control() {
            escaped.extend(character.escape_default());
        } else {
            escaped.push(character);
        }
    }
    escaped
}
