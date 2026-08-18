mod cycle;
mod palette;
mod rng;
mod sand;
mod term;

use std::io::{self, BufWriter};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

use cycle::{Cycle, Endless, Pile, Start, Want};
use palette::Palette;
use rng::Rng;
use sand::Grid;
use term::color::{self, ColorMode, Rgb};
use term::frame::Frame;
use term::render::Renderer;
use term::screen::Screen;

const TICK: Duration = Duration::from_millis(33);
const SETTLES_PER_TICK: usize = 2;
const POUR_CAP: usize = 40;

const HELP: &str = "\
sunayama: playing with sand in your terminal

    sunayama                 sand piles up, tops out, and starts over
    sunayama --timer <dur>   fills over the time given: 90s, 25m, 1h

    Space                    more sand, in the default mode only
    q, Esc, Ctrl-C           quit

colours: ~/.config/sunayama/config
";

static RELOAD: AtomicBool = AtomicBool::new(false);

extern "C" fn mark_reload(_: libc::c_int) {
    RELOAD.store(true, Ordering::Relaxed);
}

/// A colour generator repaints a running pile with `pkill -USR1 sunayama`.
fn listen_for_reload() {
    unsafe {
        libc::signal(
            libc::SIGUSR1,
            mark_reload as *const () as libc::sighandler_t,
        )
    };
}

fn main() -> io::Result<()> {
    let period = match cycle::start_from_args(std::env::args().skip(1)) {
        Ok(Start::Pile(period)) => period,
        Ok(Start::Help) => {
            print!("{HELP}");
            return Ok(());
        }
        Err(complaint) => {
            eprintln!("sunayama: {complaint}\nsunayama: try --help");
            std::process::exit(2);
        }
    };

    let mut grains = match palette::load() {
        Ok(grains) => grains,
        Err(complaint) => {
            eprintln!("sunayama: {complaint}");
            std::process::exit(2);
        }
    };

    let bg = Screen::probe_background()?;
    if bg.is_some_and(color::is_light) {
        eprintln!(
            "sunayama: the terminal background looks light; the default palette is made for a dark one (set [color] gradient_color_1..4 in your config)"
        );
    }
    let bg = bg.unwrap_or(Rgb::new(0, 0, 0));

    listen_for_reload();

    let _screen = Screen::enter()?;
    let mut out = BufWriter::new(io::stdout());
    let mut renderer = Renderer::new(ColorMode::detect());
    let mut rng = Rng::new(seed());

    let (cols, rows) = Screen::size()?;
    let mut grid = Grid::new(cols, rows * 2);
    let mut frame = Frame::new(cols, rows * 2);
    let mut palette = Palette::new(grains, bg, grid.rows());

    let mut cycle = match period {
        Some(period) => Cycle::Timer {
            period,
            start: SystemTime::now(),
        },
        None => Cycle::Endless(Endless::opening(
            Instant::now(),
            sand::pile_full(grid.cols(), grid.rows()),
            &mut rng,
        )),
    };

    let mut resting = true;
    let mut next = Instant::now();
    loop {
        next += TICK;
        let deadline = next.max(Instant::now());

        while let Some(wait) = deadline.checked_duration_since(Instant::now()) {
            match event::poll(wait) {
                Ok(true) => {}
                Ok(false) => break,
                // A signal cuts the wait short; the tick carries on regardless.
                Err(error) if error.kind() == io::ErrorKind::Interrupted => break,
                Err(error) => return Err(error),
            }
            match event::read()? {
                Event::Key(key) if quits(&key) => return Ok(()),
                Event::Key(key) if key.code == KeyCode::Char(' ') && cycle.takes_input() => {
                    cycle.nudge(sand::pile_full(grid.cols(), grid.rows()));
                }
                Event::Resize(cols, rows) => {
                    let (cols, rows) = (cols as usize, rows as usize * 2);
                    let full = sand::pile_full(cols, rows);
                    grid = grid.resized(cols, rows);
                    grid.drain_to(cycle.asked(SystemTime::now(), full));
                    frame = Frame::new(cols, rows);
                    palette = Palette::new(grains, bg, grid.rows());
                }
                _ => {}
            }
        }
        next = deadline;

        if RELOAD.swap(false, Ordering::Relaxed)
            && let Ok(fresh) = palette::load()
        {
            grains = fresh;
            palette = Palette::new(grains, bg, grid.rows());
        }

        let full = sand::pile_full(grid.cols(), grid.rows());
        let pile = Pile {
            filled: grid.filled(),
            resting,
        };
        let want = cycle.want(Instant::now(), SystemTime::now(), full, pile, &mut rng);
        let mut sway = 0;
        match want {
            Want::Sand(goal) => {
                let owed = goal.saturating_sub(grid.filled());
                grid.pour(&mut rng, owed.min(POUR_CAP));
            }
            Want::Drain(offset) => {
                grid.drain_floor();
                sway = offset;
            }
            Want::Nothing => {}
        }
        let mut moving = false;
        for _ in 0..SETTLES_PER_TICK {
            moving |= grid.settle(&mut rng);
        }
        resting = !moving;

        paint(&grid, &mut frame, sway, &palette);
        renderer.render(&frame, &mut out)?;
    }
}

fn paint(grid: &Grid, frame: &mut Frame, sway: isize, palette: &Palette) {
    frame.clear();
    for y in 0..grid.rows() {
        for x in 0..grid.cols() {
            let kind = grid.at(x, y);
            if kind == 0 {
                continue;
            }
            if let Ok(shifted) = usize::try_from(x as isize + sway) {
                frame.set(shifted, y, palette.grain(kind, y));
            }
        }
    }
}

fn quits(key: &KeyEvent) -> bool {
    let ctrl_c = key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL);
    ctrl_c || matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
}

fn seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_nanos() as u64)
        .unwrap_or(1)
}
