//! Output formatting for prompt strings.

use crate::color::{
    BLUE, BRIGHT_MAGENTA, CAT_GREEN, CAT_MAROON, CAT_MAUVE, CAT_PEACH, CAT_RED, CAT_SUBTEXT0,
    CAT_YELLOW, GREEN, PURPLE, RED, RESET, YELLOW,
};
use crate::config::Config;
#[cfg(feature = "git")]
use crate::git::{GitInfo, GitRepoStateKind};
use crate::jj::JjInfo;
#[cfg(feature = "git")]
use std::fmt::Write;

fn format_segment(text: &str, color: &str, show_color: bool) -> String {
    if show_color {
        format!("{color}{text}{RESET}")
    } else {
        text.to_string()
    }
}

/// Format shortest `change_id` output with JJ-style prefix coloring.
fn format_change_id(change_id: &str, show_prefix_color: bool) -> String {
    if show_prefix_color {
        format!("{BRIGHT_MAGENTA}{change_id}{RESET}")
    } else {
        change_id.to_string()
    }
}

/// Format JJ info as prompt string.
/// Pattern: `{symbol}{change_id} ({bookmarks}) [{status}]`
#[must_use = "returns formatted string, does not print"]
pub fn format_jj(info: &JjInfo, config: &Config) -> String {
    let mut out = String::with_capacity(128);
    let display = &config.jj_display;

    if display.show_prefix {
        out.push_str(&format_segment(&config.jj_symbol, BLUE, display.show_color));
    }

    if display.show_id {
        let use_prefix_color = display.show_color && display.show_prefix_color;
        if use_prefix_color {
            out.push_str(&format_change_id(&info.change_id, true));
        } else {
            out.push_str(&format_segment(&info.change_id, PURPLE, display.show_color));
        }
    }

    if display.show_name && !info.bookmarks.is_empty() {
        if !out.is_empty() {
            out.push(' ');
        }

        let total = info.bookmarks.len();
        let limit = config.bookmarks_display_limit;
        let show_count = if limit == 0 { total } else { limit.min(total) };
        let hidden = total.saturating_sub(show_count);

        let mut bookmark_strs: Vec<String> = info
            .bookmarks
            .iter()
            .take(show_count)
            .map(|(name, dist)| {
                let stripped = config.strip_prefix(name);
                let truncated = config.truncate(&stripped);
                if *dist > 0 {
                    format!("{truncated}~{dist}")
                } else {
                    truncated.into_owned()
                }
            })
            .collect();

        if hidden > 0 {
            bookmark_strs.push(format!("…+{hidden}"));
        }

        let bookmarks_text = format!("({})", bookmark_strs.join(", "));
        out.push_str(&format_segment(&bookmarks_text, GREEN, display.show_color));
    }

    if display.show_status {
        let mut status = String::with_capacity(8);
        if info.conflict {
            status.push('!');
        }
        if info.divergent {
            status.push('⇔');
        }
        if info.empty_desc && !info.empty_commit {
            status.push('∅');
        }
        if info.has_remote && !info.is_synced {
            status.push('⇡');
        }

        if !status.is_empty() {
            if !out.is_empty() {
                out.push(' ');
            }
            let status_text = format!("[{status}]");
            let color = if status == "∅" { YELLOW } else { RED };
            out.push_str(&format_segment(&status_text, color, display.show_color));
        }
    }

    out
}

