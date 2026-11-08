use chrono::Local;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use crate::app::{App, Focus, Status};

const ACCENT: Color = Color::Cyan;
const DIM: Color = Color::DarkGray;
const LABEL_WIDTH: usize = 10;

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let [_, clock, _, card, _, help] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Length(9),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(area);

    let now = Local::now();
    let clock_text = vec![
        Line::from(Span::styled(
            now.format("%H:%M").to_string(),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            now.format("%A, %-d %B").to_string().to_lowercase(),
            Style::default().fg(DIM),
        )),
    ];
    frame.render_widget(Paragraph::new(clock_text).alignment(Alignment::Center), clock);

    let card = center(card, 46);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if app.busy { Color::Yellow } else { DIM }))
        .title(Span::styled(format!(" {} ", app.host), Style::default().fg(ACCENT)))
        .title_alignment(Alignment::Center);
    let inner = block.inner(card);
    frame.render_widget(block, card);

    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(inner.inner(ratatui::layout::Margin::new(2, 0)));

    let in_auth = app.in_auth();
    frame.render_widget(
        field("user", &app.user, !in_auth && app.focus == Focus::User, in_auth),
        rows[1],
    );
    let name = app.session().name.to_lowercase();
    let session_focus = !in_auth && app.focus == Focus::Session;
    let session_text = if session_focus { format!("‹ {name} ›") } else { name };
    frame.render_widget(field("session", &session_text, session_focus, in_auth), rows[2]);

    let (label, masked) = match &app.prompt {
        Some((text, secret)) => (text.clone(), *secret),
        None => ("password".to_string(), true),
    };
    let shown = if masked {
        "•".repeat(app.secret.chars().count())
    } else {
        app.secret.clone()
    };
    let secret_focus = in_auth || app.focus == Focus::Secret;
    frame.render_widget(field(&label, &shown, secret_focus, false), rows[3]);

    let status = match &app.status {
        Status::Idle => Line::default(),
        Status::Info(m) => Line::from(Span::styled(m.clone(), Style::default().fg(ACCENT))),
        Status::Error(m) => Line::from(Span::styled(m.clone(), Style::default().fg(Color::Red))),
    };
    frame.render_widget(Paragraph::new(status).alignment(Alignment::Center), rows[5]);

    let mut hints = vec![
        "tab move",
        "←/→ session",
        "enter login",
        "esc clear",
        "f2 reboot",
        "f3 poweroff",
    ];
    if app.preview {
        hints.push("ctrl+c quit");
    }
    frame.render_widget(
        Paragraph::new(Span::styled(hints.join("  ·  "), Style::default().fg(DIM)))
            .alignment(Alignment::Center),
        help,
    );

    let cursor_row = if in_auth {
        Some((rows[3], label.chars().count(), shown.chars().count()))
    } else {
        match app.focus {
            Focus::User => Some((rows[1], 4, app.user.chars().count())),
            Focus::Secret => Some((rows[3], label.chars().count(), shown.chars().count())),
            Focus::Session => None,
        }
    };
    if let Some((row, label_len, len)) = cursor_row {
        let offset = LABEL_WIDTH.max(label_len + 1) + 2 + len;
        let x = row.x + (offset as u16).min(row.width.saturating_sub(1));
        frame.set_cursor_position((x, row.y));
    }
}

fn field<'a>(label: &str, value: &str, focused: bool, muted: bool) -> Paragraph<'a> {
    let marker = if focused { "› " } else { "  " };
    let label_style = if focused {
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(DIM)
    };
    let value_style = if muted { Style::default().fg(DIM) } else { Style::default().fg(Color::White) };
    let width = LABEL_WIDTH.max(label.chars().count() + 1);
    Paragraph::new(Line::from(vec![
        Span::styled(marker.to_string(), Style::default().fg(ACCENT)),
        Span::styled(format!("{label:<width$}"), label_style),
        Span::styled(value.to_string(), value_style),
    ]))
}

fn center(area: Rect, width: u16) -> Rect {
    let [area] = Layout::horizontal([Constraint::Length(width.min(area.width))])
        .flex(Flex::Center)
        .areas(area);
    area
}
