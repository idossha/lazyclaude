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
                    let panel_idx = row.saturating_sub(2) as usize;
                    if panel_idx < PANELS.len() {
                        self.active_panel = PANELS[panel_idx];
                        self.detail_scroll = 0;
                    }
                } else if self.active_panel == Panel::Stats {
                    // Stats panel: check heatmap hit
                    self.focus = Focus::Detail;
                    self.try_heatmap_click(col, row);
                } else {
                    // Clicked in detail area
                    if self.focus == Focus::Panels {
                        self.focus = Focus::Detail;
                    }
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

    fn try_heatmap_click(&mut self, col: u16, row: u16) {
        let (ox, oy) = self.stats_heatmap_origin;
        let n_cols = self.stats_heatmap_cols as usize;
        let base_w = self.stats_heatmap_base_w as usize;
        let extra = self.stats_heatmap_extra as usize;
        if n_cols == 0 || base_w == 0 || col < ox || row < oy {
            return;
        }
        let grid_row = (row - oy) as usize;
        // Map x to column: first `extra` cols are (base_w+1) wide, rest are base_w
        let dx = (col - ox) as usize;
        let wide_zone = extra * (base_w + 1);
        let grid_col = if dx < wide_zone {
            dx / (base_w + 1)
        } else {
            extra + (dx - wide_zone) / base_w
        };
        if grid_row >= 7 || grid_col >= n_cols {
            return;
        }
        let idx = grid_row * n_cols + grid_col;
        if let Some(date) = self.stats_heatmap_grid.get(idx) {
            if !date.is_empty() {
                self.stats_selected_date.clone_from(date);
            }
        }
    }
}
