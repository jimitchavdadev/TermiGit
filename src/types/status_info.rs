// src/types/status_info.rs

use git2::Status;

pub struct StatusInfo {
    pub path: String,
    pub status: Status,
}
