use std::io;

use crossterm::{
    cursor::{Hide, Show},
    event::{DisableFocusChange, EnableFocusChange},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, SetTitle,
    },
};

const TITLE: &str = "cat-task-manager";

pub struct TerminalGuard;

impl TerminalGuard {
    pub fn enter() -> Result<Self, Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        execute!(
            io::stdout(),
            EnterAlternateScreen,
            EnableFocusChange,
            Hide,
            SetTitle(TITLE)
        )?;
        Ok(Self)
    }

    pub fn suspend() -> Result<(), Box<dyn std::error::Error>> {
        disable_raw_mode()?;
        execute!(io::stdout(), DisableFocusChange, LeaveAlternateScreen, Show)?;
        Ok(())
    }

    pub fn resume() -> Result<(), Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        execute!(
            io::stdout(),
            EnterAlternateScreen,
            EnableFocusChange,
            Hide,
            SetTitle(TITLE)
        )?;
        Ok(())
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), DisableFocusChange, LeaveAlternateScreen, Show);
    }
}