/// Format Git info as prompt string.
///
/// Hardcoded contract:
/// - branch: `[branch(:remote_branch)]`
/// - state: `[state( current/total)]` for rebase/am where progress is available
/// - status: staged/modified, conflicted, untracked, deleted, `ahead_behind+stashed`, renamed
#[cfg(feature = "git")]
#[must_use = "returns formatted string, does not print"]
pub fn format_git(info: &GitInfo, config: &Config) -> String {
    let mut out = String::with_capacity(160);
    let display = &config.git_display;

    if display.show_name {
        let mut branch_text = info
            .branch
            .as_ref()
            .map_or_else(|| format!("HEAD@{}", info.head_short), Clone::clone);
        if let Some(remote_branch) = &info.remote_branch {
            branch_text.push(':');
            branch_text.push_str(remote_branch);
        }
        let branch_segment = format!("[{branch_text}] ");
        out.push_str(&format_segment(
            &branch_segment,
            CAT_MAUVE,
            display.show_color,
        ));
    }

    if display.show_status
        && let Some(repo_state) = info.repo_state
    {
        let mut state_text = repo_state.kind.label().to_string();
        let is_progress_state = matches!(
            repo_state.kind,
            GitRepoStateKind::Rebase | GitRepoStateKind::Am | GitRepoStateKind::AmOrRebase
        );
        if is_progress_state && let Some(progress) = repo_state.progress {
            state_text.push(' ');
            let _ = write!(state_text, "{}/{}", progress.current, progress.total);
        }
        let state_segment = format!("[{state_text}] ");
        out.push_str(&format_segment(
            &state_segment,
            CAT_YELLOW,
            display.show_color,
        ));
    }

    if display.show_status {
        let mut staged_modified = String::new();
        if info.staged > 0 {
            let _ = write!(staged_modified, "+{} ", info.staged);
        }
        if info.modified > 0 {
            let _ = write!(staged_modified, "!{} ", info.modified);
        }
        if !staged_modified.is_empty() {
            out.push_str(&format_segment(
                &staged_modified,
                CAT_PEACH,
                display.show_color,
            ));
        }

        if info.conflicted > 0 {
            let conflicted = format!("~{} ", info.conflicted);
            out.push_str(&format_segment(&conflicted, CAT_MAROON, display.show_color));
        }

        if info.untracked > 0 {
            let untracked = format!("?{} ", info.untracked);
            out.push_str(&format_segment(
                &untracked,
                CAT_SUBTEXT0,
                display.show_color,
            ));
        }

        if info.deleted > 0 {
            let deleted = format!("✘{} ", info.deleted);
            out.push_str(&format_segment(&deleted, CAT_RED, display.show_color));
        }

        let mut sync_and_stash = String::new();
        if info.ahead > 0 && info.behind > 0 {
            let _ = write!(sync_and_stash, "⇣{}⇡{} ", info.behind, info.ahead);
        } else {
            if info.ahead > 0 {
                let _ = write!(sync_and_stash, "⇡{} ", info.ahead);
            }
            if info.behind > 0 {
                let _ = write!(sync_and_stash, "⇣{} ", info.behind);
            }
        }
        if info.stashed > 0 {
            let _ = write!(sync_and_stash, "*{} ", info.stashed);
        }
        if !sync_and_stash.is_empty() {
            out.push_str(&format_segment(
                &sync_and_stash,
                CAT_GREEN,
                display.show_color,
            ));
        }

        if info.renamed > 0 {
            let renamed = format!("ɍ{} ", info.renamed);
            out.push_str(&format_segment(&renamed, CAT_PEACH, display.show_color));
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, DisplayConfig};
    #[cfg(feature = "git")]
    use crate::git::{GitRepoStateInfo, GitStateProgress};
    use std::borrow::Cow;

    fn base_config() -> Config {
        Config {
            truncate_name: 0,
            id_length: 8,
            ancestor_bookmark_depth: 10,
            bookmarks_display_limit: 0,
            strip_bookmark_prefix: Vec::new(),
            jj_symbol: Cow::Borrowed(""),
            git_symbol: Cow::Borrowed(""),
            jj_display: DisplayConfig::all_visible(),
            git_display: DisplayConfig::all_visible(),
        }
    }

    #[test]
    fn jj_format_unchanged_shape() {
        let info = JjInfo {
            change_id: "yzxv1234".into(),
            bookmarks: vec![("main".into(), 0)],
            empty_desc: false,
            empty_commit: false,
            conflict: false,
            divergent: false,
            has_remote: false,
            is_synced: true,
        };
        let rendered = format_jj(&info, &base_config());
        assert!(rendered.contains("yzxv"));
        assert!(rendered.contains("(main)"));
    }

    #[test]
    fn jj_shortest_id_has_no_dimmed_suffix() {
        let rendered = format_change_id("l", true);
        assert_eq!(rendered, format!("{BRIGHT_MAGENTA}l{RESET}"));
    }

    #[cfg(feature = "git")]
    #[test]
    fn git_branch_remote_and_status_format() {
        let info = GitInfo {
            branch: Some("main".into()),
            remote_branch: Some("origin/main".into()),
            head_short: "deadbeef".into(),
            staged: 2,
            modified: 1,
            untracked: 3,
            deleted: 4,
            conflicted: 5,
            renamed: 6,
            stashed: 7,
            ahead: 8,
            behind: 0,
            repo_state: Some(GitRepoStateInfo {
                kind: GitRepoStateKind::Merge,
                progress: None,
            }),
        };
        let config = base_config();
        let rendered = format_git(&info, &config);
        assert!(rendered.contains("[main:origin/main]"));
        assert!(rendered.contains("[merge]"));
        assert!(rendered.contains("+2 "));
        assert!(rendered.contains("!1 "));
        assert!(rendered.contains("~5 "));
        assert!(rendered.contains("?3 "));
        assert!(rendered.contains("✘4 "));
        assert!(rendered.contains("⇡8 "));
        assert!(rendered.contains("*7 "));
        assert!(rendered.contains("ɍ6 "));
    }

    #[cfg(feature = "git")]
    #[test]
    fn git_diverged_uses_combined_token() {
        let info = GitInfo {
            branch: Some("main".into()),
            remote_branch: None,
            head_short: "deadbeef".into(),
            staged: 0,
            modified: 0,
            untracked: 0,
            deleted: 0,
            conflicted: 0,
            renamed: 0,
            stashed: 0,
            ahead: 3,
            behind: 2,
            repo_state: None,
        };
        let rendered = format_git(&info, &base_config());
        assert!(rendered.contains("⇣2⇡3 "));
        assert!(!rendered.contains("⇡3 ⇣2 "));
    }

    #[cfg(feature = "git")]
    #[test]
    fn git_rebase_progress_is_rendered() {
        let info = GitInfo {
            branch: Some("main".into()),
            remote_branch: None,
            head_short: "deadbeef".into(),
            staged: 0,
            modified: 0,
            untracked: 0,
            deleted: 0,
            conflicted: 0,
            renamed: 0,
            stashed: 0,
            ahead: 0,
            behind: 0,
            repo_state: Some(GitRepoStateInfo {
                kind: GitRepoStateKind::Rebase,
                progress: Some(GitStateProgress {
                    current: 2,
                    total: 5,
                }),
            }),
        };
        let rendered = format_git(&info, &base_config());
        assert!(rendered.contains("[rebase 2/5]"));
    }

    #[cfg(feature = "git")]
    #[test]
    fn git_detached_head_fallback_uses_hash() {
        let info = GitInfo {
            branch: None,
            remote_branch: None,
            head_short: "deadbeef".into(),
            staged: 0,
            modified: 0,
            untracked: 0,
            deleted: 0,
            conflicted: 0,
            renamed: 0,
            stashed: 0,
            ahead: 0,
            behind: 0,
            repo_state: None,
        };
        let rendered = format_git(&info, &base_config());
        assert!(rendered.contains("[HEAD@deadbeef]"));
    }
}
