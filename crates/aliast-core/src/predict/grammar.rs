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
    // Conservative guards: quoting, escaping, and command separators put the
    // buffer outside this simple token model -- suggest nothing.
    if buffer.contains(['"', '\'', '`', '\\', ';', '|', '&']) {
        return Vec::new();
    }

    let trailing_space = buffer.ends_with(char::is_whitespace);
    // Match against the command after a leading `sudo `, but return full
    // strings extending the original buffer.
    let stripped = buffer.strip_prefix("sudo ").unwrap_or(buffer);
    let tokens: Vec<&str> = stripped.split_whitespace().collect();

    let Some(&tool_name) = tokens.first() else {
        return Vec::new();
    };
    let Some(tool) = GRAMMARS.iter().find(|g| g.name == tool_name) else {
        return Vec::new();
    };
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

    // Flag completion: current token starts with '-'. Candidates are the
    // known subcommand's flags (if one was typed) then global flags.
    if partial.starts_with('-') {
        let subcommand = complete_tokens
            .iter()
            .skip(1)
            .find(|token| !token.starts_with('-'))
            .and_then(|name| tool.subcommands.iter().find(|sub| sub.name == *name));
        if complete_tokens.len() > 1 && subcommand.is_none() {
            // An argument we don't recognize precedes the flag: stay silent
            // rather than suggest flags for the wrong context.
            return Vec::new();
        }
        let subcommand_flags = subcommand.map(|sub| sub.flags).unwrap_or(&[]);
        let mut seen = Vec::new();
        return subcommand_flags
            .iter()
            .chain(tool.global_flags.iter())
            .filter(|flag| flag.starts_with(partial) && **flag != partial)
            .filter(|flag| {
                if seen.contains(flag) {
                    false
                } else {
                    seen.push(*flag);
                    true
                }
            })
            .take(limit)
            .map(|flag| format!("{}{}", buffer, &flag[partial.len()..]))
            .collect();
    }

    // Subcommand completion: only the tool typed so far.
    if complete_tokens == [tool_name] {
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
