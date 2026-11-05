//! The theme's look, by role, which every surface the reader draws takes
//! from: its colours for the page, its code, its diagrams, the prompt, the
//! marks search lays over the page and a selection and its cursor, and its type's levels, which
//! every run of text the page and its foot set is at. The module writes them as a file under the
//! configuration home, reader/theme.json, so a theme the account picks
//! reaches the reader as it reaches everything else; `READER_THEME` names
//! another file. The reader names no colour or face of its own: without a
//! file it sets the page as Typst does by itself, dark ink on white paper
//! in Typst's own faces at Typst's own size.
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

/// A level of the theme's type as the page sets it: one kind of run of
/// text, its face, size, weight and the rest worked out by the module.
#[derive(Clone, Debug, PartialEq)]
pub struct Level {
    pub family: String,
    /// In points.
    pub size: f64,
    /// The face's weight; the serif's is its weight axis, which Typst sets
    /// from it.
    pub weight: u16,
    /// The variable face's other axes, by their tags.
    pub axes: Vec<(String, f64)>,
    pub italic: bool,
    /// Set in capitals, which say label.
    pub capitals: bool,
    /// Its letter spacing, in ems.
    pub tracking: f64,
    /// The OpenType features it's drawn with, by their tags.
    pub features: Vec<String>,
    /// The colour role it's drawn in, by its name.
    pub colour: String,
    /// The weight of a strong span in a line at this level, if it holds any.
    pub strong: Option<u16>,
}

impl Level {
    /// Typst's own face at Typst's own size, upright and regular.
    fn plain(colour: &str) -> Self {
        Level {
            family: TYPSTS_OWN.into(),
            size: TYPSTS_SIZE,
            weight: TYPSTS_WEIGHT,
            axes: Vec::new(),
            italic: false,
            capitals: false,
            tracking: 0.0,
            features: Vec::new(),
            colour: colour.into(),
            strong: None,
        }
    }

    /// Reads what the theme's file says of a level, keeping this one's own
    /// for anything it doesn't say or says wrongly.
    fn read(&mut self, said: &serde_json::Value) {
        use serde_json::Value;
        if let Some(family) = said.get("family").and_then(Value::as_str) {
            self.family = family.to_string();
        }
        if let Some(size) = said
            .get("size")
            .and_then(Value::as_f64)
            .filter(|size| *size > 0.0)
        {
            self.size = size;
        }
        let weight = |value: Option<&Value>| {
            value
                .and_then(Value::as_u64)
                .filter(|weight| (100..=900).contains(weight))
                .and_then(|weight| u16::try_from(weight).ok())
        };
        if let Some(weight) = weight(said.get("weight")) {
            self.weight = weight;
        }
        if let Some(Value::Object(axes)) = said.get("axes") {
            self.axes = axes
                .iter()
                .filter(|(tag, _)| is_tag(tag))
                .filter_map(|(tag, value)| Some((tag.clone(), value.as_f64()?)))
                .collect();
        }
        for (name, slot) in [
            ("italic", &mut self.italic),
            ("capitals", &mut self.capitals),
        ] {
            if let Some(said) = said.get(name).and_then(Value::as_bool) {
                *slot = said;
            }
        }
        if let Some(tracking) = said
            .get("tracking")
            .and_then(Value::as_f64)
            .filter(|tracking| tracking.is_finite())
        {
            self.tracking = tracking;
        }
        if let Some(Value::Array(features)) = said.get("features") {
            self.features = features
                .iter()
                .filter_map(Value::as_str)
                .filter(|tag| is_tag(tag))
                .map(str::to_string)
                .collect();
        }
        if let Some(colour) = said
            .get("colour")
            .and_then(Value::as_str)
            .filter(|colour| ROLES.contains(colour))
        {
            self.colour = colour.to_string();
        }
        match said.get("strong") {
            Some(Value::Null) => self.strong = None,
            Some(strong) => {
                if let Some(strong) = weight(Some(strong)) {
                    self.strong = Some(strong);
                }
            }
            None => {}
        }
    }

    /// The level as Typst takes it: `text`, the arguments `text` is set
    /// with, its fill one of `ink`'s roles; whether it's in capitals; and
    /// its strong span's weight, or none.
    fn typst(&self) -> String {
        let axes: Vec<String> = self
            .axes
            .iter()
            .map(|(tag, value)| format!("{tag:?}: {value}"))
            .collect();
        let features: Vec<String> = self
            .features
            .iter()
            .map(|tag| format!("{tag:?},"))
            .collect();
        format!(
            "(text: (font: {:?}, size: {}pt, weight: {}, variations: ({}), style: {:?}, tracking: {}em, features: ({}), fill: ink.{}), capitals: {}, strong: {})",
            self.family,
            self.size,
            self.weight,
            if axes.is_empty() {
                ":".into()
            } else {
                axes.join(", ")
            },
            if self.italic { "italic" } else { "normal" },
            self.tracking,
            features.concat(),
            self.colour,
            self.capitals,
            self.strong
                .map_or_else(|| "none".into(), |strong| strong.to_string()),
        )
    }
}

