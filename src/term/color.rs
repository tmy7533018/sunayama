use std::env;
use std::io::{self, IsTerminal, Write};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn from_hex(text: &str) -> Option<Self> {
        let text = text.strip_prefix('#').unwrap_or(text);
        match text.len() {
            6 => Some(Self::new(
                u8::from_str_radix(&text[0..2], 16).ok()?,
                u8::from_str_radix(&text[2..4], 16).ok()?,
                u8::from_str_radix(&text[4..6], 16).ok()?,
            )),
            3 => Some(Self::new(
                u8::from_str_radix(&text[0..1].repeat(2), 16).ok()?,
                u8::from_str_radix(&text[1..2].repeat(2), 16).ok()?,
                u8::from_str_radix(&text[2..3].repeat(2), 16).ok()?,
            )),
            _ => None,
        }
    }
}

pub fn blend(fg: Rgb, bg: Rgb, alpha: f64) -> Rgb {
    Rgb::new(
        blend_channel(fg.r, bg.r, alpha),
        blend_channel(fg.g, bg.g, alpha),
        blend_channel(fg.b, bg.b, alpha),
    )
}

fn blend_channel(fg: u8, bg: u8, alpha: f64) -> u8 {
    (f64::from(fg) * alpha + f64::from(bg) * (1.0 - alpha)).round() as u8
}

pub fn is_light(c: Rgb) -> bool {
    luma(c) > 0.5
}

fn luma(c: Rgb) -> f64 {
    (0.2126 * f64::from(c.r) + 0.7152 * f64::from(c.g) + 0.0722 * f64::from(c.b)) / 255.0
}

const OSC11_QUERY: &[u8] = b"\x1b]11;?\x07";
const OSC11_TIMEOUT: Duration = Duration::from_millis(100);
const OSC11_MAX: usize = 64;

/// Must run before crossterm's reader touches stdin, which eats the OSC 11 reply.
pub fn background() -> Option<Rgb> {
    if io::stdin().is_terminal()
        && let Some(rgb) = query_osc11()
    {
        return Some(rgb);
    }
    colorfgbg()
}

fn query_osc11() -> Option<Rgb> {
    io::stdout().write_all(OSC11_QUERY).ok()?;
    io::stdout().flush().ok()?;

    let deadline = Instant::now() + OSC11_TIMEOUT;
    let mut buf = [0u8; OSC11_MAX];
    let mut len = 0;
    while len < OSC11_MAX {
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            break;
        };
        if !poll_stdin(remaining) {
            break;
        }
        // A buffered reader would swallow the rest, leaving poll nothing to report.
        let n = unsafe { libc::read(0, buf[len..].as_mut_ptr().cast(), OSC11_MAX - len) };
        if n <= 0 {
            break;
        }
        len += n as usize;
        if ends_reply(&buf[..len]) {
            break;
        }
    }
    parse_osc11(&buf[..len])
}

fn ends_reply(buf: &[u8]) -> bool {
    buf.last() == Some(&0x07) || buf.ends_with(b"\x1b\\")
}

fn poll_stdin(timeout: Duration) -> bool {
    let mut fds = [libc::pollfd {
        fd: 0,
        events: libc::POLLIN,
        revents: 0,
    }];
    let millis = timeout.as_millis().min(i32::MAX as u128) as i32;
    let ready = unsafe { libc::poll(fds.as_mut_ptr(), 1, millis) };
    ready > 0 && (fds[0].revents & libc::POLLIN) != 0
}

pub(crate) fn parse_osc11(bytes: &[u8]) -> Option<Rgb> {
    let text = std::str::from_utf8(bytes).ok()?;
    let body = &text[text.find("rgb:")? + 4..];
    let end = body
        .find(|c: char| !(c.is_ascii_hexdigit() || c == '/'))
        .unwrap_or(body.len());
    let mut parts = body[..end].split('/');
    Some(Rgb::new(
        parse_channel(parts.next()?)?,
        parse_channel(parts.next()?)?,
        parse_channel(parts.next()?)?,
    ))
}

fn parse_channel(text: &str) -> Option<u8> {
    if text.is_empty() || text.len() > 4 {
        return None;
    }
    let value = u32::from_str_radix(text, 16).ok()?;
    let max = 16u32.pow(text.len() as u32) - 1;
    Some((value * 255 / max) as u8)
}

