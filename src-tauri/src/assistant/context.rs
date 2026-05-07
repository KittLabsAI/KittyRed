use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThinkStreamEvent {
    Visible(String),
    ThinkingStatus(String),
    ThinkingDelta(String),
}

#[derive(Default)]
pub struct ThinkBlockStreamFilter {
    inside_think: bool,
    pending: String,
    seen_think: bool,
}

impl ThinkBlockStreamFilter {
    pub fn consume(&mut self, chunk: &str) -> Vec<ThinkStreamEvent> {
        let text = format!("{}{}", self.pending, chunk);
        self.pending.clear();
        let mut events = Vec::new();
        let mut visible = String::new();
        let mut thinking = String::new();
        let mut index = 0;

        while index < text.len() {
            let remainder = &text[index..];
            if remainder.starts_with("<think>") {
                if !visible.is_empty() {
                    events.push(ThinkStreamEvent::Visible(std::mem::take(&mut visible)));
                }
                self.inside_think = true;
                self.seen_think = true;
                events.push(ThinkStreamEvent::ThinkingStatus("running".into()));
                index += "<think>".len();
                continue;
            }
            if remainder.starts_with("</think>") {
                if !thinking.is_empty() {
                    events.push(ThinkStreamEvent::ThinkingDelta(std::mem::take(
                        &mut thinking,
                    )));
                }
                self.inside_think = false;
                events.push(ThinkStreamEvent::ThinkingStatus("finished".into()));
                index += "</think>".len();
                continue;
            }
            if remainder.starts_with('<')
                && ("<think>".starts_with(remainder) || "</think>".starts_with(remainder))
            {
                self.pending = remainder.to_string();
                break;
            }
            let Some(character) = remainder.chars().next() else {
                break;
            };
            if self.inside_think {
                thinking.push(character);
            } else {
                visible.push(character);
            }
            index += character.len_utf8();
        }

        if !visible.is_empty() {
            events.push(ThinkStreamEvent::Visible(visible));
        }
        if !thinking.is_empty() {
            events.push(ThinkStreamEvent::ThinkingDelta(thinking));
        }
        events
    }

    pub fn needs_finish_event(&self) -> bool {
        self.seen_think && self.inside_think
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AssistantStoredMessage {
    pub role: String,
    pub content: String,
}

impl AssistantStoredMessage {
    pub fn new(role: &str, content: &str) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AssistantContextBreakdown {
    pub system: usize,
    pub user: usize,
    pub assistant: usize,
    pub tool: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AssistantContextSnapshot {
    pub used_tokens: usize,
    pub max_tokens: usize,
    pub remaining_tokens: usize,
    pub thinking_tokens: usize,
    pub breakdown: AssistantContextBreakdown,
}

pub fn estimate_context(
    system_prompt: &str,
    messages: &[AssistantStoredMessage],
    max_tokens: usize,
) -> AssistantContextSnapshot {
    let mut breakdown = AssistantContextBreakdown {
        system: estimate_tokens(system_prompt),
        user: 0,
        assistant: 0,
        tool: 0,
    };
    let mut thinking_tokens = 0;

    for message in messages {
        let tokens = estimate_tokens(&message.content);
        match message.role.as_str() {
            "user" => breakdown.user += tokens,
            "assistant" => {
                breakdown.assistant += tokens;
                thinking_tokens += estimate_thinking_tokens(&message.content);
            }
            "tool" => breakdown.tool += tokens,
            _ => {}
        }
    }

    let used_tokens = breakdown.system + breakdown.user + breakdown.assistant + breakdown.tool;
    AssistantContextSnapshot {
        used_tokens,
        max_tokens,
        remaining_tokens: max_tokens.saturating_sub(used_tokens),
        thinking_tokens,
        breakdown,
    }
}

fn estimate_tokens(text: &str) -> usize {
    let chars = text.chars().count();
    chars.saturating_add(3) / 4
}

fn estimate_thinking_tokens(text: &str) -> usize {
    let mut total = 0;
    let mut rest = text;
    while let Some(start) = rest.find("<think>") {
        let after_start = &rest[start + "<think>".len()..];
        let Some(end) = after_start.find("</think>") else {
            total += estimate_tokens(after_start);
            break;
        };
        total += estimate_tokens(&after_start[..end]);
        rest = &after_start[end + "</think>".len()..];
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn think_filter_splits_partial_tags() {
        let mut filter = ThinkBlockStreamFilter::default();
        let mut events = Vec::new();
        for chunk in ["hello <thi", "nk>hidden</th", "ink> visible"] {
            events.extend(filter.consume(chunk));
        }
        assert_eq!(
            events,
            vec![
                ThinkStreamEvent::Visible("hello ".into()),
                ThinkStreamEvent::ThinkingStatus("running".into()),
                ThinkStreamEvent::ThinkingDelta("hidden".into()),
                ThinkStreamEvent::ThinkingStatus("finished".into()),
                ThinkStreamEvent::Visible(" visible".into()),
            ]
        );
    }

    #[test]
    fn context_snapshot_counts_roles_and_thinking() {
        let messages = vec![
            AssistantStoredMessage::new("user", "hello"),
            AssistantStoredMessage::new("assistant", "<think>hidden</think>visible"),
            AssistantStoredMessage::new("tool", "tool output"),
        ];
        let snapshot = estimate_context("system prompt", &messages, 10_000);

        assert!(snapshot.used_tokens > 0);
        assert!(snapshot.breakdown.system > 0);
        assert!(snapshot.breakdown.user > 0);
        assert!(snapshot.breakdown.assistant > 0);
        assert!(snapshot.breakdown.tool > 0);
        assert!(snapshot.thinking_tokens > 0);
        assert_eq!(snapshot.max_tokens, 10_000);
    }
}
