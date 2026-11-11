//! Markdown to a Typst program: comrak writes GitHub's HTML with each
//! element's place in the source, ammonia keeps what GitHub's sanitizer
//! allows, html5ever parses it once, and one walk writes the program. Every
//! piece of the document's text goes in as a string literal, so nothing in it
//! can be read as markup or code.

mod code;
mod pictures;
mod tables;

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write;
use std::time::{Duration, Instant};

use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use syntect::highlighting::Theme;
use syntect::parsing::SyntaxSet;

use crate::world::PageWorld;

/// What one piece cost to make, for the budgets.
#[derive(Default, Debug)]
pub struct Costs {
    pub markdown: Duration,
    pub sanitize: Duration,
    pub parse: Duration,
    pub walk: Duration,
    pub diagrams: Vec<(usize, Duration, bool)>,
    pub equations: Vec<(Duration, bool)>,
    pub code: Duration,
}

pub struct Pieces {
    pub syntaxes: SyntaxSet,
    pub theme: Theme,
    pub mermaid: merman::Renderer,
    /// Diagrams already made, by their source, as a path in the world.
    pub made: RefCell<HashMap<String, Option<String>>>,
    pub converted: RefCell<HashMap<String, Option<String>>>,
}

impl Pieces {
    pub fn new() -> Self {
        let syntaxes = two_face::syntax::extra_newlines();
        let ink = crate::theme::get();
        // The theme's code colours, as bat and Neovim wear them; two-face's
        // Nord only where the theme gives none or it can't be read.
        let theme = ink
            .code
            .as_ref()
            .and_then(|path| syntect::highlighting::ThemeSet::get_theme(path).ok())
            .unwrap_or_else(|| {
                two_face::theme::extra()
                    .get(two_face::theme::EmbeddedThemeName::Nord)
                    .clone()
            });
        let config = merman::MermaidConfig::from_value(serde_json::json!({
            "theme": "base",
            "htmlLabels": false,
            "flowchart": { "htmlLabels": false },
            "fontFamily": ink.levels.words.family,
            "themeVariables": {
                "background": ink.background.hex(),
                "primaryColor": ink.raised.hex(),
                "primaryBorderColor": ink.accent.hex(),
                "primaryTextColor": ink.text.hex(),
                "lineColor": ink.muted.hex(),
                "textColor": ink.text.hex()
            }
        }));
        let mermaid =
            merman::Renderer::new().with_engine(merman::Engine::new().with_site_config(config));
        Self {
            syntaxes,
            theme,
            mermaid,
            made: RefCell::new(HashMap::new()),
            converted: RefCell::new(HashMap::new()),
        }
    }
}

pub fn options() -> comrak::Options<'static> {
    let mut o = comrak::Options::default();
    let e = &mut o.extension;
    e.strikethrough = true;
    e.table = true;
    e.autolink = true;
    e.tasklist = true;
    e.footnotes = true;
    e.alerts = true;
    e.math_dollars = true;
    e.math_code = true;
    e.shortcodes = true;
    e.header_id_prefix = Some(String::new());
    e.front_matter_delimiter = Some("---".into());
    o.render.r#unsafe = true;
    o.render.github_pre_lang = true;
    o.render.sourcepos = true;
    o
}

fn sanitizer() -> ammonia::Builder<'static> {
    let mut b = ammonia::Builder::default();
    b.add_generic_attributes([
        "data-sourcepos",
        "class",
        "data-math-style",
        "align",
        "lang",
        "id",
    ])
    .add_tags([
        "input", "section", "details", "summary", "kbd", "picture", "source",
    ])
    .add_tag_attributes("input", ["type", "checked", "disabled"])
    .add_tag_attributes("img", ["width", "height", "src", "alt"])
    .add_tag_attributes("source", ["media", "srcset"])
    .id_prefix(None)
    .link_rel(None);
    b
}

