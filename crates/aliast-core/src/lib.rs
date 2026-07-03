pub mod ai;
pub mod history;
pub mod predict;

use history::{HistoryStore, SuggestionContext};

/// Returns a suffix-only completion suggestion for the given command buffer,
/// merging three sources: personalized history (frecency-ranked), the
/// bundled grammar pack (subcommands/flags), and directory completion.
///
/// Returns None if the buffer is empty or no source produces a candidate.
pub fn suggest(store: &HistoryStore, buffer: &str, context: &SuggestionContext) -> Option<String> {
    suggest_at(store, buffer, context, 0)
}

/// Two full-command strings name the same suggestion if they are equal once
/// a trailing path separator is ignored. History stores raw `cd target`
/// commands without the trailing '/' that directory completion appends, so
/// a byte-exact comparison would miss that duplicate.
fn same_suggestion(a: &str, b: &str) -> bool {
    a == b || a.trim_end_matches('/') == b.trim_end_matches('/')
}

/// Like [`suggest`], but returns the candidate at rank `skip` (0 = best),
/// powering fish-style cycling through alternatives. Merges history,
/// grammar, and directory completion into one ranked candidate list.
pub fn suggest_at(
    store: &HistoryStore,
    buffer: &str,
    context: &SuggestionContext,
    skip: u32,
) -> Option<String> {
    if buffer.is_empty() {
        return None;
    }

    let mut candidates: Vec<String> = Vec::new();

    // 1. Directory completion, first when eligible: it ranks candidates by
    //    real navigation history plus filesystem presence, a stronger signal
    //    for a partially-typed cd/pushd/... argument than the raw, un-slashed
    //    command text stored in general history.
    if predict::paths::is_eligible(buffer) {
        let cd_history = context
            .cwd
            .as_deref()
            .map(|cwd| store.cd_commands_for_cwd(cwd, 32).unwrap_or_default())
            .unwrap_or_default();
        for candidate in predict::paths::complete(buffer, context.cwd.as_deref(), &cd_history, 8) {
            if !candidates
                .iter()
                .any(|existing| same_suggestion(existing, &candidate))
            {
                candidates.push(candidate);
            }
        }
    }

    // 2. History: personalized full commands, frecency-ranked. When the
    //    top-ranked command is exactly the buffer, the user has fully typed
    //    the winner -- suggest no history continuation rather than surfacing
    //    a lower-ranked extension (pre-pipeline behavior, kept deliberately).
    let history = store
        .suggest_ranked_list(buffer, context, 8)
        .unwrap_or_default();
    if history.first().map(String::as_str) != Some(buffer) {
        for candidate in history {
            if !candidates
                .iter()
                .any(|existing| same_suggestion(existing, &candidate))
            {
                candidates.push(candidate);
            }
        }
    }

    // 3. Grammar pack: valid subcommands/flags the user may never have typed.
    for candidate in predict::grammar::complete(buffer, 8) {
        if !candidates
            .iter()
            .any(|existing| same_suggestion(existing, &candidate))
        {
            candidates.push(candidate);
        }
    }

    candidates
        .iter()
        .filter_map(|full| full.strip_prefix(buffer))
        .filter(|suffix| !suffix.is_empty())
        .nth(skip as usize)
        .map(str::to_string)
}
