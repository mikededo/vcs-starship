//! Git repository info collection using git2.

use crate::error::{Error, Result};
use git2::{BranchType, Repository, RepositoryState, Status, StatusOptions};
use std::path::Path;

/// Repository state kinds that map to starship's `git_state` labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitRepoStateKind {
    Am,
    AmOrRebase,
    Bisect,
    CherryPick,
    Merge,
    Rebase,
    Revert,
}

impl GitRepoStateKind {
    #[must_use = "returns static display label"]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Am => "am",
            Self::AmOrRebase => "am/rebase",
            Self::Bisect => "bisect",
            Self::CherryPick => "cherry-pick",
            Self::Merge => "merge",
            Self::Rebase => "rebase",
            Self::Revert => "revert",
        }
    }
}

/// Progress information for multi-step repo states (rebase/am).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GitStateProgress {
    pub current: usize,
    pub total: usize,
}

/// Current repository state plus optional progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GitRepoStateInfo {
    pub kind: GitRepoStateKind,
    pub progress: Option<GitStateProgress>,
}

/// Git repository status info.
#[derive(Debug)]
pub struct GitInfo {
    /// Branch name (None if detached)
    pub branch: Option<String>,
    /// Upstream branch name (None if no upstream or detached)
    pub remote_branch: Option<String>,
    /// Short commit hash
    pub head_short: String,
    /// Count of staged files (excluding renamed)
    pub staged: usize,
    /// Count of modified (unstaged) files
    pub modified: usize,
    /// Count of untracked files
    pub untracked: usize,
    /// Count of deleted files
    pub deleted: usize,
    /// Count of conflicted files
    pub conflicted: usize,
    /// Count of renamed files
    pub renamed: usize,
    /// Count of stash entries
    pub stashed: usize,
    /// Commits ahead of upstream
    pub ahead: usize,
    /// Commits behind upstream
    pub behind: usize,
    /// Current repo state
    pub repo_state: Option<GitRepoStateInfo>,
}

/// Collect Git repo info from the given path.
#[must_use = "returns collected repo info, does not modify state"]
#[allow(clippy::too_many_lines)]
pub fn collect(repo_root: &Path, id_length: usize) -> Result<GitInfo> {
    let mut repo = Repository::open(repo_root).map_err(|e| Error::Git(format!("open: {e}")))?;

    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(false)
        .include_ignored(false)
        .exclude_submodules(true)
        .renames_head_to_index(true)
        .renames_index_to_workdir(true);

    let statuses = repo
        .statuses(Some(&mut opts))
        .map_err(|e| Error::Git(format!("statuses: {e}")))?;

    let mut staged = 0usize;
    let mut modified = 0usize;
    let mut untracked = 0usize;
    let mut deleted = 0usize;
    let mut conflicted = 0usize;
    let mut renamed = 0usize;

    for entry in statuses.iter() {
        let status = entry.status();

        if status.contains(Status::CONFLICTED) {
            conflicted += 1;
            continue;
        }

        if status.contains(Status::INDEX_RENAMED) {
            renamed += 1;
        }

        if status.intersects(
            Status::INDEX_NEW
                | Status::INDEX_MODIFIED
                | Status::INDEX_DELETED
                | Status::INDEX_TYPECHANGE,
        ) {
            staged += 1;
        }

        if status.intersects(Status::WT_MODIFIED | Status::WT_TYPECHANGE) {
            modified += 1;
        }
        if status.contains(Status::WT_DELETED) {
            deleted += 1;
        }
        if status.contains(Status::WT_NEW) {
            untracked += 1;
        }
    }

    drop(statuses);
    let stashed = collect_stash_count(&mut repo);
    let repo_state = detect_repo_state(&repo);

    let Ok(head) = repo.head() else {
        let branch = repo
            .find_reference("HEAD")
            .ok()
            .and_then(|r| r.symbolic_target().map(std::string::ToString::to_string))
            .and_then(|s| s.strip_prefix("refs/heads/").map(String::from));

        return Ok(GitInfo {
            branch,
            remote_branch: None,
            head_short: "empty".into(),
            staged,
            modified,
            untracked,
            deleted,
            conflicted,
            renamed,
            stashed,
            ahead: 0,
            behind: 0,
            repo_state,
        });
    };

    let detached = repo
        .head_detached()
        .map_err(|e| Error::Git(format!("head_detached: {e}")))?;

    let head_commit = head
        .peel_to_commit()
        .map_err(|e| Error::Git(format!("peel_to_commit: {e}")))?;
    let full_hash = head_commit.id().to_string();
    let head_short = full_hash[..id_length.min(full_hash.len())].to_string();

    let (branch, remote_branch, ahead, behind) = if detached {
        (None, None, 0, 0)
    } else {
        let branch = head.shorthand().map(String::from);
        if let Some(local_branch) = branch.as_deref() {
            let (remote_branch, ahead, behind) =
                get_upstream_info(&repo, local_branch, head_commit.id()).unwrap_or((None, 0, 0));
            (branch, remote_branch, ahead, behind)
        } else {
            (None, None, 0, 0)
        }
    };

    Ok(GitInfo {
        branch,
        remote_branch,
        head_short,
        staged,
        modified,
        untracked,
        deleted,
        conflicted,
        renamed,
        stashed,
        ahead,
        behind,
        repo_state,
    })
}

