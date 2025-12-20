use color_eyre::eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use msc_core::Player;
use ratatui::{DefaultTerminal, Frame};
use std::time::Duration;

pub struct App {
    running: bool,
    player: Player,
}

impl App {
    pub fn new() -> Result<Self> {
        Ok(Self {
            running: true,
            player: Player::new()?,
        })
    }

    pub fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn handle_events(&mut self) -> Result<()> {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => self.running = false,
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }

    fn render(&self, frame: &mut Frame) {
        frame.render_widget("q to quit", frame.area());
    }
}
