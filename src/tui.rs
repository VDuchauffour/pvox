use std::io::{self, stdout};

use crossterm::{
    ExecutableCommand,
    cursor::{Hide, Show},
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use ratatui::{Terminal, backend::CrosstermBackend};

pub struct Tui {
    pub terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl Tui {
    pub fn new() -> io::Result<Self> {
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = disable_raw_mode();
            let _ = stdout().execute(Show);
            let _ = stdout().execute(LeaveAlternateScreen);
            hook(info);
        }));

        enable_raw_mode()?;
        stdout()
            .execute(EnterAlternateScreen)?
            .execute(Hide)?
            .execute(Clear(ClearType::All))?;

        let terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

        Ok(Self { terminal })
    }

    pub fn leave(&mut self) -> io::Result<()> {
        disable_raw_mode()?;
        stdout().execute(Show)?.execute(LeaveAlternateScreen)?;
        Ok(())
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        let _ = self.leave();
    }
}