/// The page's program at `width`, with what each step took. `about` is
/// the eyebrow's words over the title: what the file is and when it
/// changed.
pub fn program(
    markdown: &str,
    about: &str,
    base: Option<&std::path::Path>,
    pieces: &Pieces,
    world: &PageWorld,
    width: crate::page::Width,
    costs: &mut Costs,
) -> String {
    let t = Instant::now();
    let html = comrak::markdown_to_html(markdown, &options());
    costs.markdown = t.elapsed();

    let t = Instant::now();
    let clean = sanitizer().clean(&html).to_string();
    costs.sanitize = t.elapsed();

    let t = Instant::now();
    let dom =
        html5ever::parse_document(RcDom::default(), html5ever::ParseOpts::default()).one(clean);
    costs.parse = t.elapsed();

    let t = Instant::now();
    let mut out = String::with_capacity(markdown.len() * 3);
    let template = crate::page::template(world, width);
    out.push_str(&template);
    let mut walker = Walker {
        pieces,
        world,
        costs,
        out: &mut out,
        base,
        pictures: 0,
        blocks: 0,
        after_title: false,
        titled: false,
        after_box: false,
        caption: None,
        source: markdown.lines().collect(),
    };
    walker.children(&dom.document);
    let (titled, blocks) = (walker.titled, walker.blocks);
    if blocks > 0 {
        end_with_its_mark(&mut out);
    }
    // The eyebrow goes before any block, so the first set carries it
    // wherever it starts, and sits closer over a title than over words.
    out.insert_str(
        template.len(),
        &format!("#about(titled: {titled})[#{}]\n", literal(about)),
    );
    costs.walk = t.elapsed();
    out
}

/// Each block of the program at `width` on its own, for setting the first
/// screen alone.
pub fn blocks(
    program: &str,
    world: &PageWorld,
    width: crate::page::Width,
) -> (String, Vec<String>) {
    let (head, body) = program.split_at(crate::page::template(world, width).len());
    (
        head.to_string(),
        body.split("\n// block\n").map(str::to_string).collect(),
    )
}

pub fn literal(text: &str) -> String {
    let mut s = String::with_capacity(text.len() + 2);
    s.push('"');
    for c in text.chars() {
        match c {
            '"' => s.push_str("\\\""),
            '\\' => s.push_str("\\\\"),
            '\n' => s.push_str("\\n"),
            '\r' => {}
            '\t' => s.push_str("\\t"),
            c => s.push(c),
        }
    }
    s.push('"');
    s
}

/// A run of whitespace is one space, as HTML has it: a soft break between
/// two lines of a paragraph never joins their words.
fn collapse(text: &str) -> String {
    let mut s = String::with_capacity(text.len());
    let mut space = false;
    for c in text.chars() {
        if c.is_whitespace() {
            if !space {
                s.push(' ');
            }
            space = true;
        } else {
            s.push(c);
            space = false;
        }
    }
    s
}

struct Walker<'a> {
    pieces: &'a Pieces,
    world: &'a PageWorld,
    costs: &'a mut Costs,
    out: &'a mut String,
    /// The folder the file is in, which its pictures' paths start from.
    base: Option<&'a std::path::Path>,
    /// How many pictures the world holds, to name the next.
    pictures: usize,
    /// How many blocks the page has, to number the next.
    blocks: usize,
    /// Whether the block just walked was the title, so the next paragraph
    /// is its lede.
    after_title: bool,
    /// Whether the page opens with its title.
    titled: bool,
    /// Whether a task's box was just passed over, so the space after it
    /// goes.
    after_box: bool,
    /// The alt text of the last picture set, for its caption; none when
    /// it stood as a box saying it.
    caption: Option<String>,
    /// The file's lines, to tell a fenced block of code from an indented
    /// one.
    source: Vec<&'a str>,
}

