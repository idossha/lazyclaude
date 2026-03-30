pub mod dashboard;
pub mod views;

use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::{App, InputMode, View};

pub fn render(frame: &mut Frame, app: &mut App) {
    match app.view {
        View::Dashboard => dashboard::render(frame, app),
        _ => views::render(frame, app),
    }

    // Render input/confirm overlay on top
    render_overlay(frame, app);

    // Render status message if present
    render_message(frame, app);
}

fn render_overlay(frame: &mut Frame, app: &App) {
    let area = frame.area();

    match &app.input_mode {
        InputMode::Normal => {}
        InputMode::Input(state) => {
            let input_area = Rect {
                x: area.x + 2,
                y: area.y + area.height.saturating_sub(4),
                width: area.width.saturating_sub(4),
                height: 3,
            };
            frame.render_widget(Clear, input_area);
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(format!(" {} ", state.prompt));
            let paragraph = Paragraph::new(state.value.as_str())
                .block(block)
                .style(Style::default().fg(Color::White));
            frame.render_widget(paragraph, input_area);

            frame.set_cursor_position((
                input_area.x + 1 + state.cursor as u16,
                input_area.y + 1,
            ));
        }
        InputMode::Confirm(state) => {
            let confirm_area = Rect {
                x: area.x + 2,
                y: area.y + area.height.saturating_sub(4),
                width: area.width.saturating_sub(4),
                height: 3,
            };
            frame.render_widget(Clear, confirm_area);
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(" Confirm ");
            let text = format!("{} (y/n)", state.message);
            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(Color::Yellow));
            frame.render_widget(paragraph, confirm_area);
        }
    }
}

fn render_message(frame: &mut Frame, app: &mut App) {
    if let Some(msg) = app.message.take() {
        let area = frame.area();
        let msg_area = Rect {
            x: area.x + 2,
            y: area.y + area.height.saturating_sub(1),
            width: area.width.saturating_sub(4).min(msg.len() as u16 + 2),
            height: 1,
        };
        let paragraph = Paragraph::new(msg).style(Style::default().fg(Color::Green));
        frame.render_widget(paragraph, msg_area);
    }
}
