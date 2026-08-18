use std::io::{self, Write};
use std::sync::Once;

use crossterm::{ExecutableCommand, cursor, terminal};

use super::color::{self, Rgb};

pub struct Screen(());

impl Screen {
    pub fn enter() -> io::Result<Self> {
        install_panic_hook();
        terminal::enable_raw_mode()?;
        let mut out = io::stdout();
        out.execute(terminal::EnterAlternateScreen)?;
        out.execute(cursor::Hide)?;
        Ok(Self(()))
    }

    pub fn size() -> io::Result<(usize, usize)> {
        let (cols, rows) = terminal::size()?;
        Ok((cols as usize, rows as usize))
    }

    /// Probes before the alternate screen so a warning survives on the normal buffer.
    pub fn probe_background() -> io::Result<Option<Rgb>> {
        terminal::enable_raw_mode()?;
        let bg = color::background();
        terminal::disable_raw_mode()?;
        Ok(bg)
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        restore();
    }
}

fn restore() {
    let mut out = io::stdout();
    let _ = out.execute(cursor::Show);
    let _ = out.execute(terminal::LeaveAlternateScreen);
    let _ = terminal::disable_raw_mode();
    let _ = out.flush();
}

fn install_panic_hook() {
    static HOOK: Once = Once::new();
    HOOK.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            restore();
            previous(info);
        }));
    });
}