/// The lines of the file an element was written on, from 1, as comrak
/// says them.
fn written_on(node: &Handle) -> Option<(usize, usize)> {
    let place = attr(node, "data-sourcepos")?;
    let (from, to) = place.split_once('-')?;
    let line = |at: &str| at.split_once(':')?.0.parse::<usize>().ok();
    Some((line(from)?, line(to)?))
}

/// Marks where a piece of the file that's selected whole starts on the
/// page, with the lines it was written on: a paragraph, a heading, a
/// list's item, a table's row or a line of code, which V and y y take as a
/// whole and copy as its lines (select/ reads them back). A block's own
/// number is an integer; this is a string, so the two are never mistaken.
fn unit(out: &mut String, (first, last): (usize, usize)) {
    let _ = write!(out, "#metadata(\"s {first}-{last}\")");
}

fn attr(node: &Handle, name: &str) -> Option<String> {
    if let NodeData::Element { attrs, .. } = &node.data {
        attrs
            .borrow()
            .iter()
            .find(|a| &*a.name.local == name)
            .map(|a| a.value.to_string())
    } else {
        None
    }
}

fn tag(node: &Handle) -> Option<String> {
    if let NodeData::Element { name, .. } = &node.data {
        Some(name.local.to_string())
    } else {
        None
    }
}

/// The tag of the element `node` sits in, where it sits in one.
fn parent_tag(node: &Handle) -> Option<String> {
    let parent = node.parent.take();
    let tag = parent
        .as_ref()
        .and_then(std::rc::Weak::upgrade)
        .and_then(|p| tag(&p));
    node.parent.set(parent);
    tag
}

fn text_of(node: &Handle) -> String {
    fn go(n: &Handle, s: &mut String) {
        if let NodeData::Text { contents } = &n.data {
            s.push_str(&contents.borrow());
        }
        for c in n.children.borrow().iter() {
            go(c, s);
        }
    }
    let mut s = String::new();
    go(node, &mut s);
    s
}

/// What a block's metadata says of it, as a Typst dictionary: its number,
/// the source lines comrak says it was written on, and a heading's level
/// and words (place.rs reads it back).
fn block_said(node: &Handle, name: &str, number: usize) -> String {
    // comrak's sourcepos is `3:1-5:10`: lines and columns, from and to.
    let lines = attr(node, "data-sourcepos").and_then(|pos| {
        let (from, to) = pos.split_once('-')?;
        let line = |at: &str| at.split(':').next()?.parse::<usize>().ok();
        Some((line(from)?, line(to)?))
    });
    let lines = lines.map_or_else(|| "none".into(), |(from, to)| format!("({from}, {to})"));
    let heading = match name.as_bytes() {
        [b'h', level @ b'1'..=b'6'] => format!(
            "({}, {})",
            level - b'0',
            literal(collapse(&text_of(node)).trim())
        ),
        _ => "none".into(),
    };
    format!("(block: {number}, lines: {lines}, heading: {heading})")
}

/// A picture's kind by its first bytes, as Typst names it, for the kinds
/// Typst sets: PNG, JPEG, GIF, WebP, SVG and PDF.
pub fn picture_kind(bytes: &[u8]) -> Option<&'static str> {
    let head = &bytes[..bytes.len().min(512)];
    if head.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("png")
    } else if head.starts_with(b"\xff\xd8\xff") {
        Some("jpg")
    } else if head.starts_with(b"GIF87a") || head.starts_with(b"GIF89a") {
        Some("gif")
    } else if head.len() >= 12 && &head[..4] == b"RIFF" && &head[8..12] == b"WEBP" {
        Some("webp")
    } else if head.starts_with(b"%PDF-") {
        Some("pdf")
    } else if String::from_utf8_lossy(head).contains("<svg") {
        Some("svg")
    } else {
        None
    }
}

