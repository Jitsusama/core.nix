//! The theme's colours, by role, which every surface the reader draws takes
//! its ink from: the page, its code, its diagrams, the prompt and the marks
//! search lays over the page. The module writes them as a file under the
//! configuration home, reader/theme.json, so a theme the account picks
//! reaches the reader as it reaches everything else; `READER_THEME` names
//! another file. Without one the reader wears Osaka Jade.
use std::path::PathBuf;
use std::sync::OnceLock;

/// A colour, its red, green and blue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Colour(pub [u8; 3]);

impl Colour {
    /// Reads `#rrggbb`.
    pub fn parse(hex: &str) -> Option<Self> {
        let hex = hex.strip_prefix('#')?;
        if hex.len() != 6 {
            return None;
        }
        let channel = |at: usize| u8::from_str_radix(hex.get(at..at + 2)?, 16).ok();
        Some(Colour([channel(0)?, channel(2)?, channel(4)?]))
    }

    /// As `#rrggbb`.
    pub fn hex(self) -> String {
        let [r, g, b] = self.0;
        format!("#{r:02x}{g:02x}{b:02x}")
    }

    /// As GL takes it, each channel from 0 to 1.
    pub fn gl(self) -> [f32; 3] {
        self.0.map(|channel| f32::from(channel) / 255.0)
    }
}

/// The roles the reader draws with, as the theme names them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Theme {
    pub background: Colour,
    pub sunken: Colour,
    pub raised: Colour,
    pub text: Colour,
    pub strong: Colour,
    pub muted: Colour,
    pub rule: Colour,
    pub accent: Colour,
    pub on_accent: Colour,
    /// The theme as a `TextMate` theme, for code; two-face's Nord without one.
    pub code: Option<PathBuf>,
}

impl Theme {
    /// Osaka Jade, the theme's own pigments.
    fn osaka_jade() -> Self {
        let c = |hex| Colour::parse(hex).unwrap_or(Colour([0, 0, 0]));
        Theme {
            background: c("#111c18"),
            sunken: c("#0c1512"),
            raised: c("#23372b"),
            text: c("#d6d5bc"),
            strong: c("#f7e8b2"),
            muted: c("#b7b69d"),
            rule: c("#53685b"),
            accent: c("#509475"),
            on_accent: c("#090f0d"),
            code: None,
        }
    }

    /// Reads the theme's file: every role it names, Osaka Jade's for any it
    /// doesn't.
    fn read(text: &str) -> Self {
        let mut theme = Self::osaka_jade();
        let Ok(serde_json::Value::Object(roles)) = serde_json::from_str::<serde_json::Value>(text)
        else {
            return theme;
        };
        let role = |name: &str| {
            roles
                .get(name)
                .and_then(serde_json::Value::as_str)
                .and_then(Colour::parse)
        };
        for (name, slot) in [
            ("background", &mut theme.background),
            ("sunken", &mut theme.sunken),
            ("raised", &mut theme.raised),
            ("text", &mut theme.text),
            ("strong", &mut theme.strong),
            ("muted", &mut theme.muted),
            ("rule", &mut theme.rule),
            ("accent", &mut theme.accent),
            ("on_accent", &mut theme.on_accent),
        ] {
            if let Some(colour) = role(name) {
                *slot = colour;
            }
        }
        theme.code = roles
            .get("code")
            .and_then(serde_json::Value::as_str)
            .map(PathBuf::from);
        theme
    }

    /// The page's colours as Typst names them, set before its template.
    pub fn typst(&self) -> String {
        let roles = [
            ("background", self.background),
            ("sunken", self.sunken),
            ("raised", self.raised),
            ("text", self.text),
            ("strong", self.strong),
            ("muted", self.muted),
            ("rule", self.rule),
            ("accent", self.accent),
        ];
        let named: Vec<String> = roles
            .iter()
            .map(|(name, colour)| format!("{name}: rgb(\"{}\")", colour.hex()))
            .collect();
        format!("#let ink = ({})\n", named.join(", "))
    }
}

fn file() -> Option<PathBuf> {
    if let Some(named) = std::env::var_os("READER_THEME") {
        return Some(PathBuf::from(named));
    }
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(config.join("reader").join("theme.json"))
}

/// The theme, read once.
pub fn get() -> &'static Theme {
    static THEME: OnceLock<Theme> = OnceLock::new();
    THEME.get_or_init(|| match file().map(std::fs::read_to_string) {
        Some(Ok(text)) => Theme::read(&text),
        _ => Theme::osaka_jade(),
    })
}

#[cfg(test)]
mod tests {
    use super::{Colour, Theme};

    #[test]
    fn a_theme_names_its_roles_and_keeps_osaka_jade_for_the_rest() {
        let theme = Theme::read(
            r##"{"background": "#FFFCF0", "text": "#100F0F", "code": "/a/theme.tmTheme"}"##,
        );
        assert_eq!(theme.background, Colour([0xff, 0xfc, 0xf0]));
        assert_eq!(theme.text.hex(), "#100f0f");
        assert_eq!(
            theme.strong,
            Theme::osaka_jade().strong,
            "a role it doesn't name stays"
        );
        assert_eq!(
            theme.code.as_deref(),
            Some(std::path::Path::new("/a/theme.tmTheme"))
        );
        assert!(theme.typst().contains(r##"background: rgb("#fffcf0")"##));
    }

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "one channel's division, done the same way on both sides"
    )]
    fn a_colour_is_six_hex_digits_after_a_hash() {
        assert_eq!(Colour::parse("#509475"), Some(Colour([0x50, 0x94, 0x75])));
        assert_eq!(Colour::parse("509475"), None);
        assert_eq!(Colour::parse("#5094"), None);
        assert_eq!(Colour::parse("#50947g"), None);
        assert_eq!(Colour([80, 148, 117]).gl()[0], 80.0 / 255.0);
    }
}
