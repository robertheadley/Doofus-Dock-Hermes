//! Streaming safety for provider reasoning tags embedded in assistant text.
//!
//! The pinned Desktop re-runs this scrubber over accumulated text on every
//! stream flush. In particular, an unfinished reasoning block at a line
//! boundary and a partial opening tag must not flash into the answer bubble.

const REASONING_TAGS: &[&str] = &[
    "think",
    "thinking",
    "reasoning",
    "thought",
    "reasoning_scratchpad",
    "scratchpad",
    "analysis",
];

/// Current safe rendering result for an accumulated reasoning stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReasoningUpdate {
    /// Assistant text safe to render as the answer.
    pub answer: String,
    /// Text captured inside recognized reasoning blocks.
    pub reasoning: String,
}

/// Reprocesses accumulated deltas so partial tags and open blocks never flash
/// into the answer surface while they are arriving.
#[derive(Debug, Default, Clone)]
pub struct ReasoningStreamScrubber {
    accumulated: String,
}

impl ReasoningStreamScrubber {
    /// Append one provider delta and return the current safe answer/reasoning.
    pub fn push_delta(&mut self, delta: &str) -> ReasoningUpdate {
        self.accumulated.push_str(delta);
        scrub_with_reasoning(&self.accumulated)
    }

    /// Recompute a snapshot without changing the accumulated stream.
    pub fn snapshot(&self) -> ReasoningUpdate {
        scrub_with_reasoning(&self.accumulated)
    }
}

/// Remove reasoning blocks and suppress an unterminated reasoning preamble.
///
/// Paired tags are removed anywhere in the text. Unterminated tags are removed
/// only when they begin at the start of the text or on a new line (after
/// optional spaces), matching the pinned Desktop's treatment of prose that
/// merely mentions a tag inline. A partial opening tag is held back only when
/// its name is a prefix of a supported reasoning tag, so ordinary HTML-like
/// text such as `<div` remains visible for the Markdown layer to escape.
pub fn strip_reasoning_blocks(text: &str) -> String {
    scrub_with_reasoning(text).answer
}

fn scrub_with_reasoning(text: &str) -> ReasoningUpdate {
    let (closed, mut reasoning) = strip_closed_blocks(text);
    let answer = strip_open_block_or_partial_tag(&closed, &mut reasoning);
    ReasoningUpdate { answer, reasoning }
}

fn strip_closed_blocks(text: &str) -> (String, String) {
    let bytes = text.as_bytes();
    let mut output = String::with_capacity(text.len());
    let mut copied_until = 0;
    let mut search_from = 0;
    let mut reasoning = String::new();

    while let Some((start, open_end, close_start, close_end)) =
        find_next_closed_block(bytes, search_from)
    {
        let end = skip_whitespace(text, close_end);
        output.push_str(&text[copied_until..start]);
        append_reasoning(&mut reasoning, &text[open_end..close_start]);

        let previous = output.chars().next_back();
        let next = text[end..].chars().next();
        if previous.is_some_and(|ch| !ch.is_whitespace())
            && next.is_some_and(|ch| !ch.is_whitespace())
        {
            // Removing a block between two words must not fuse them.
            output.push(' ');
        }

        copied_until = end;
        search_from = end;
    }

    output.push_str(&text[copied_until..]);
    (output, reasoning)
}

fn find_next_closed_block(bytes: &[u8], from: usize) -> Option<(usize, usize, usize, usize)> {
    let mut cursor = from;
    while cursor < bytes.len() {
        let relative = bytes[cursor..].iter().position(|byte| *byte == b'<')?;
        let start = cursor + relative;
        let Some((tag, open_end)) = opening_tag_at(bytes, start) else {
            cursor = start + 1;
            continue;
        };
        if let Some(close_start) = find_closing_tag(bytes, open_end, tag) {
            return Some((start, open_end, close_start, close_start + tag.len() + 3));
        }
        cursor = open_end;
    }
    None
}

