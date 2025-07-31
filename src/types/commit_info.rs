// src/types/commit_info.rs

#[derive(Clone, Debug)]
pub struct CommitInfo {
    pub id: String,
    pub message: String,
    pub author: String,
}