fn colorfgbg() -> Option<Rgb> {
    let value = env::var("COLORFGBG").ok()?;
    let index: usize = value.rsplit(';').next()?.parse().ok()?;
    ANSI_16.get(index).copied()
}

const ANSI_16: [Rgb; 16] = [
    Rgb::new(0x00, 0x00, 0x00),
    Rgb::new(0x80, 0x00, 0x00),
    Rgb::new(0x00, 0x80, 0x00),
    Rgb::new(0x80, 0x80, 0x00),
    Rgb::new(0x00, 0x00, 0x80),
    Rgb::new(0x80, 0x00, 0x80),
    Rgb::new(0x00, 0x80, 0x80),
    Rgb::new(0xc0, 0xc0, 0xc0),
    Rgb::new(0x80, 0x80, 0x80),
    Rgb::new(0xff, 0x00, 0x00),
    Rgb::new(0x00, 0xff, 0x00),
    Rgb::new(0xff, 0xff, 0x00),
    Rgb::new(0x00, 0x00, 0xff),
    Rgb::new(0xff, 0x00, 0xff),
    Rgb::new(0x00, 0xff, 0xff),
    Rgb::new(0xff, 0xff, 0xff),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ColorMode {
    True,
    Ansi256,
}

impl ColorMode {
    pub fn detect() -> Self {
        match env::var("COLORTERM").as_deref() {
            Ok("truecolor") | Ok("24bit") => Self::True,
            _ => Self::Ansi256,
        }
    }
}

const GRAY_TOLERANCE: u8 = 8;
const CUBE_BASE: u8 = 16;
const GRAY_BASE: u8 = 232;
const GRAY_STEPS: u16 = 23;

pub fn to_ansi256(c: Rgb) -> u8 {
    let max = c.r.max(c.g).max(c.b);
    let min = c.r.min(c.g).min(c.b);
    if max - min <= GRAY_TOLERANCE {
        let level = (u16::from(max) * GRAY_STEPS + 127) / 255;
        return GRAY_BASE + level as u8;
    }
    CUBE_BASE + 36 * cube_level(c.r) + 6 * cube_level(c.g) + cube_level(c.b)
}

fn cube_level(v: u8) -> u8 {
    ((u16::from(v) * 5 + 127) / 255) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gray_goes_to_the_ramp() {
        assert_eq!(to_ansi256(Rgb::new(0, 0, 0)), 232);
        assert_eq!(to_ansi256(Rgb::new(255, 255, 255)), 255);
    }

    #[test]
    fn saturated_goes_to_the_cube() {
        assert_eq!(to_ansi256(Rgb::new(255, 0, 0)), 16 + 36 * 5);
        assert_eq!(to_ansi256(Rgb::new(0, 0, 255)), 16 + 5);
    }

    #[test]
    fn hex_parses_with_and_without_the_hash() {
        assert_eq!(Rgb::from_hex("#a68cd9"), Some(Rgb::new(0xa6, 0x8c, 0xd9)));
        assert_eq!(Rgb::from_hex("a68cd9"), Some(Rgb::new(0xa6, 0x8c, 0xd9)));
        assert_eq!(Rgb::from_hex("#fff"), Some(Rgb::new(0xff, 0xff, 0xff)));
        assert_eq!(Rgb::from_hex("not-a-colour"), None);
    }

    #[test]
    fn blending_at_zero_and_one_is_the_endpoints() {
        let fg = Rgb::new(255, 200, 100);
        let bg = Rgb::new(10, 20, 30);
        assert_eq!(blend(fg, bg, 1.0), fg);
        assert_eq!(blend(fg, bg, 0.0), bg);
    }

    #[test]
    fn light_and_dark_backgrounds_are_told_apart() {
        assert!(!is_light(Rgb::new(0, 0, 0)));
        assert!(is_light(Rgb::new(255, 255, 255)));
    }

    #[test]
    fn an_osc11_reply_parses_at_every_digit_width() {
        assert_eq!(
            parse_osc11(b"\x1b]11;rgb:1e1e/1e1e/2e2e\x07"),
            Some(Rgb::new(30, 30, 46))
        );
        assert_eq!(
            parse_osc11(b"\x1b]11;rgb:1e/1e/2e\x1b\\"),
            Some(Rgb::new(30, 30, 46))
        );
        assert_eq!(parse_osc11(b"not an osc reply"), None);
    }
}