fn opening_tag_at(bytes: &[u8], start: usize) -> Option<(&'static str, usize)> {
    if bytes.get(start) != Some(&b'<') {
        return None;
    }
    for tag in REASONING_TAGS {
        let tag_bytes = tag.as_bytes();
        let name_start = start + 1;
        let name_end = name_start + tag_bytes.len();
        if bytes
            .get(name_start..name_end)
            .is_some_and(|candidate| candidate.eq_ignore_ascii_case(tag_bytes))
            && bytes.get(name_end) == Some(&b'>')
        {
            return Some((tag, name_end + 1));
        }
    }
    None
}

fn find_closing_tag(bytes: &[u8], from: usize, tag: &str) -> Option<usize> {
    let mut needle = Vec::with_capacity(tag.len() + 3);
    needle.extend_from_slice(b"</");
    needle.extend_from_slice(tag.as_bytes());
    needle.push(b'>');
    bytes[from..]
        .windows(needle.len())
        .position(|candidate| candidate.eq_ignore_ascii_case(&needle))
        .map(|relative| from + relative)
}

fn skip_whitespace(text: &str, mut index: usize) -> usize {
    while let Some(ch) = text[index..].chars().next() {
        if !ch.is_whitespace() {
            break;
        }
        index += ch.len_utf8();
    }
    index
}

fn strip_open_block_or_partial_tag(text: &str, reasoning: &mut String) -> String {
    let bytes = text.as_bytes();
    for boundary in line_boundaries(bytes) {
        let mut cursor = boundary;
        while matches!(bytes.get(cursor), Some(b' ' | b'\t')) {
            cursor += 1;
        }

        if let Some((_tag, open_end)) = opening_tag_at(bytes, cursor) {
            // Paired blocks have already been removed. A remaining complete
            // opener at a block boundary is therefore still in flight.
            append_reasoning(reasoning, &text[open_end..]);
            return text[..boundary].to_owned();
        }

        if is_partial_reasoning_opener(bytes, cursor) {
            return text[..boundary].to_owned();
        }
    }
    text.to_owned()
}

fn append_reasoning(reasoning: &mut String, content: &str) {
    if content.is_empty() {
        return;
    }
    if !reasoning.is_empty() {
        reasoning.push('\n');
    }
    reasoning.push_str(content);
}

fn line_boundaries(bytes: &[u8]) -> impl Iterator<Item = usize> + '_ {
    std::iter::once(0).chain(
        bytes
            .iter()
            .enumerate()
            .filter_map(|(index, byte)| (*byte == b'\n').then_some(index + 1)),
    )
}

fn is_partial_reasoning_opener(bytes: &[u8], start: usize) -> bool {
    if bytes.get(start) != Some(&b'<') {
        return false;
    }
    let suffix = &bytes[start + 1..];
    // The pinned renderer only holds a suffix that is an exact prefix of a
    // known tag. A bare `<` is a valid prefix; `<div` and malformed tag names
    // are not.
    REASONING_TAGS.iter().any(|tag| {
        suffix.len() <= tag.len() && suffix.eq_ignore_ascii_case(&tag.as_bytes()[..suffix.len()])
    }) || suffix.is_empty()
}

#[cfg(test)]
mod tests {
    use super::{ReasoningStreamScrubber, strip_reasoning_blocks};

    #[test]
    fn strips_each_supported_reasoning_tag_case_insensitively() {
        for tag in [
            "think",
            "thinking",
            "reasoning",
            "thought",
            "reasoning_scratchpad",
            "scratchpad",
            "analysis",
        ] {
            let input = format!("before<{tag}>private</{tag}> after");
            assert_eq!(strip_reasoning_blocks(&input), "before after", "{tag}");
            let upper = tag.to_ascii_uppercase();
            let input = format!("before<{upper}>private</{upper}> after");
            assert_eq!(strip_reasoning_blocks(&input), "before after", "{tag}");
        }
    }

    #[test]
    fn adjacent_blocks_do_not_fuse_visible_words() {
        assert_eq!(
            strip_reasoning_blocks("no<thinking>a</thinking><think>b</think>Hermes"),
            "no Hermes"
        );
        assert_eq!(
            strip_reasoning_blocks("no<thinking>a</thinking>  <thought>b</thought> Hermes"),
            "no Hermes"
        );
        assert_eq!(
            strip_reasoning_blocks("backend e o <thinking>x</thinking>useSmoothReveal"),
            "backend e o useSmoothReveal"
        );
    }

