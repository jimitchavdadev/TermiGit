// src/git.rs

use crate::types::{commit_info::CommitInfo, status_info::StatusInfo};
use git2::{self, Commit, DiffFormat, Repository, Sort};
use std::env;
use std::path::Path;
use tui::style::{Color, Style};
use tui::text::{Span, Spans};

// ... (fetch_log, fetch_status, stage_toggle are unchanged) ...
pub fn fetch_log(repo: &Repository) -> Result<Vec<CommitInfo>, git2::Error> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(Sort::TIME)?;

    let mut commits = Vec::new();
    for oid in revwalk {
        let oid = oid?;
        let commit: Commit = repo.find_commit(oid)?;
        let author = commit.author();

        commits.push(CommitInfo {
            id: oid.to_string(),
            message: commit.summary().unwrap_or("No commit message").to_string(),
            author: author.name().unwrap_or("Unknown").to_string(),
        });
    }
    Ok(commits)
}

pub fn fetch_status(repo: &Repository) -> Result<Vec<StatusInfo>, git2::Error> {
    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true);

    let statuses = repo.statuses(Some(&mut opts))?;
    Ok(statuses
        .iter()
        .map(|entry| StatusInfo {
            path: entry.path().unwrap_or("").to_string(),
            status: entry.status(),
        })
        .collect())
}

pub fn stage_toggle(repo: &Repository, file_path: &str) -> Result<(), git2::Error> {
    let mut index = repo.index()?;
    let path = Path::new(file_path);
    let status_entry = repo.status_file(path)?;

    let is_staged = status_entry.is_index_new()
        || status_entry.is_index_modified()
        || status_entry.is_index_deleted()
        || status_entry.is_index_renamed()
        || status_entry.is_index_typechange();

    if is_staged {
        repo.reset_default(None, &[path])?;
    } else {
        index.add_path(path)?;
    }
    index.write()?;
    Ok(())
}

// CORRECTED: Returns Vec<Spans<'static>>
fn format_diff(diff: &git2::Diff) -> Result<Vec<Spans<'static>>, git2::Error> {
    let mut lines = Vec::new();
    diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
        let style = match line.origin() {
            '+' => Style::default().fg(Color::Green),
            '-' => Style::default().fg(Color::Red),
            'H' | 'F' => Style::default().fg(Color::Cyan),
            _ => Style::default(),
        };
        // By using .to_string(), we create an owned String, which has a 'static lifetime.
        let content = format!(
            "{}{}",
            line.origin(),
            String::from_utf8_lossy(line.content())
        );
        lines.push(Spans::from(Span::styled(content, style)));
        true
    })?;
    Ok(lines)
}

// CORRECTED: Returns Vec<Spans<'static>>
pub fn get_commit_diff(
    repo: &Repository,
    commit: &CommitInfo,
) -> Result<Vec<Spans<'static>>, git2::Error> {
    let commit_oid = git2::Oid::from_str(&commit.id)?;
    let commit = repo.find_commit(commit_oid)?;
    let parent_commit = if commit.parent_count() > 0 {
        Some(commit.parent(0)?)
    } else {
        None
    };
    let tree = commit.tree()?;
    let parent_tree = parent_commit.as_ref().and_then(|p| p.tree().ok());
    let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;
    format_diff(&diff)
}

// CORRECTED: Returns Vec<Spans<'static>>
pub fn get_file_diff(
    repo: &Repository,
    file: &StatusInfo,
) -> Result<Vec<Spans<'static>>, git2::Error> {
    let diff = repo.diff_tree_to_workdir_with_index(
        None,
        Some(git2::DiffOptions::new().pathspec(&file.path)),
    )?;
    format_diff(&diff)
}

// create_commit is unchanged
pub fn create_commit(repo: &Repository, message: &str) -> Result<(), git2::Error> {
    let mut index = repo.index()?;
    let oid = index.write_tree()?;
    let tree = repo.find_tree(oid)?;
    let signature = repo.signature()?;
    let head = repo.head()?;
    if let Some(target) = head.target() {
        let parent_commit = repo.find_commit(target)?;
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &[&parent_commit],
        )?;
    } else {
        repo.commit(Some("HEAD"), &signature, &signature, message, &tree, &[])?;
    }
    Ok(())
}

// CORRECTED: No longer async. This is a blocking function.
pub fn push_to_remote(repo: &Repository) -> Result<(), git2::Error> {
    let mut remote = repo.find_remote("origin")?;
    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.credentials(|_url, username_from_url, _allowed_types| {
        let username = username_from_url.unwrap_or("git");
        if let Ok(cred) = git2::Cred::ssh_key_from_agent(username) {
            return Ok(cred);
        }
        let private_key = Path::new(&env::var("HOME").unwrap()).join(".ssh/id_rsa");
        git2::Cred::ssh_key(username, None, &private_key, None)
    });
    let mut push_options = git2::PushOptions::new();
    push_options.remote_callbacks(callbacks);
    let head = repo.head()?;
    let refspec = head.name().unwrap();
    remote.push(&[refspec], Some(&mut push_options))
}
