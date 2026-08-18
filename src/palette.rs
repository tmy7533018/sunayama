use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::term::color::{Rgb, blend};
use crate::term::ini::Ini;

pub const DEFAULT_GRAINS: [Rgb; 4] = [
    Rgb::new(0x9a, 0x74, 0x4a),
    Rgb::new(0xc8, 0x93, 0x55),
    Rgb::new(0xe3, 0xba, 0x76),
    Rgb::new(0xff, 0xff, 0xff),
];

const DEPTH_ALPHA: (f64, f64) = (0.34, 0.84);
const ACCENT_ALPHA: f64 = 0.9;

pub struct Palette {
    depths: Vec<Rgb>,
    accents: [Rgb; 3],
}

impl Palette {
    pub fn new(grains: [Rgb; 4], bg: Rgb, rows: usize) -> Self {
        let (lo, hi) = DEPTH_ALPHA;
        let rows = rows.max(1);
        let depths = (0..rows)
            .map(|y| blend(grains[0], bg, lo + (y as f64 / rows as f64) * (hi - lo)))
            .collect();
        let accents = [
            blend(grains[1], bg, ACCENT_ALPHA),
            blend(grains[2], bg, ACCENT_ALPHA),
            blend(grains[3], bg, ACCENT_ALPHA),
        ];
        Self { depths, accents }
    }

    pub fn grain(&self, kind: u8, y: usize) -> Rgb {
        match kind {
            1 => self.depths[y.min(self.depths.len() - 1)],
            2..=4 => self.accents[usize::from(kind) - 2],
            _ => unreachable!("grain kind {kind} is out of range"),
        }
    }
}

pub fn load() -> Result<[Rgb; 4], String> {
    let Some(path) = config_path() else {
        return Ok(DEFAULT_GRAINS);
    };
    let Ok(text) = fs::read_to_string(&path) else {
        return Ok(DEFAULT_GRAINS);
    };
    let mut ini = Ini::parse(&text)?;

    if let Some(theme) = ini.get("color", "theme") {
        let theme_path = resolve_theme(&path, theme);
        let theme_text = fs::read_to_string(&theme_path)
            .map_err(|e| format!("theme {}: {e}", theme_path.display()))?;
        let mut theme_ini = Ini::parse(&theme_text)?;
        theme_ini.extend(ini);
        ini = theme_ini;
    }

    from_ini(&ini)
}

/// An empty or relative `XDG_CONFIG_HOME` would point at the cwd, not a config home.
fn config_path() -> Option<PathBuf> {
    let home = match env::var_os("XDG_CONFIG_HOME").map(PathBuf::from) {
        Some(dir) if dir.is_absolute() => dir,
        _ => Path::new(&env::var_os("HOME")?).join(".config"),
    };
    Some(home.join("sunayama/config"))
}

fn resolve_theme(config_path: &Path, theme: &str) -> PathBuf {
    let theme_path = Path::new(theme);
    if theme_path.is_absolute() {
        theme_path.to_path_buf()
    } else {
        config_path
            .parent()
            .map(|dir| dir.join(theme_path))
            .unwrap_or_else(|| theme_path.to_path_buf())
    }
}

fn from_ini(ini: &Ini) -> Result<[Rgb; 4], String> {
    let count = match ini.get("color", "gradient_count") {
        Some(text) => text
            .parse::<usize>()
            .map_err(|_| format!("[color] gradient_count {text:?} is not a number"))?
            .clamp(1, 4),
        None => (1..=4)
            .rev()
            .find(|n| ini.get("color", &format!("gradient_color_{n}")).is_some())
            .unwrap_or(0),
    };

    let mut grains = DEFAULT_GRAINS;
    for n in 1..=count {
        let key = format!("gradient_color_{n}");
        let text = ini
            .get("color", &key)
            .ok_or_else(|| format!("[color] {key} is missing"))?;
        grains[n - 1] =
            Rgb::from_hex(text).ok_or_else(|| format!("[color] {key} {text:?} is not a colour"))?;
    }
    Ok(grains)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_config_gives_the_default_grains() {
        let ini = Ini::parse("").unwrap();
        assert_eq!(from_ini(&ini).unwrap(), DEFAULT_GRAINS);
    }

    #[test]
    fn three_gradient_colours_leave_the_fourth_white() {
        let ini = Ini::parse(
            "[color]\ngradient_count = 3\ngradient_color_1 = '#111111'\ngradient_color_2 = '#222222'\ngradient_color_3 = '#333333'\n",
        )
        .unwrap();
        let grains = from_ini(&ini).unwrap();
        assert_eq!(grains[0], Rgb::new(0x11, 0x11, 0x11));
        assert_eq!(grains[1], Rgb::new(0x22, 0x22, 0x22));
        assert_eq!(grains[2], Rgb::new(0x33, 0x33, 0x33));
        assert_eq!(grains[3], DEFAULT_GRAINS[3]);
    }

    #[test]
    fn the_crest_is_faint_and_the_base_is_solid() {
        let bg = Rgb::new(0, 0, 0);
        let palette = Palette::new(DEFAULT_GRAINS, bg, 10);
        let crest = palette.grain(1, 0);
        let base = palette.grain(1, 9);
        let dist = |a: Rgb, b: Rgb| {
            (i32::from(a.r) - i32::from(b.r)).abs()
                + (i32::from(a.g) - i32::from(b.g)).abs()
                + (i32::from(a.b) - i32::from(b.b)).abs()
        };
        assert!(dist(crest, bg) < dist(base, bg));
    }

    #[test]
    fn accents_ignore_the_depth() {
        let palette = Palette::new(DEFAULT_GRAINS, Rgb::new(0, 0, 0), 20);
        assert_eq!(palette.grain(2, 0), palette.grain(2, 19));
        assert_eq!(palette.grain(4, 3), palette.grain(4, 17));
    }

    #[test]
    fn only_an_absolute_config_home_is_trusted() {
        // SAFETY: no other test reads HOME or XDG_CONFIG_HOME.
        unsafe {
            env::set_var("HOME", "/home/someone");
            for bogus in ["", ".", "relative/path"] {
                env::set_var("XDG_CONFIG_HOME", bogus);
                assert_eq!(
                    config_path().unwrap(),
                    PathBuf::from("/home/someone/.config/sunayama/config"),
                    "{bogus:?} should not have counted as a config home"
                );
            }
            env::set_var("XDG_CONFIG_HOME", "/xdg");
            assert_eq!(
                config_path().unwrap(),
                PathBuf::from("/xdg/sunayama/config")
            );
            env::remove_var("XDG_CONFIG_HOME");
            assert_eq!(
                config_path().unwrap(),
                PathBuf::from("/home/someone/.config/sunayama/config")
            );
            env::remove_var("HOME");
        }
    }

    #[test]
    fn a_bad_colour_names_the_key() {
        let ini =
            Ini::parse("[color]\ngradient_count = 1\ngradient_color_1 = 'not-a-colour'\n").unwrap();
        let err = from_ini(&ini).unwrap_err();
        assert!(err.contains("gradient_color_1"), "{err}");
    }
}