fn get_upstream_info(
    repo: &Repository,
    local_branch: &str,
    local_oid: git2::Oid,
) -> std::result::Result<(Option<String>, usize, usize), git2::Error> {
    let branch = repo.find_branch(local_branch, BranchType::Local)?;
    let upstream = branch.upstream()?;
    let upstream_ref = upstream.get();
    let remote_branch = upstream_ref
        .shorthand()
        .map(std::string::ToString::to_string);
    let upstream_oid = upstream_ref.peel_to_commit()?.id();
    let (ahead, behind) = repo.graph_ahead_behind(local_oid, upstream_oid)?;
    Ok((remote_branch, ahead, behind))
}

fn collect_stash_count(repo: &mut Repository) -> usize {
    let mut count = 0usize;
    if repo
        .stash_foreach(|_, _, _| {
            count += 1;
            true
        })
        .is_ok()
    {
        count
    } else {
        0
    }
}

fn detect_repo_state(repo: &Repository) -> Option<GitRepoStateInfo> {
    match repo.state() {
        RepositoryState::Clean => None,
        RepositoryState::Merge => Some(GitRepoStateInfo {
            kind: GitRepoStateKind::Merge,
            progress: None,
        }),
        RepositoryState::Revert | RepositoryState::RevertSequence => Some(GitRepoStateInfo {
            kind: GitRepoStateKind::Revert,
            progress: None,
        }),
        RepositoryState::CherryPick | RepositoryState::CherryPickSequence => {
            Some(GitRepoStateInfo {
                kind: GitRepoStateKind::CherryPick,
                progress: None,
            })
        }
        RepositoryState::Bisect => Some(GitRepoStateInfo {
            kind: GitRepoStateKind::Bisect,
            progress: None,
        }),
        RepositoryState::Rebase
        | RepositoryState::RebaseInteractive
        | RepositoryState::RebaseMerge => Some(GitRepoStateInfo {
            kind: GitRepoStateKind::Rebase,
            progress: read_rebase_progress(repo.path()),
        }),
        RepositoryState::ApplyMailbox => Some(GitRepoStateInfo {
            kind: GitRepoStateKind::Am,
            progress: read_am_progress(repo.path()),
        }),
        RepositoryState::ApplyMailboxOrRebase => Some(GitRepoStateInfo {
            kind: GitRepoStateKind::AmOrRebase,
            progress: read_am_progress(repo.path()),
        }),
    }
}

fn read_rebase_progress(git_dir: &Path) -> Option<GitStateProgress> {
    let rebase_merge = git_dir.join("rebase-merge");
    if rebase_merge.is_dir() {
        return read_progress(&rebase_merge, "msgnum", "end");
    }
    let rebase_apply = git_dir.join("rebase-apply");
    if rebase_apply.is_dir() {
        return read_progress(&rebase_apply, "next", "last");
    }
    None
}

fn read_am_progress(git_dir: &Path) -> Option<GitStateProgress> {
    let rebase_apply = git_dir.join("rebase-apply");
    if !rebase_apply.is_dir() {
        return None;
    }
    read_progress(&rebase_apply, "next", "last")
}

