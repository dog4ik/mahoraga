use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};

use crate::app::App;

pub fn ui(frame: &mut Frame, app: &App) {
    let [history_area, input_area] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(3)]).areas(frame.area());

    render_history(frame, app, history_area);
    render_input(frame, app, input_area);
}

fn render_history(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::bordered().title(" mahoraga ");

    let lines: Vec<Line> = app
        .history
        .iter()
        .flat_map(|entry| {
            [
                Line::from(vec![
                    Span::styled("> ", Style::new().fg(Color::DarkGray)),
                    Span::styled(entry.input.as_str(), Style::new().fg(Color::Cyan)),
                ]),
                Line::from(Span::styled(
                    entry.output.as_str(),
                    Style::new().fg(Color::Green),
                )),
            ]
        })
        .collect();

    // Stick to the bottom like a chat: scroll past the lines that don't fit.
    let height = block.inner(area).height as usize;
    let scroll = lines
        .len()
        .saturating_sub(height)
        .try_into()
        .unwrap_or(u16::MAX);

    let history = Paragraph::new(lines).block(block).scroll((scroll, 0));
    frame.render_widget(history, area);
}

fn render_input(frame: &mut Frame, app: &App, area: Rect) {
    let (color, title) = match &app.preview {
        None => (Color::Gray, " input ".to_string()),
        Some(Ok(value)) => (Color::LightGreen, format!(" = {value} ")),
        Some(Err(err)) => (Color::LightRed, format!(" error: {err} ")),
    };

    let block = Block::bordered()
        .border_style(color)
        .title(Line::from(title).fg(color))
        .title_bottom(
            Line::from(" ↑↓: history · enter: submit · esc: quit ")
                .right_aligned()
                .dark_gray(),
        );

    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(&app.textarea, inner);
}
