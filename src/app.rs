// src/app.rs

use crate::git;
use crate::types::{commit_info::CommitInfo, status_info::StatusInfo};
use crossterm::event::{self, KeyCode, KeyEvent};
use git2::Repository;
use tokio::sync::mpsc;
use tui::text::Spans;
use tui::widgets::ListState;
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;

pub enum ActivePanel {
    Commits,
    Status,
}

pub enum AppMode {
    Normal,
    CommitInput,
    Pushing(String),
}

pub struct App {
    pub repo: Repository,
    pub should_quit: bool,
    pub active_panel: ActivePanel,
    pub mode: AppMode,
    pub commits: Vec<CommitInfo>,
    pub status_files: Vec<StatusInfo>,
    pub commit_list_state: ListState,
    pub status_list_state: ListState,
    pub diff_text: Vec<Spans<'static>>,
    pub commit_input: Input,
    pub push_feedback_sender: mpsc::Sender<String>,
    pub push_feedback_receiver: mpsc::Receiver<String>,
}

impl App {
    pub fn new() -> Result<Self, git2::Error> {
        let repo = Repository::open(".").expect("Couldn't open repository in current dir");
        let commits = git::fetch_log(&repo)?;
        let status_files = git::fetch_status(&repo)?;
        let (tx, rx) = mpsc::channel(1);

        let mut app = Self {
            repo,
            should_quit: false,
            active_panel: ActivePanel::Commits,
            mode: AppMode::Normal,
            commits,
            status_files,
            commit_list_state: ListState::default(),
            status_list_state: ListState::default(),
            diff_text: Vec::new(),
            commit_input: Input::default(),
            push_feedback_sender: tx,
            push_feedback_receiver: rx,
        };

        if !app.commits.is_empty() {
            app.commit_list_state.select(Some(0));
        }
        if !app.status_files.is_empty() {
            app.status_list_state.select(Some(0));
        }
        app.update_diff();

        Ok(app)
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) {
        match self.mode {
            AppMode::Normal => self.handle_normal_mode_keys(key),
            AppMode::CommitInput => self.handle_commit_input_keys(key),
            AppMode::Pushing(_) => {
                if let KeyCode::Enter | KeyCode::Esc = key.code {
                    self.mode = AppMode::Normal;
                }
            }
        }
    }

    fn handle_normal_mode_keys(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Tab => {
                self.active_panel = match self.active_panel {
                    ActivePanel::Commits => ActivePanel::Status,
                    ActivePanel::Status => ActivePanel::Commits,
                };
                self.update_diff();
            }
            KeyCode::Down => self.select_next(),
            KeyCode::Up => self.select_previous(),
            KeyCode::Char('c') => {
                if !self.status_files.is_empty() {
                    self.mode = AppMode::CommitInput;
                }
            }
            KeyCode::Char(' ') => {
                if let ActivePanel::Status = self.active_panel {
                    self.toggle_stage_selection();
                }
            }
            KeyCode::Char('P') => self.initiate_push(),
            _ => {}
        }
    }

    fn handle_commit_input_keys(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => self.submit_commit(),
            KeyCode::Esc => {
                self.commit_input.reset();
                self.mode = AppMode::Normal;
            }
            _ => {
                self.commit_input.handle_event(&event::Event::Key(key));
            }
        }
    }

    fn submit_commit(&mut self) {
        let message = self.commit_input.value();
        // CORRECTED: Collapsed the nested 'if' statement
        if !message.is_empty() && git::create_commit(&self.repo, message).is_ok() {
            self.commit_input.reset();
            self.mode = AppMode::Normal;
            self.refresh_all();
        }
    }

    fn initiate_push(&mut self) {
        self.mode = AppMode::Pushing("Pushing to origin...".to_string());
        let sender = self.push_feedback_sender.clone();
        let repo_path = self.repo.path().to_path_buf();

        tokio::task::spawn_blocking(move || {
            let result_msg = match Repository::open(repo_path) {
                Ok(repo) => match git::push_to_remote(&repo) {
                    Ok(_) => "Push successful!".to_string(),
                    // CORRECTED: Use modern f-string style formatting
                    Err(e) => format!("Push failed: {e}"),
                },
                // CORRECTED: Use modern f-string style formatting
                Err(e) => format!("Failed to open repo: {e}"),
            };
            let _ = sender.blocking_send(result_msg);
        });
    }

    fn toggle_stage_selection(&mut self) {
        if let Some(selected) = self.status_list_state.selected() {
            if let Some(item) = self.status_files.get(selected) {
                if git::stage_toggle(&self.repo, &item.path).is_ok() {
                    self.refresh_status();
                    self.update_diff();
                }
            }
        }
    }

    pub fn update_diff(&mut self) {
        let diff_result = match self.active_panel {
            ActivePanel::Commits => {
                if let Some(selected) = self.commit_list_state.selected() {
                    let commit_info = self.commits[selected].clone();
                    git::get_commit_diff(&self.repo, &commit_info)
                } else {
                    Ok(Vec::new())
                }
            }
            ActivePanel::Status => {
                if let Some(selected) = self.status_list_state.selected() {
                    if let Some(file_info) = self.status_files.get(selected) {
                        git::get_file_diff(&self.repo, file_info)
                    } else {
                        Ok(Vec::new())
                    }
                } else {
                    Ok(Vec::new())
                }
            }
        };

        self.diff_text = match diff_result {
            Ok(spans) => spans,
            // CORRECTED: Use modern f-string style formatting
            Err(e) => vec![Spans::from(format!("Could not load diff: {e}"))],
        };
    }

    fn refresh_all(&mut self) {
        self.commits = git::fetch_log(&self.repo).unwrap_or_default();
        if self.commits.is_empty() {
            self.commit_list_state.select(None);
        } else if self.commit_list_state.selected().is_none() {
            self.commit_list_state.select(Some(0));
        }
        self.refresh_status();
        self.update_diff();
    }

    fn refresh_status(&mut self) {
        self.status_files = git::fetch_status(&self.repo).unwrap_or_default();
        if self.status_files.is_empty() {
            self.status_list_state.select(None);
        } else {
            let selected_index = self.status_list_state.selected().unwrap_or(0);
            if selected_index >= self.status_files.len() {
                self.status_list_state
                    .select(Some(self.status_files.len() - 1));
            }
        }
    }

    fn select_next(&mut self) {
        let (list_len, state) = match self.active_panel {
            ActivePanel::Commits => (self.commits.len(), &mut self.commit_list_state),
            ActivePanel::Status => (self.status_files.len(), &mut self.status_list_state),
        };
        if list_len == 0 {
            return;
        }
        let i = state
            .selected()
            .map_or(0, |i| if i >= list_len - 1 { 0 } else { i + 1 });
        state.select(Some(i));
        self.update_diff();
    }

    fn select_previous(&mut self) {
        let (list_len, state) = match self.active_panel {
            ActivePanel::Commits => (self.commits.len(), &mut self.commit_list_state),
            ActivePanel::Status => (self.status_files.len(), &mut self.status_list_state),
        };
        if list_len == 0 {
            return;
        }
        let i = state
            .selected()
            .map_or(0, |i| if i == 0 { list_len - 1 } else { i - 1 });
        state.select(Some(i));
        self.update_diff();
    }
}