/// Whether a name is an OpenType tag: four letters or figures.
fn is_tag(name: &str) -> bool {
    name.len() == 4 && name.chars().all(|c| c.is_ascii_alphanumeric())
}

/// The page's tiers of type, each one of the theme's levels as the page
/// sets it (home.nix says which, and how the page bends it).
#[derive(Clone, Debug, PartialEq)]
pub struct Levels {
    /// The page's title (#).
    pub title: Level,
    /// A section (##).
    pub section: Level,
    /// A part (###).
    pub part: Level,
    /// The words, and a diagram's.
    pub words: Level,
    /// The lede and a quote.
    pub voice: Level,
    /// Code in a line, and a key.
    pub code: Level,
    /// A table's figures.
    pub figure: Level,
    /// A code block.
    pub listing: Level,
    /// An eyebrow, a table's heads, a group's name in the key list.
    pub label: Level,
    /// A caption, a record's field, the foot and what it lists.
    pub note: Level,
    /// The fleuron's asterisks.
    pub ornament: Level,
}

impl Levels {
    /// Typst's page with nothing asked of it: its own face, its words at
    /// its own size, and the steps the page has always taken above and
    /// below them, so a page without a theme still reads as one: a title
    /// and a section that stand out, a part and the voice in its italic, a
    /// strong span Typst's own 300 heavier, and the eyebrow small.
    fn plain() -> Self {
        let at = |size: f64, weight: u16, colour| Level {
            size,
            weight,
            ..Level::plain(colour)
        };
        let italic = |weight, colour| Level {
            italic: true,
            ..at(TYPSTS_SIZE, weight, colour)
        };
        Levels {
            title: Level {
                tracking: -0.02,
                ..at(42.0, 540, "strong")
            },
            section: at(17.1, 520, "strong"),
            part: italic(460, "strong"),
            words: Level {
                strong: Some(TYPSTS_WEIGHT + 300),
                ..Level::plain("text")
            },
            voice: italic(360, "text"),
            code: at(0.93 * TYPSTS_SIZE, TYPSTS_WEIGHT, "text"),
            figure: Level::plain("text"),
            listing: at(10.5, TYPSTS_WEIGHT, "text"),
            label: Level {
                capitals: true,
                ..at(8.4, 500, "muted")
            },
            note: at(10.5, TYPSTS_WEIGHT, "muted"),
            ornament: at(1.3 * TYPSTS_SIZE, 500, "accent"),
        }
    }

