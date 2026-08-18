use std::io::{self, Write};

use super::color::{ColorMode, Rgb, to_ansi256};
use super::frame::Frame;

const UPPER: &str = "▀";
const LOWER: &str = "▄";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Cell {
    Empty,
    Upper(Rgb),
    Lower(Rgb),
    Both(Rgb, Rgb),
}

impl Cell {
    fn of(top: Option<Rgb>, bottom: Option<Rgb>) -> Self {
        match (top, bottom) {
            (None, None) => Self::Empty,
            (Some(t), None) => Self::Upper(t),
            (None, Some(b)) => Self::Lower(b),
            (Some(t), Some(b)) => Self::Both(t, b),
        }
    }
}

pub struct Renderer {
    mode: ColorMode,
    prev: Vec<Cell>,
    cur: Vec<Cell>,
    cols: usize,
    rows: usize,
}

impl Renderer {
    pub fn new(mode: ColorMode) -> Self {
        Self {
            mode,
            prev: Vec::new(),
            cur: Vec::new(),
            cols: 0,
            rows: 0,
        }
    }

    pub fn render(&mut self, frame: &Frame, out: &mut impl Write) -> io::Result<()> {
        let cols = frame.width();
        let rows = frame.height() / 2;
        if cols != self.cols || rows != self.rows {
            self.cols = cols;
            self.rows = rows;
            self.prev = vec![Cell::Empty; cols * rows];
            self.cur = vec![Cell::Empty; cols * rows];
            out.write_all(b"\x1b[2J")?;
        }

        for y in 0..rows {
            for x in 0..cols {
                self.cur[y * cols + x] = Cell::of(frame.get(x, y * 2), frame.get(x, y * 2 + 1));
            }
        }

        let mut pen = Pen::new(cols, self.mode);
        for y in 0..rows {
            for x in 0..cols {
                let i = y * cols + x;
                if self.cur[i] == self.prev[i] {
                    continue;
                }
                pen.move_to(out, x, y)?;
                pen.paint(out, self.cur[i])?;
            }
        }
        pen.finish(out)?;

        std::mem::swap(&mut self.prev, &mut self.cur);
        out.flush()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Bg {
    Unknown,
    Default,
    Color(Rgb),
}

struct Pen {
    cols: usize,
    mode: ColorMode,
    at: Option<(usize, usize)>,
    fg: Option<Rgb>,
    bg: Bg,
    painted: bool,
}

impl Pen {
    fn new(cols: usize, mode: ColorMode) -> Self {
        Self {
            cols,
            mode,
            at: None,
            fg: None,
            bg: Bg::Unknown,
            painted: false,
        }
    }

    fn move_to(&mut self, out: &mut impl Write, x: usize, y: usize) -> io::Result<()> {
        if self.at != Some((x, y)) {
            write!(out, "\x1b[{};{}H", y + 1, x + 1)?;
            self.at = Some((x, y));
        }
        Ok(())
    }

    fn paint(&mut self, out: &mut impl Write, cell: Cell) -> io::Result<()> {
        let glyph = match cell {
            Cell::Empty => {
                self.set_bg(out, Bg::Default)?;
                " "
            }
            Cell::Upper(t) => {
                self.set_fg(out, t)?;
                self.set_bg(out, Bg::Default)?;
                UPPER
            }
            Cell::Lower(b) => {
                self.set_fg(out, b)?;
                self.set_bg(out, Bg::Default)?;
                LOWER
            }
            Cell::Both(t, b) => {
                self.set_fg(out, t)?;
                self.set_bg(out, Bg::Color(b))?;
                UPPER
            }
        };
        out.write_all(glyph.as_bytes())?;
        self.painted = true;
        self.advance();
        Ok(())
    }

    // The last column leaves the cursor mid-wrap, so re-home instead of trusting DECAWM.
    fn advance(&mut self) {
        self.at = match self.at {
            Some((x, y)) if x + 1 < self.cols => Some((x + 1, y)),
            _ => None,
        };
    }

    fn set_fg(&mut self, out: &mut impl Write, c: Rgb) -> io::Result<()> {
        if self.fg == Some(c) {
            return Ok(());
        }
        match self.mode {
            ColorMode::True => write!(out, "\x1b[38;2;{};{};{}m", c.r, c.g, c.b)?,
            ColorMode::Ansi256 => write!(out, "\x1b[38;5;{}m", to_ansi256(c))?,
        }
        self.fg = Some(c);
        Ok(())
    }

    fn set_bg(&mut self, out: &mut impl Write, bg: Bg) -> io::Result<()> {
        if self.bg == bg {
            return Ok(());
        }
        match bg {
            Bg::Unknown => return Ok(()),
            Bg::Default => out.write_all(b"\x1b[49m")?,
            Bg::Color(c) => match self.mode {
                ColorMode::True => write!(out, "\x1b[48;2;{};{};{}m", c.r, c.g, c.b)?,
                ColorMode::Ansi256 => write!(out, "\x1b[48;5;{}m", to_ansi256(c))?,
            },
        }
        self.bg = bg;
        Ok(())
    }

    fn finish(&mut self, out: &mut impl Write) -> io::Result<()> {
        if self.painted {
            out.write_all(b"\x1b[m")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: Rgb = Rgb::new(10, 20, 30);
    const B: Rgb = Rgb::new(40, 50, 60);

    fn draw(frame: &Frame) -> String {
        let mut out = Vec::new();
        Renderer::new(ColorMode::True)
            .render(frame, &mut out)
            .unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn bottom_pixel_alone_uses_the_lower_half_block() {
        let mut f = Frame::new(1, 2);
        f.set(0, 1, A);
        assert!(draw(&f).contains(LOWER));
    }

    #[test]
    fn top_pixel_alone_uses_the_upper_half_block() {
        let mut f = Frame::new(1, 2);
        f.set(0, 0, A);
        let out = draw(&f);
        assert!(out.contains(UPPER));
        assert!(out.contains("\x1b[49m"));
    }

    #[test]
    fn both_pixels_pack_into_one_cell() {
        let mut f = Frame::new(1, 2);
        f.set(0, 0, A);
        f.set(0, 1, B);
        let out = draw(&f);
        assert!(out.contains("\x1b[38;2;10;20;30m"));
        assert!(out.contains("\x1b[48;2;40;50;60m"));
        assert_eq!(out.matches(UPPER).count(), 1);
    }

    #[test]
    fn an_unchanged_frame_writes_nothing() {
        let mut f = Frame::new(4, 4);
        f.set(1, 1, A);
        let mut r = Renderer::new(ColorMode::True);
        let mut first = Vec::new();
        r.render(&f, &mut first).unwrap();
        let mut second = Vec::new();
        r.render(&f, &mut second).unwrap();
        assert!(!first.is_empty());
        assert!(second.is_empty());
    }

    #[test]
    fn a_run_of_cells_moves_the_cursor_once() {
        let mut f = Frame::new(3, 2);
        for x in 0..3 {
            f.set(x, 0, A);
        }
        assert_eq!(draw(&f).matches("\x1b[").count(), 5);
    }
}
