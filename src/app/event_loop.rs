use super::*;

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while self.running {
            // Handle pending editor launch (needs terminal access)
            if let Some(path) = self.pending_edit.take() {
                let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());

                // Leave TUI mode
                crossterm::terminal::disable_raw_mode().ok();
                crossterm::execute!(
                    std::io::stdout(),
                    crossterm::terminal::LeaveAlternateScreen,
                    crossterm::cursor::Show
                )
                .ok();

                // Run editor — blocks until user closes it
                match std::process::Command::new(&editor).arg(&path).status() {
                    Ok(status) if !status.success() => {
                        self.set_message(format!("Editor exited with {status}"));
                    }
                    Err(e) => {
                        self.set_message(format!("Failed to launch editor '{editor}': {e}"));
                    }
                    _ => {}
                }

                // Return to TUI mode
                crossterm::execute!(
                    std::io::stdout(),
                    crossterm::terminal::EnterAlternateScreen,
                    crossterm::cursor::Hide
                )
                .ok();
                crossterm::terminal::enable_raw_mode().ok();

                // Force full redraw — ratatui's buffer is stale after editor
                terminal.clear()?;

                self.refresh();
                continue;
            }

            // Poll for background search results
            self.poll_search_results();

            // Check for file system changes (debounced)
            if let Some(ref rx) = self.watch_rx {
                if rx.try_recv().is_ok() {
                    // Drain any additional queued events (debounce)
                    while rx.try_recv().is_ok() {}
                    self.reload_data();
                    self.set_message("Auto-refreshed".to_string());
                }
            }

            terminal.draw(|frame| ui::render(frame, self))?;

            if event::poll(Duration::from_millis(50))? {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        self.handle_key(key);
                    }
                    Event::Mouse(mouse) => {
                        self.handle_mouse(mouse);
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}
