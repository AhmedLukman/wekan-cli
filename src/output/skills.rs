use crate::command_result::SkillListSuccess;

pub(super) fn list(data: &SkillListSuccess) -> String {
    let width = data
        .topics
        .iter()
        .map(|topic| topic.name.len())
        .max()
        .unwrap_or(5)
        .max(5);
    let mut lines = vec![format!("{:<width$}  DESCRIPTION", "TOPIC")];
    lines.extend(
        data.topics
            .iter()
            .map(|topic| format!("{:<width$}  {}", topic.name, topic.description)),
    );
    lines.join("\n")
}