/// Ends the page with its mark: after the last word when the last block is
/// a paragraph, else on a line of its own.
fn end_with_its_mark(out: &mut String) {
    let last = out.rfind("\n// block\n").unwrap_or(0);
    let body = out[last..]
        .split_once(")\n")
        .map_or("", |(_, body)| body)
        .trim_end();
    let words = body.starts_with("#par[") || body.starts_with("#lede[");
    if words
        && body.ends_with(']')
        && let Some(close) = out.rfind(']')
    {
        out.insert_str(close, "#h(0.3em)#the-end");
    } else {
        out.push_str("\n#par[#the-end]\n");
    }
}

/// A figure's caption as Typst content: empty when there are no words.
fn caption_of(alt: &str) -> String {
    let alt = alt.trim();
    if alt.is_empty() {
        "[]".into()
    } else {
        format!("[#{}]", literal(alt))
    }
}

/// Whether a paragraph holds one picture and nothing else but space, or a
/// link around one.
fn lone_picture(p: &Handle) -> bool {
    let children = p.children.borrow();
    let mut shown = children.iter().filter(|it| match &it.data {
        NodeData::Text { contents } => !contents.borrow().trim().is_empty(),
        NodeData::Element { .. } => true,
        _ => false,
    });
    let (Some(only), None) = (shown.next(), shown.next()) else {
        return false;
    };
    match tag(only).as_deref() {
        Some("img") => true,
        Some("a") => {
            let inner = only.children.borrow();
            inner.len() == 1 && tag(&inner[0]).as_deref() == Some("img")
        }
        _ => false,
    }
}

const BLOCKS: &[&str] = &[
    "p",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "ul",
    "ol",
    "pre",
    "blockquote",
    "table",
    "hr",
    "div",
    "section",
    "details",
];

