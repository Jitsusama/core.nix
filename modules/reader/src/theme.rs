//! The theme's look, by role, which every surface the reader draws takes
//! from: its colours for the page, its code, its diagrams, the prompt and
//! the marks search lays over the page, and its faces for the text, the
//! headings and code. The module writes them as a file under the
//! configuration home, reader/theme.json, so a theme the account picks
//! reaches the reader as it reaches everything else; `READER_THEME` names
//! another file. The reader names no colour or face of its own: without a
//! file it sets the page as Typst does by itself, dark ink on white paper
//! in Typst's own faces.
use std::path::PathBuf;
use std::sync::OnceLock;

/// A colour, its red, green and blue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Colour(pub [u8; 3]);

impl Colour {
    /// Reads a hash and six hex digits, as the theme writes a colour.
    pub fn parse(hex: &str) -> Option<Self> {
        let hex = hex.strip_prefix('#')?;
        if hex.len() != 6 {
            return None;
        }
        let channel = |at: usize| u8::from_str_radix(hex.get(at..at + 2)?, 16).ok();
        Some(Colour([channel(0)?, channel(2)?, channel(4)?]))
    }

    /// As a hash and six hex digits.
    pub fn hex(self) -> String {
        let [r, g, b] = self.0;
        format!("#{r:02x}{g:02x}{b:02x}")
    }

    /// As GL takes it, each channel from 0 to 1.
    pub fn gl(self) -> [f32; 3] {
        self.0.map(|channel| f32::from(channel) / 255.0)
    }
}

/// The faces the reader sets in, by the theme's families.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Faces {
    /// What's read: the page's text, a diagram's words.
    pub text: String,
    /// Titles and headings, in the theme's wide face.
    pub wide: String,
    /// Code, the prompt, the key list and the labels over links.
    pub code: String,
}

/// The face Typst sets in when nothing names one.
const TYPSTS_OWN: &str = "libertinus serif";

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
    pub faces: Faces,
    /// The theme as a `TextMate` theme, for code; two-face's Nord without one.
    pub code: Option<PathBuf>,
}

impl Theme {
    /// Typst's page as it is with nothing asked of it: ink on white, a grey
    /// between them for what's quieter, in the faces Typst carries.
    fn plain() -> Self {
        let grey = |level: u8| Colour([level; 3]);
        Theme {
            background: grey(255),
            sunken: grey(242),
            raised: grey(230),
            text: grey(0),
            strong: grey(0),
            muted: grey(96),
            rule: grey(160),
            accent: grey(0),
            on_accent: grey(255),
            faces: Faces {
                text: TYPSTS_OWN.into(),
                wide: TYPSTS_OWN.into(),
                code: TYPSTS_OWN.into(),
            },
            code: None,
        }
    }

    /// Reads the theme's file: every role it names, Typst's plain page's for
    /// any it doesn't.
    fn read(text: &str) -> Self {
        let mut theme = Self::plain();
        let Ok(serde_json::Value::Object(roles)) = serde_json::from_str::<serde_json::Value>(text)
        else {
            return theme;
        };
        let named = |name: &str| roles.get(name).and_then(serde_json::Value::as_str);
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
            if let Some(colour) = named(name).and_then(Colour::parse) {
                *slot = colour;
            }
        }
        if let Some(serde_json::Value::Object(faces)) = roles.get("faces") {
            for (name, slot) in [
                ("text", &mut theme.faces.text),
                ("wide", &mut theme.faces.wide),
                ("code", &mut theme.faces.code),
            ] {
                if let Some(family) = faces.get(name).and_then(serde_json::Value::as_str) {
                    *slot = family.to_string();
                }
            }
        }
        theme.code = named("code").map(PathBuf::from);
        theme
    }

    /// The page's colours and faces as Typst names them, set before its
    /// template: `paint` makes a colour of the theme's red, green and blue.
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
            ("on_accent", self.on_accent),
        ];
        let named: Vec<String> = roles
            .iter()
            .map(|(name, Colour([r, g, b]))| format!("{name}: paint({r}, {g}, {b})"))
            .collect();
        format!(
            "#let paint = rgb\n#let ink = ({})\n#let face = (text: {:?}, wide: {:?}, code: {:?})\n",
            named.join(", "),
            self.faces.text,
            self.faces.wide,
            self.faces.code
        )
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
        _ => Theme::plain(),
    })
}

#[cfg(test)]
mod tests {
    use super::{Colour, Theme};

    #[test]
    fn a_theme_names_its_roles_and_faces_and_keeps_the_plain_page_for_the_rest() {
        let paper = Colour([250, 245, 230]);
        let ink = Colour([20, 15, 15]);
        let file = serde_json::json!({
            "background": paper.hex().to_uppercase(),
            "text": ink.hex(),
            "faces": {"text": "A Text Face", "code": "A Code Face"},
            "code": "/a/theme.tmTheme",
        });
        let theme = Theme::read(&file.to_string());
        assert_eq!(theme.background, paper);
        assert_eq!(theme.text, ink);
        assert_eq!(
            theme.strong,
            Theme::plain().strong,
            "a role it doesn't name stays"
        );
        assert_eq!(theme.faces.text, "A Text Face");
        assert_eq!(theme.faces.wide, Theme::plain().faces.wide);
        assert_eq!(
            theme.code.as_deref(),
            Some(std::path::Path::new("/a/theme.tmTheme"))
        );
        let typst = theme.typst();
        assert!(
            typst.contains("background: paint(250, 245, 230)"),
            "{typst}"
        );
        assert!(typst.contains("code: \"A Code Face\""), "{typst}");
    }

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "one channel's division, done the same way on both sides"
    )]
    fn a_colour_is_six_hex_digits_after_a_hash() {
        let jade = Colour([80, 148, 117]);
        assert_eq!(Colour::parse(&jade.hex()), Some(jade));
        assert_eq!(Colour::parse(&jade.hex().to_uppercase()), Some(jade));
        assert_eq!(Colour::parse(jade.hex().trim_start_matches('#')), None);
        assert_eq!(Colour::parse(&jade.hex()[..5]), None);
        assert_eq!(Colour::parse(&jade.hex().replace('5', "g")), None);
        assert_eq!(jade.gl()[0], 80.0 / 255.0);
    }
}
