use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use super::*;

impl App {
    pub(super) fn handle_mouse(&mut self, mouse: MouseEvent) {
        match mouse.kind {
            MouseEventKind::ScrollUp => match self.focus {
                Focus::Panels => self.move_panel_up(),
                Focus::Detail => self.move_up(),
                Focus::Preview => {
                    self.detail_scroll = self.detail_scroll.saturating_sub(3);
                }
            },
            MouseEventKind::ScrollDown => match self.focus {
                Focus::Panels => self.move_panel_down(),
                Focus::Detail => self.move_down(),
                Focus::Preview => {
                    self.detail_scroll = self.detail_scroll.saturating_add(3);
                }
            },
            MouseEventKind::Down(MouseButton::Left) => {
                // Determine which area was clicked based on x position
                // The layout is 30% panels (left), 70% detail (right)
                let col = mouse.column;
                let row = mouse.row;
                let width = crossterm::terminal::size().map(|(w, _)| w).unwrap_or(80);
                let panel_width = width * 30 / 100;

                if col < panel_width {
                    // Clicked in panels area
                    self.focus = Focus::Panels;
                    // Map row to panel index (subtract border rows)
                    let panel_idx = row.saturating_sub(2) as usize; // 2 for top border + title
                    if panel_idx < PANELS.len() {
                        self.active_panel = PANELS[panel_idx];
                        self.detail_scroll = 0;
                    }
                } else {
                    // Clicked in detail area
                    if self.focus == Focus::Panels {
                        self.focus = Focus::Detail;
                    }
                    // Map row to item index
                    let item_idx = row.saturating_sub(2) as usize;
                    let max = self.active_panel.count(self);
                    let idx = self.active_panel.index();
                    if max > 0 {
                        self.panel_offsets[idx] = item_idx.min(max.saturating_sub(1));
                        self.detail_scroll = 0;
                    }
                }
            }
            _ => {}
        }
    }
}
