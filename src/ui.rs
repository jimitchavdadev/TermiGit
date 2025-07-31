// src/ui.rs

use crate::app::{ActivePanel, App, AppMode};
use git2::Status;
use tui::{
    Frame,
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};
use tui_input::Input;

pub fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    // CORRECTED: Using percentages ensures the layout adapts to any terminal size.
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)].as_ref())
        .split(f.size());

    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(main_chunks[0]);

    draw_commits_panel(f, app, top_chunks[0]);
    draw_status_panel_with_help(f, app, top_chunks[1]);
    draw_diff_panel(f, app, main_chunks[1]);

    // Draw popups on top of everything if the mode requires it
    match &app.mode {
        AppMode::CommitInput => draw_commit_popup(f, app),
        AppMode::Pushing(msg) => draw_push_popup(f, msg),
        AppMode::Normal => {}
    }
}

fn draw_commits_panel<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    let is_active = matches!(app.active_panel, ActivePanel::Commits);
    let border_style = if is_active {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let highlight_bg = if is_active {
        Color::LightBlue
    } else {
        Color::DarkGray
    };

    let items: Vec<ListItem> = app
        .commits
        .iter()
        .map(|c| {
            ListItem::new(vec![Spans::from(vec![
                Span::styled(
                    &c.id[..7], // Short hash
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(" "),
                Span::raw(c.message.clone()),
            ])])
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Commits")
                .border_style(border_style),
        )
        .highlight_style(
            Style::default()
                .bg(highlight_bg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, area, &mut app.commit_list_state);
}

fn draw_status_panel_with_help<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);
    draw_status_panel(f, app, chunks[0]);
    draw_help(f, app, chunks[1]);
}

fn draw_status_panel<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    let is_active = matches!(app.active_panel, ActivePanel::Status);
    let border_style = if is_active {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let highlight_bg = if is_active {
        Color::LightBlue
    } else {
        Color::DarkGray
    };

    let items: Vec<ListItem> = app
        .status_files
        .iter()
        .map(|s| {
            let (prefix, style) = get_status_style(s.status);
            ListItem::new(Spans::from(vec![
                Span::styled(prefix, style),
                Span::raw(" "),
                Span::raw(s.path.clone()),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Working Directory")
                .border_style(border_style),
        )
        .highlight_style(
            Style::default()
                .bg(highlight_bg)
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(list, area, &mut app.status_list_state);
}

fn get_status_style(status: Status) -> (&'static str, Style) {
    if status.is_wt_new() {
        ("A ", Style::default().fg(Color::Green))
    } else if status.is_wt_modified() {
        ("M ", Style::default().fg(Color::Yellow))
    } else if status.is_wt_deleted() {
        ("D ", Style::default().fg(Color::Red))
    } else if status.is_wt_renamed() {
        ("R ", Style::default().fg(Color::Cyan))
    } else if status.is_index_new() {
        ("[A]", Style::default().fg(Color::Green))
    } else if status.is_index_modified() {
        ("[M]", Style::default().fg(Color::Yellow))
    } else if status.is_index_deleted() {
        ("[D]", Style::default().fg(Color::Red))
    } else if status.is_index_renamed() {
        ("[R]", Style::default().fg(Color::Cyan))
    } else {
        ("? ", Style::default().fg(Color::DarkGray))
    }
}

fn draw_help<B: Backend>(f: &mut Frame<B>, app: &App, area: Rect) {
    let help_text = match app.active_panel {
        ActivePanel::Commits => Text::from("↓↑: Navigate | <Tab>: Switch | <P>: Push | q: Quit"),
        ActivePanel::Status => Text::from(
            "↓↑: Navigate | <Space>: Stage/Unstage | <c>: Commit | <Tab>: Switch | q: Quit",
        ),
    };
    let help =
        Paragraph::new(help_text).block(Block::default().borders(Borders::ALL).title("Help"));
    f.render_widget(help, area);
}

fn draw_diff_panel<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    let diff_paragraph = Paragraph::new(app.diff_text.clone())
        .block(Block::default().borders(Borders::ALL).title("Diff"));
    f.render_widget(diff_paragraph, area);
}

fn centered_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(r.height.saturating_sub(height) / 2),
            Constraint::Length(height),
            Constraint::Length(r.height.saturating_sub(height) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn draw_commit_popup<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let area = centered_rect(60, 3, f.size());
    let input = Paragraph::new(app.commit_input.value()).style(Style::default().fg(Color::White));
    let block = Block::default()
        .title("Commit Message (Enter to submit, Esc to cancel)")
        .borders(Borders::ALL);
    f.render_widget(Clear, area);
    f.render_widget(input.block(block), area);
}

fn draw_push_popup<B: Backend>(f: &mut Frame<B>, msg: &str) {
    let area = centered_rect(50, 3, f.size());
    let text = Paragraph::new(msg).block(
        Block::default()
            .title("Pushing... (Press Enter to close)")
            .borders(Borders::ALL),
    );
    f.render_widget(Clear, area);
    f.render_widget(text, area);
}