    #[test]
    fn unfinished_reasoning_is_hidden_only_from_a_block_boundary() {
        assert_eq!(strip_reasoning_blocks("<thinking>private preamble"), "");
        assert_eq!(
            strip_reasoning_blocks("Answer.\n<reasoning_scratchpad>private"),
            "Answer.\n"
        );
        assert_eq!(
            strip_reasoning_blocks("inline <thinking> is a literal mention"),
            "inline <thinking> is a literal mention"
        );
    }

    #[test]
    fn partial_reasoning_tag_prefixes_are_held_but_other_tags_are_not() {
        for prefix in ["", "t", "thi", "think", "reason", "reasoning_s", "anal"] {
            let input = format!("Answer.\n<{prefix}");
            assert_eq!(strip_reasoning_blocks(&input), "Answer.\n", "<{prefix}");
        }
        assert_eq!(strip_reasoning_blocks("Answer.\n<div"), "Answer.\n<div");
        assert_eq!(
            strip_reasoning_blocks("Answer.\n<thinking attr"),
            "Answer.\n<thinking attr"
        );
        assert_eq!(
            strip_reasoning_blocks("Answer.\n<reasoning_extra"),
            "Answer.\n<reasoning_extra"
        );
    }

    #[test]
    fn split_stream_frames_never_paint_a_reasoning_block_or_partial_tag() {
        let raw =
            "<thinking>let me think about the render pipeline in detail</thinking>Final response.";
        let mut previous = String::new();
        for end in (3..=raw.len() + 2).step_by(3) {
            let frame = &raw[..end.min(raw.len())];
            let visible = strip_reasoning_blocks(frame);
            assert!(
                !visible.contains("let me think"),
                "frame {end}: {visible:?}"
            );
            assert!(
                !visible.to_ascii_lowercase().contains("<think"),
                "frame {end}: {visible:?}"
            );
            assert!(visible.starts_with(&previous), "frame {end}: {visible:?}");
            previous = visible;
        }
        assert_eq!(previous, "Final response.");
    }

    #[test]
    fn preserves_pinned_prose_seams_and_unrelated_malformed_text() {
        assert_eq!(
            strip_reasoning_blocks("<thought>x</thought>\n\nResposta final."),
            "Resposta final."
        );
        assert_eq!(
            strip_reasoning_blocks("O texto explica <thinking> sem fechá-lo"),
            "O texto explica <thinking> sem fechá-lo"
        );
        assert_eq!(
            strip_reasoning_blocks("Answer.\n<thinking nonsense"),
            "Answer.\n<thinking nonsense"
        );
    }

    #[test]
    fn incremental_api_holds_partial_tags_and_returns_reasoning_separately() {
        let mut stream = ReasoningStreamScrubber::default();
        let first = stream.push_delta("Answer.\n<thin");
        assert_eq!(first.answer, "Answer.\n");
        assert_eq!(first.reasoning, "");

        let second = stream.push_delta("king>private thought");
        assert_eq!(second.answer, "Answer.\n");
        assert_eq!(second.reasoning, "private thought");

        let final_update = stream.push_delta("</thinking> Final.");
        assert_eq!(final_update.answer, "Answer.\nFinal.");
        assert_eq!(final_update.reasoning, "private thought");
        assert_eq!(stream.snapshot(), final_update);
    }

    #[test]
    fn three_character_frames_preserve_safe_answer_and_extract_reasoning() {
        let raw = "Visible\n<analysis>internal</analysis> answer.";
        let mut stream = ReasoningStreamScrubber::default();
        let mut final_update = stream.snapshot();
        for frame in raw.as_bytes().chunks(3) {
            final_update = stream.push_delta(std::str::from_utf8(frame).unwrap());
            assert!(!final_update.answer.contains("internal"));
            assert!(!final_update.answer.to_ascii_lowercase().contains("<anal"));
        }
        assert_eq!(final_update.answer, "Visible\nanswer.");
        assert_eq!(final_update.reasoning, "internal");
    }
}