impl Walker<'_> {
    fn children(&mut self, node: &Handle) {
        for child in node.children.borrow().iter() {
            self.node(child);
        }
    }

    /// A paragraph holding a picture alone: a figure, its alt text its
    /// caption when the picture itself shows.
    fn alone(&mut self, node: &Handle) {
        self.caption = None;
        self.wrap("#alone[", node, "]");
        let caption = self.caption.take().unwrap_or_default();
        let _ = writeln!(self.out, "{}", caption_of(&caption));
    }

    fn wrap(&mut self, open: &str, node: &Handle, close: &str) {
        self.out.push_str(open);
        self.children(node);
        self.out.push_str(close);
    }

    /// Wraps a piece of the file that's selected whole, marked as one.
    fn wrap_unit(&mut self, open: &str, node: &Handle, close: &str) {
        self.out.push_str(open);
        if let Some(lines) = written_on(node) {
            unit(self.out, lines);
        }
        self.children(node);
        self.out.push_str(close);
    }

    fn node(&mut self, node: &Handle) {
        match &node.data {
            NodeData::Document => self.children(node),
            NodeData::Text { contents } => {
                let mut text = collapse(&contents.borrow());
                // The line breaks between blocks are no words: set, a space
                // there would be a paragraph of its own, a blank line and a
                // second gap between the blocks either side of it. A quote,
                // an alert or a section holds only blocks; a list's item may
                // hold words, whose spaces stay.
                let between_blocks = text == " "
                    && parent_tag(node).is_some_and(|parent| {
                        ["body", "blockquote", "div", "section", "details"]
                            .contains(&parent.as_str())
                    });
                if std::mem::take(&mut self.after_box) {
                    text = text.trim_start().to_string();
                }
                if !text.is_empty() && !between_blocks {
                    self.out.push('#');
                    self.out.push_str(&literal(&text));
                }
            }
            NodeData::Element { .. } => self.element(node),
            _ => {}
        }
    }

    fn element(&mut self, node: &Handle) {
        let name = tag(node).unwrap_or_default();
        let top = parent_tag(node).as_deref() == Some("body");
        if top && BLOCKS.contains(&name.as_str()) {
            // Each block says where it starts on the page, so a place on it
            // is kept as a block and a distance into it, which the first
            // set can reach without setting the whole file. It says too the
            // source lines it was written on, for :{n}, and a heading its
            // level and words, for ]], [[ and gO's outline.
            let _ = write!(
                self.out,
                "\n// block\n#metadata({})\n",
                block_said(node, &name, self.blocks)
            );
            self.blocks += 1;
        }
        // The paragraph right after the title is its lede, and a title
        // opening the page sits under the eyebrow.
        let lede = top && self.after_title && name == "p";
        let opening = top && self.blocks == 1 && name == "h1";
        if opening {
            self.titled = true;
        }
        if top && BLOCKS.contains(&name.as_str()) {
            self.after_title = opening;
        }
        match name.as_str() {
            "html" | "body" | "span" | "section" | "picture" | "thead" | "tbody" => {
                if let Some(style) = attr(node, "data-math-style") {
                    self.equation(&text_of(node), style == "display");
                } else {
                    self.children(node);
                }
            }
            "head" | "source" | "p_alert_title" => {}
            // An alert's title, its kind, which the alert says as its
            // eyebrow.
            "p" if attr(node, "class")
                .is_some_and(|class| class.starts_with("markdown-alert-title")) => {}
            "p" if lone_picture(node) => self.alone(node),
            "p" if lede => self.wrap_unit("#lede[", node, "]\n"),
            "p" => self.wrap_unit("#par[", node, "]\n"),
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                let level = &name[1..];
                let end = if opening { "] <title>\n" } else { "]\n" };
                self.wrap_unit(&format!("#heading(level: {level})["), node, end);
            }
            "em" | "i" => self.wrap("#emph[", node, "]"),
            "strong" | "b" => self.wrap("#strong[", node, "]"),
            "del" | "s" => self.wrap("#strike[", node, "]"),
            "sup" => self.wrap("#super[", node, "]"),
            "sub" => self.wrap("#sub[", node, "]"),
            "kbd" => self.wrap("#kbd[", node, "]"),
            "br" => self.out.push_str("#linebreak()"),
            "hr" => self.out.push_str("#rule()\n"),
            "a" => {
                let href = attr(node, "href").unwrap_or_default();
                if href.is_empty() {
                    self.children(node);
                } else {
                    self.wrap(&format!("#link({})[", literal(&href)), node, "]");
                }
            }
            "code" => {
                if let Some(style) = attr(node, "data-math-style") {
                    self.equation(&text_of(node), style == "display");
                } else {
                    let t = text_of(node);
                    let _ = write!(self.out, "#raw({})", literal(&t));
                }
            }
            "pre" => self.pre(node),
            "ul" => self.list("#list(", node),
            "ol" => {
                let start = attr(node, "start").unwrap_or_else(|| "1".into());
                self.list(
                    &format!("#enum(start: {}, ", start.parse::<u32>().unwrap_or(1)),
                    node,
                );
            }
            // A task's box in a loose item, its words in their own paragraph.
            "input" => {
                let checked = attr(node, "checked").is_some();
                let _ = write!(self.out, "#task({checked})[]");
            }
            "blockquote" => self.wrap("#quote(block: true)[", node, "]\n"),
            "div" => {
                let class = attr(node, "class").unwrap_or_default();
                if let Some(kind) = class.strip_prefix("markdown-alert markdown-alert-") {
                    self.wrap(&format!("#alert({})[", literal(kind)), node, "]\n");
                } else if class == "markdown-alert-title"
                    || class.starts_with("markdown-alert-title")
                {
                } else {
                    self.children(node);
                }
            }
            "details" => self.wrap("#details[", node, "]\n"),
            "summary" => self.wrap("#strong[", node, "]#parbreak()"),
            "table" => self.table(node),
            "img" => self.picture(node),
            _ => self.children(node),
        }
    }
}

#[cfg(test)]
mod tests;
