//! Text colors, for Say Color (Insert+5): a page's text and background colors as the platform
//! gives them, and the names said for them.
//!
//! The names are a table of colors (`locales/en-US/colors.txt`): X11's names, as JAWS says
//! them, with the entries JAWS was seen to name otherwise [obs, 565, 618, 619]. A color not in
//! the table is said by its nearest entry's name.

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// The English color table: one `r g b<TAB>name` line per color, `#` lines are comments.
const TABLE: &str = include_str!("../locales/en-US/colors.txt");

/// The table, read once.
fn table() -> &'static [(Rgb, String)] {
    static PARSED: OnceLock<Vec<(Rgb, String)>> = OnceLock::new();
    PARSED.get_or_init(|| {
        TABLE
            .lines()
            .filter(|line| !line.starts_with('#'))
            .filter_map(|line| {
                let (rgb, name) = line.split_once('\t')?;
                let mut channels = rgb.split_whitespace().map(|c| c.parse::<u8>().ok());
                let mut channel = || channels.next().flatten();
                let color = Rgb::new(channel()?, channel()?, channel()?);
                Some((color, name.trim().to_owned()))
            })
            .collect()
    })
}

/// The name said for `color`: its table entry's, else the nearest entry's.
pub fn name_of(color: Rgb) -> Option<&'static str> {
    table()
        .iter()
        .min_by_key(|(entry, _)| distance(*entry, color))
        .map(|(_, name)| name.as_str())
}

/// How far apart two colors are, for the nearest name: the "redmean" distance, the squared
/// differences weighted by how red the two are (2 + r̄/256 for red, 4 for green, 2 + (255 −
/// r̄)/256 for blue), here times 512 to stay in integers. It names all 148 CSS colors, the
/// 216 colors of a 6 by 6 by 6 grid and #123456 as JAWS does [obs, 618, 619, 565/s15], where
/// the plain distance misses 37 of those 365 and fixed weights (2, 4, 3) 18.
fn distance(a: Rgb, b: Rgb) -> u32 {
    let d = |x: u8, y: u8| u32::from(x.abs_diff(y)).pow(2);
    let red_sum = u32::from(a.r) + u32::from(b.r);
    (1024 + red_sum) * d(a.r, b.r) + 2048 * d(a.g, b.g) + (1534 - red_sum) * d(a.b, b.b)
}

/// A color by its red, green and blue, each 0 to 255.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// A CSS color as Chromium writes it in IA2 text attributes: `rgb(0, 0, 238)`, or
    /// `rgba(0, 0, 0, 0)`. A fully transparent color is none: the background shows through.
    pub fn parse_css(raw: &str) -> Option<Self> {
        let raw = raw.trim();
        let inner = raw
            .strip_prefix("rgba(")
            .or_else(|| raw.strip_prefix("rgb("))?
            .strip_suffix(')')?;
        let mut parts = inner.split(',').map(str::trim);
        let mut channel = || parts.next()?.parse::<u8>().ok();
        let (r, g, b) = (channel()?, channel()?, channel()?);
        let alpha = parts.next().map(|a| a.parse::<f32>().ok());
        match alpha {
            Some(Some(a)) if a <= 0.0 => None,
            Some(None) => None,
            _ => Some(Self::new(r, g, b)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn css_colors_parse_as_chromium_writes_them() {
        assert_eq!(Rgb::parse_css("rgb(0, 0, 238)"), Some(Rgb::new(0, 0, 238)));
        assert_eq!(
            Rgb::parse_css("rgb(255,255,255)"),
            Some(Rgb::new(255, 255, 255))
        );
        assert_eq!(
            Rgb::parse_css("rgba(10, 20, 30, 0.5)"),
            Some(Rgb::new(10, 20, 30))
        );
        assert_eq!(Rgb::parse_css("rgba(0, 0, 0, 0)"), None);
        assert_eq!(Rgb::parse_css("transparent"), None);
        assert_eq!(Rgb::parse_css("rgb(300, 0, 0)"), None);
    }

    /// Names as JAWS says them: X11's in words, its numbered ones split at the capital, the
    /// web colors X11 names otherwise, and the nearest name for a color not in the table
    /// [obs, 565, 618].
    #[test]
    fn colors_are_named_as_jaws_names_them() {
        let name = |r, g, b| name_of(Rgb::new(r, g, b));
        assert_eq!(name(240, 248, 255), Some("alice blue"));
        assert_eq!(name(0, 0, 238), Some("blue2"));
        assert_eq!(name(144, 238, 144), Some("Pale Green2"));
        assert_eq!(name(0, 0, 139), Some("blue4"));
        assert_eq!(name(128, 128, 128), Some("gray"));
        assert_eq!(name(0, 128, 0), Some("lime green 2"));
        assert_eq!(name(169, 169, 169), Some("Dark Gray"));
        assert_eq!(name(220, 20, 60), Some("firebrick3"));
        assert_eq!(name(102, 51, 153), Some("Medium Orchid4"));
        assert_eq!(name(0x12, 0x34, 0x56), Some("dark slate gray"));
    }
}