fn read_progress(
    state_dir: &Path,
    current_file: &str,
    total_file: &str,
) -> Option<GitStateProgress> {
    let current = std::fs::read_to_string(state_dir.join(current_file))
        .ok()?
        .trim()
        .parse::<usize>()
        .ok()?;
    let total = std::fs::read_to_string(state_dir.join(total_file))
        .ok()?
        .trim()
        .parse::<usize>()
        .ok()?;
    Some(GitStateProgress { current, total })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    fn run_git(repo_dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(["-c", "commit.gpgsign=false", "-c", "tag.gpgSign=false"])
            .args(args)
            .current_dir(repo_dir)
            .status()
            .expect("failed to run git command");
        assert!(status.success(), "git command failed: git {args:?}");
    }

    #[test]
    fn state_kind_labels_match_expected_strings() {
        assert_eq!(GitRepoStateKind::Am.label(), "am");
        assert_eq!(GitRepoStateKind::AmOrRebase.label(), "am/rebase");
        assert_eq!(GitRepoStateKind::Bisect.label(), "bisect");
        assert_eq!(GitRepoStateKind::CherryPick.label(), "cherry-pick");
        assert_eq!(GitRepoStateKind::Merge.label(), "merge");
        assert_eq!(GitRepoStateKind::Rebase.label(), "rebase");
        assert_eq!(GitRepoStateKind::Revert.label(), "revert");
    }

    #[test]
    fn reads_rebase_progress_from_rebase_merge_dir() {
        let tmp = TempDir::new().expect("tempdir");
        let git_dir = tmp.path().join(".git");
        fs::create_dir_all(git_dir.join("rebase-merge")).expect("create rebase-merge dir");
        fs::write(git_dir.join("rebase-merge/msgnum"), "2\n").expect("write msgnum");
        fs::write(git_dir.join("rebase-merge/end"), "5\n").expect("write end");

        let progress = read_rebase_progress(&git_dir).expect("progress");
        assert_eq!(
            progress,
            GitStateProgress {
                current: 2,
                total: 5
            }
        );
    }

    #[test]
    fn collect_counts_renamed_files() {
        let tmp = TempDir::new().expect("tempdir");
        run_git(tmp.path(), &["init", "-q"]);
        run_git(tmp.path(), &["config", "user.name", "test"]);
        run_git(tmp.path(), &["config", "user.email", "test@example.com"]);

        fs::write(tmp.path().join("a.txt"), "a").expect("write initial file");
        run_git(tmp.path(), &["add", "a.txt"]);
        run_git(tmp.path(), &["commit", "-qm", "initial"]);

        run_git(tmp.path(), &["mv", "a.txt", "b.txt"]);

        let info = collect(tmp.path(), 8).expect("collect info");
        assert_eq!(info.renamed, 1);
    }

    #[test]
    fn collect_counts_stashes() {
        let tmp = TempDir::new().expect("tempdir");
        run_git(tmp.path(), &["init", "-q"]);
        run_git(tmp.path(), &["config", "user.name", "test"]);
        run_git(tmp.path(), &["config", "user.email", "test@example.com"]);

        fs::write(tmp.path().join("a.txt"), "a").expect("write initial file");
        run_git(tmp.path(), &["add", "a.txt"]);
        run_git(tmp.path(), &["commit", "-qm", "initial"]);

        fs::write(tmp.path().join("a.txt"), "changed").expect("modify file");
        run_git(tmp.path(), &["stash", "push", "-q", "-m", "stash-one"]);

        let info = collect(tmp.path(), 8).expect("collect info");
        assert_eq!(info.stashed, 1);
    }

    #[test]
    fn detects_rebase_state_and_progress() {
        let tmp = TempDir::new().expect("tempdir");
        run_git(tmp.path(), &["init", "-q"]);
        let repo = Repository::open(tmp.path()).expect("open repo");

        let rebase_merge = repo.path().join("rebase-merge");
        fs::create_dir_all(&rebase_merge).expect("create rebase-merge dir");
        fs::write(rebase_merge.join("msgnum"), "3\n").expect("write msgnum");
        fs::write(rebase_merge.join("end"), "9\n").expect("write end");

        let state = detect_repo_state(&repo).expect("state");
        assert_eq!(state.kind, GitRepoStateKind::Rebase);
        assert_eq!(
            state.progress,
            Some(GitStateProgress {
                current: 3,
                total: 9
            })
        );
    }
}
