//! Completion over the bundled grammar pack (see `grammars_data`).
//!
//! Returns FULL command strings that literally extend the buffer, so the
//! caller can strip the buffer prefix uniformly across suggestion sources.
//! Grammar never guesses arguments -- only subcommands and (Task 2) flags.

use super::grammars_data::GRAMMARS;

/// One tool's curated grammar: subcommands (common-first == rank order),
/// per-subcommand flags, and flags valid anywhere.
pub struct ToolGrammar {
    pub name: &'static str,
    pub subcommands: &'static [Subcommand],
    pub global_flags: &'static [&'static str],
}

/// A subcommand and the flags commonly used with it.
pub struct Subcommand {
    pub name: &'static str,
    pub flags: &'static [&'static str],
}

/// Complete `buffer` against the grammar pack. Returns at most `limit` full
/// command strings, each starting with `buffer`, in table (rank) order.
pub fn complete(buffer: &str, limit: usize) -> Vec<String> {
    let trailing_space = buffer.ends_with(char::is_whitespace);
    let tokens: Vec<&str> = buffer.split_whitespace().collect();

    let Some(&tool_name) = tokens.first() else {
        return Vec::new();
    };
    let Some(tool) = GRAMMARS.iter().find(|g| g.name == tool_name) else {
        return Vec::new();
    };
    // A space after the tool is required: completing the tool name itself is
    // history's job.
    if tokens.len() == 1 && !trailing_space {
        return Vec::new();
    }

    let partial = if trailing_space {
        ""
    } else {
        *tokens.last().unwrap()
    };
    let complete_tokens = if trailing_space {
        &tokens[..]
    } else {
        &tokens[..tokens.len() - 1]
    };

    // tool + partial subcommand (nothing else typed yet)
    if complete_tokens == [tool_name] && !partial.starts_with('-') {
        return tool
            .subcommands
            .iter()
            .filter(|sub| sub.name.starts_with(partial) && sub.name != partial)
            .take(limit)
            .map(|sub| format!("{}{}", buffer, &sub.name[partial.len()..]))
            .collect();
    }

    Vec::new()
}