    fn each(&mut self) -> [(&'static str, &mut Level); 11] {
        [
            ("title", &mut self.title),
            ("section", &mut self.section),
            ("part", &mut self.part),
            ("words", &mut self.words),
            ("voice", &mut self.voice),
            ("code", &mut self.code),
            ("figure", &mut self.figure),
            ("listing", &mut self.listing),
            ("label", &mut self.label),
            ("note", &mut self.note),
            ("ornament", &mut self.ornament),
        ]
    }
}

/// The face Typst sets in when nothing names one.
pub const TYPSTS_OWN: &str = "libertinus serif";

/// The size Typst sets at when nothing names one.
const TYPSTS_SIZE: f64 = 11.0;

/// The weight Typst sets at when nothing names one: regular.
const TYPSTS_WEIGHT: u16 = 400;

/// The roles a level may be drawn in, as the page's `ink` names them.
const ROLES: [&str; 15] = [
    "background",
    "sunken",
    "raised",
    "text",
    "strong",
    "muted",
    "rule",
    "selection",
    "accent",
    "on_accent",
    "action",
    "program",
    "machine",
    "person",
    "voice",
];

/// The roles the reader draws with, as the theme names them.
#[derive(Clone, Debug, PartialEq)]
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
    /// The hues marks wear, each one kind of thing: an alert's rule takes
    /// its kind's.
    pub action: Colour,
    pub program: Colour,
    pub machine: Colour,
    pub person: Colour,
    pub voice: Colour,
    /// Behind what's selected, as everywhere on the desktop.
    pub selection: Colour,
    /// The terminal's cursor and the text under it, which visual mode's
    /// cursor wears, as kitty and Neovim draw it.
    pub cursor: Colour,
    pub cursor_text: Colour,
    pub levels: Levels,
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
            action: grey(0),
            program: grey(0),
            machine: grey(0),
            person: grey(0),
            voice: grey(0),
            selection: grey(204),
            cursor: grey(0),
            cursor_text: grey(255),
            levels: Levels::plain(),
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
            ("action", &mut theme.action),
            ("program", &mut theme.program),
            ("machine", &mut theme.machine),
            ("person", &mut theme.person),
            ("voice", &mut theme.voice),
            ("selection", &mut theme.selection),
            ("cursor", &mut theme.cursor),
            ("cursor_text", &mut theme.cursor_text),
        ] {
            if let Some(colour) = named(name).and_then(Colour::parse) {
                *slot = colour;
            }
        }
        if let Some(levels) = roles.get("levels") {
            for (name, level) in theme.levels.each() {
                if let Some(said) = levels.get(name) {
                    level.read(said);
                }
            }
        }
        theme.code = named("code").map(PathBuf::from);
        theme
    }

    /// The page's colours and levels as Typst names them, set before its
    /// template: `paint` makes a colour of the theme's red, green and blue,
    /// `level` holds each level, and `at` sets words at one, as
    /// `at(level.note, [words])`.
    pub fn typst(&self) -> String {
        let roles = [
            ("background", self.background),
            ("sunken", self.sunken),
            ("raised", self.raised),
            ("text", self.text),
            ("strong", self.strong),
            ("muted", self.muted),
            ("rule", self.rule),
            ("selection", self.selection),
            ("accent", self.accent),
            ("on_accent", self.on_accent),
            ("action", self.action),
            ("program", self.program),
            ("machine", self.machine),
            ("person", self.person),
            ("voice", self.voice),
        ];
        let named: Vec<String> = roles
            .iter()
            .map(|(name, Colour([r, g, b]))| format!("{name}: paint({r}, {g}, {b})"))
            .collect();
        let mut levels = self.levels.clone();
        let levels: Vec<String> = levels
            .each()
            .iter()
            .map(|(name, level)| format!("{name}: {}", level.typst()))
            .collect();
        format!(
            "#let paint = rgb\n#let ink = ({})\n#let level = ({})\n#let at(tier, body) = text(..tier.text, if tier.capitals {{ upper(body) }} else {{ body }})\n",
            named.join(", "),
            levels.join(", "),
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
    fn a_theme_names_its_roles_and_levels_and_keeps_the_plain_page_for_the_rest() {
        let paper = Colour([250, 245, 230]);
        let ink = Colour([20, 15, 15]);
        let file = serde_json::json!({
            "background": paper.hex().to_uppercase(),
            "text": ink.hex(),
            "levels": {
                "words": {
                    "family": "A Text Face", "size": 13.69, "weight": 380,
                    "axes": {"opsz": 14, "SOFT": 50, "far too long": 1},
                    "colour": "text", "strong": 480,
                },
                "label": {
                    "family": "A Code Face", "weight": 500, "capitals": true,
                    "tracking": 0.16, "features": ["case", "nope!"], "colour": "nowhere",
                },
                "part": {"italic": true, "strong": null, "weight": 1000},
            },
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
        let words = &theme.levels.words;
        assert_eq!(words.family, "A Text Face");
        assert!((words.size - 13.69).abs() < 1e-9, "{words:?}");
        assert_eq!((words.weight, words.strong), (380, Some(480)));
        assert_eq!(
            words.axes,
            [("opsz".to_string(), 14.0), ("SOFT".to_string(), 50.0)],
            "only the axes named by their tags, in the file's order"
        );
        let label = &theme.levels.label;
        assert_eq!(
            label.features,
            ["case"],
            "only features named by their tags"
        );
        assert_eq!(
            label.colour, "muted",
            "a role the page has no ink for stays"
        );
        assert!(label.capitals && (label.tracking - 0.16).abs() < 1e-9);
        assert_eq!(
            theme.levels.part.weight,
            Theme::plain().levels.part.weight,
            "a weight no face has stays"
        );
        assert_eq!(theme.levels.part.strong, None);
        assert_eq!(
            theme.levels.title,
            Theme::plain().levels.title,
            "a level it doesn't name stays"
        );
        assert!(
            (Theme::plain().levels.words.size - 11.0).abs() < 1e-9,
            "without a size, Typst's own"
        );
        assert_eq!(
            theme.code.as_deref(),
            Some(std::path::Path::new("/a/theme.tmTheme"))
        );
        let typst = theme.typst();
        assert!(
            typst.contains("background: paint(250, 245, 230)"),
            "{typst}"
        );
        assert!(
            typst.contains("words: (text: (font: \"A Text Face\", size: 13.69pt, weight: 380, variations: (\"opsz\": 14, \"SOFT\": 50), style: \"normal\", tracking: 0em, features: (), fill: ink.text), capitals: false, strong: 480)"),
            "{typst}"
        );
        assert!(
            typst.contains(
                "tracking: 0.16em, features: (\"case\",), fill: ink.muted), capitals: true"
            ),
            "{typst}"
        );
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
