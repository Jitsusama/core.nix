//! Markdown to a Typst program: comrak writes GitHub's HTML with each
//! element's place in the source, ammonia keeps what GitHub's sanitizer
//! allows, html5ever parses it once, and one walk writes the program. Every
//! piece of the document's text goes in as a string literal, so nothing in it
//! can be read as markup or code.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write;
use std::time::{Duration, Instant};

use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use syntect::easy::HighlightLines;
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
            "fontFamily": ink.faces.text,
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

/// The page's program, with what each step took. `about` is the eyebrow's
/// words over the title: what the file is and when it changed.
pub fn program(
    markdown: &str,
    about: &str,
    base: Option<&std::path::Path>,
    pieces: &Pieces,
    world: &PageWorld,
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
    out.push_str(crate::page::template(world));
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
    };
    walker.children(&dom.document);
    // The eyebrow goes before any block, so the first set carries it
    // wherever it starts, and sits closer over a title than over words.
    let titled = walker.titled;
    out.insert_str(
        crate::page::template(world).len(),
        &format!("#about(titled: {titled})[#{}]\n", literal(about)),
    );
    costs.walk = t.elapsed();
    out
}

/// Each block of the program on its own, for setting the first screen alone.
pub fn blocks(program: &str, world: &PageWorld) -> (String, Vec<String>) {
    let (head, body) = program.split_at(crate::page::template(world).len());
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

/// A cell's words as they're set, each saying whether it's code.
fn cell_words(node: &Handle, code: bool, out: &mut Vec<(String, bool)>) {
    if let NodeData::Text { contents } = &node.data {
        out.extend(
            contents
                .borrow()
                .split_whitespace()
                .map(|word| (word.to_string(), code)),
        );
    }
    let code = code || tag(node).as_deref() == Some("code");
    for child in node.children.borrow().iter() {
        cell_words(child, code, out);
    }
}

/// Whether a cell holds a figure alone: digits, with what money, time and
/// sizes are written with.
fn is_figure(text: &str) -> bool {
    text.chars().any(|c| c.is_ascii_digit())
        && text
            .chars()
            .all(|c| c.is_ascii_digit() || " .,%$\u{20ac}\u{a3}+-\u{2212}\u{d7}:/~".contains(c))
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

/// Whether a list item is a task its words follow (a tight item), and if
/// so whether it's done.
fn task_of(item: &Handle) -> Option<bool> {
    let first = item
        .children
        .borrow()
        .iter()
        .find(|it| matches!(it.data, NodeData::Element { .. }))
        .cloned()?;
    (tag(&first).as_deref() == Some("input") && attr(&first, "type").as_deref() == Some("checkbox"))
        .then(|| attr(&first, "checked").is_some())
}

/// Whether a list item is a task, tight or with its box in a paragraph.
fn holds_task(item: &Handle) -> bool {
    task_of(item).is_some()
        || item
            .children
            .borrow()
            .iter()
            .find(|it| matches!(it.data, NodeData::Element { .. }))
            .is_some_and(|first| tag(first).as_deref() == Some("p") && task_of(first).is_some())
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
            // set can reach without setting the whole file.
            let _ = write!(self.out, "\n// block\n#metadata({})\n", self.blocks);
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
            "p" if lede => self.wrap("#lede[", node, "]\n"),
            "p" => self.wrap("#par[", node, "]\n"),
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                let level = &name[1..];
                let end = if opening { "] <title>\n" } else { "]\n" };
                self.wrap(&format!("#heading(level: {level})["), node, end);
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

    fn list(&mut self, open: &str, node: &Handle) {
        // A loose list, its items paragraphs, is spaced as paragraphs are,
        // as GitHub spaces one.
        let loose = node.children.borrow().iter().any(|item| {
            tag(item).as_deref() == Some("li")
                && item
                    .children
                    .borrow()
                    .iter()
                    .any(|it| tag(it).as_deref() == Some("p"))
        });
        self.out.push_str(open);
        if loose {
            self.out.push_str("tight: false, ");
        }
        // A task list's boxes hang where its bullets would.
        if node.children.borrow().iter().any(holds_task) {
            self.out.push_str("marker: no-mark, ");
        }
        for child in node.children.borrow().iter() {
            if tag(child).as_deref() == Some("li") {
                self.out.push('[');
                // A task's words go inside it, so a done one's are muted.
                let task = task_of(child);
                if let Some(done) = task {
                    let _ = write!(self.out, "#task({done})[");
                }
                // A loose list's item holds paragraphs, and the newlines
                // between them set an empty first line that the item's
                // mark sat on alone, its text below.
                let blocks = child
                    .children
                    .borrow()
                    .iter()
                    .any(|it| tag(it).is_some_and(|name| BLOCKS.contains(&name.as_str())));
                for it in child.children.borrow().iter() {
                    let blank = matches!(&it.data, NodeData::Text { contents } if contents.borrow().trim().is_empty());
                    let its_box = task.is_some() && tag(it).as_deref() == Some("input");
                    if its_box {
                        // The space after the box isn't the words'.
                        self.after_box = true;
                    } else if !(blocks && blank) {
                        self.node(it);
                    }
                }
                if task.is_some() {
                    self.out.push(']');
                }
                self.out.push_str("], ");
            }
        }
        self.out.push_str(")\n");
    }

    fn table(&mut self, node: &Handle) {
        fn collect(n: &Handle, rows: &mut Vec<(bool, Vec<Handle>)>) {
            for c in n.children.borrow().iter() {
                match tag(c).as_deref() {
                    Some("tr") => {
                        let cells: Vec<Handle> = c
                            .children
                            .borrow()
                            .iter()
                            .filter(|x| matches!(tag(x).as_deref(), Some("td" | "th")))
                            .cloned()
                            .collect();
                        let head = cells
                            .first()
                            .is_some_and(|x| tag(x).as_deref() == Some("th"));
                        rows.push((head, cells));
                    }
                    Some(_) => collect(c, rows),
                    None => {}
                }
            }
        }
        let mut rows: Vec<(bool, Vec<Handle>)> = Vec::new();
        collect(node, &mut rows);
        let columns = rows.iter().map(|r| r.1.len()).max().unwrap_or(1);
        // What the page's sheet needs to lay the table out: each column's
        // longest words, to measure the least it may be, and whether it holds
        // figures alone, to set them right-aligned.
        let mut words: Vec<Vec<(String, bool)>> = vec![Vec::new(); columns];
        let mut figures = vec![true; columns];
        let mut filled = vec![false; columns];
        for (head, cells) in &rows {
            for (i, cell) in cells.iter().enumerate() {
                cell_words(cell, false, &mut words[i]);
                let text = text_of(cell);
                let text = text.trim();
                if !head && !text.is_empty() {
                    filled[i] = true;
                    figures[i] &= is_figure(text);
                }
            }
        }
        let numeric: Vec<&str> = (0..columns)
            .map(|i| {
                if figures[i] && filled[i] {
                    "true"
                } else {
                    "false"
                }
            })
            .collect();
        let _ = write!(
            self.out,
            "#sheet(numeric: ({},), words: (",
            numeric.join(", ")
        );
        for column in &mut words {
            column.sort_by_key(|(word, _)| std::cmp::Reverse(word.chars().count()));
            column.dedup();
            column.truncate(6);
            self.out.push('(');
            for (word, code) in column.iter() {
                let quoted = format!("{word:?}");
                let kind = if *code { "raw" } else { "text" };
                let _ = write!(self.out, "{kind}({quoted}), ");
            }
            self.out.push_str("), ");
        }
        self.out.push_str("), ");
        let mut rows = rows.into_iter().peekable();
        if rows.peek().is_some_and(|(head, _)| *head)
            && let Some((_, cells)) = rows.next()
        {
            self.out.push_str("head: (");
            self.cells(&cells, columns);
            self.out.push_str("), ");
        }
        self.out.push_str("rows: (");
        for (_, cells) in rows {
            self.out.push('(');
            self.cells(&cells, columns);
            self.out.push_str("), ");
        }
        self.out.push_str("))\n");
    }

    /// A row's cells as Typst content, one for every column.
    fn cells(&mut self, cells: &[Handle], columns: usize) {
        for i in 0..columns {
            self.out.push('[');
            if let Some(cell) = cells.get(i) {
                self.children(cell);
            }
            self.out.push_str("], ");
        }
    }

    fn pre(&mut self, node: &Handle) {
        let lang = attr(node, "lang").unwrap_or_default();
        let code = text_of(node);
        let code_math = node
            .children
            .borrow()
            .iter()
            .find_map(|c| attr(c, "data-math-style"))
            .is_some();
        if code_math || lang == "math" {
            self.equation(&code, true);
        } else if lang == "mermaid" {
            self.diagram(&code);
        } else {
            self.code(&code, &lang);
        }
    }

    fn code(&mut self, code: &str, lang: &str) {
        let t = Instant::now();
        let syntax = self
            .pieces
            .syntaxes
            .find_syntax_by_token(lang)
            .unwrap_or_else(|| self.pieces.syntaxes.find_syntax_plain_text());
        let mut h = HighlightLines::new(syntax, &self.pieces.theme);
        // The template places the language and widens the block by the
        // mono's width, so it's told its lines' lengths in characters.
        let columns = |line: &str| -> usize {
            line.trim_end()
                .chars()
                .map(|c| if c == '\t' { 4 } else { 1 })
                .sum()
        };
        let first = code.lines().next().map_or(0, columns);
        let longest = code.lines().map(columns).max().unwrap_or(0);
        let _ = write!(
            self.out,
            "#codeblock(lang: {}, first: {first}, longest: {longest})[",
            literal(lang)
        );
        for line in syntect::util::LinesWithEndings::from(code) {
            match h.highlight_line(line, &self.pieces.syntaxes) {
                Ok(runs) => {
                    for (style, text) in runs {
                        let text = text.trim_end_matches('\n');
                        if text.is_empty() {
                            continue;
                        }
                        let c = style.foreground;
                        let _ = write!(
                            self.out,
                            "#text(fill: paint({}, {}, {}), {})",
                            c.r,
                            c.g,
                            c.b,
                            literal(text)
                        );
                    }
                }
                Err(_) => {
                    let _ = write!(self.out, "#{}", literal(line.trim_end()));
                }
            }
            if line.ends_with('\n') {
                self.out.push_str("#linebreak()");
            }
        }
        self.out.push_str("]\n");
        self.costs.code += t.elapsed();
    }

    fn equation(&mut self, latex: &str, block: bool) {
        let t = Instant::now();
        let converted = self
            .pieces
            .converted
            .borrow_mut()
            .entry(latex.to_string())
            .or_insert_with(|| mitex::convert_math(latex, None).ok())
            .clone();
        if let Some(typst_math) = converted.filter(|_| self.world.sets_equations()) {
            let _ = write!(self.out, "#mx({}, {block})", literal(&typst_math));
            self.costs.equations.push((t.elapsed(), true));
        } else {
            let _ = write!(self.out, "#broken({})", literal(latex));
            self.costs.equations.push((t.elapsed(), false));
        }
    }

    /// A picture: a file beside the page is put in the world and set at its
    /// own size, as GitHub shows it, no wider than the measure. One that
    /// can't be read, or is on the web, stands as a box its size saying
    /// what it shows.
    fn picture(&mut self, node: &Handle) {
        let alt = attr(node, "alt").unwrap_or_default();
        let src = attr(node, "src").unwrap_or_default();
        let given_w =
            attr(node, "width").and_then(|w| w.trim_end_matches("px").parse::<f64>().ok());
        let given_h =
            attr(node, "height").and_then(|h| h.trim_end_matches("px").parse::<f64>().ok());
        if let Some((path, w, h)) = self.local_picture(&src) {
            // CSS pixels are 0.75 pt; a width the page gives wins, and a
            // height alone scales the width with it.
            let width = match (given_w, given_h, w, h) {
                (Some(gw), _, _, _) => Some(gw),
                (None, Some(gh), Some(w), Some(h)) if h > 0.0 => Some(gh * w / h),
                (None, _, w, _) => w,
            };
            let width = width.map_or("none".to_string(), |w| format!("{:.1}pt", w * 0.75));
            self.caption = Some(alt.clone());
            self.set_picture(
                node,
                &format!("#pic({}, {}, {})", literal(&path), literal(&alt), width),
            );
            return;
        }
        let w = given_w.unwrap_or(320.0);
        let h = given_h.unwrap_or(w * 0.5);
        self.caption = None;
        self.set_picture(
            node,
            &format!(
                "#picture({}, {:.1}pt, {:.1}pt)",
                literal(&alt),
                w * 0.75,
                h * 0.75
            ),
        );
    }

    /// A picture in a paragraph stays in its line; one standing among the
    /// blocks, as an HTML picture does, is a paragraph of its own, so it
    /// keeps the page's edge and the space between paragraphs.
    fn set_picture(&mut self, node: &Handle, picture: &str) {
        let parent = node.parent.take();
        let among_blocks = parent
            .as_ref()
            .and_then(std::rc::Weak::upgrade)
            .and_then(|parent| tag(&parent))
            .is_none_or(|name| {
                ["body", "div", "section", "details", "blockquote"].contains(&name.as_str())
            });
        node.parent.set(parent);
        if among_blocks {
            let caption = self.caption.take().unwrap_or_default();
            let _ = write!(self.out, "\n#alone[{picture}]{}\n", caption_of(&caption));
        } else {
            self.out.push_str(picture);
        }
    }

    /// Reads a picture beside the page into the world: its path there, named
    /// for its kind as Typst reads it, and its size in pixels where its
    /// header gives one. None for a picture on the web, a missing file, or one
    /// whose kind Typst can't set.
    fn local_picture(&mut self, src: &str) -> Option<(String, Option<f64>, Option<f64>)> {
        // A picture on the web is set once fetched; until then it's a box,
        // and it's asked for.
        if crate::web::is_web(src) {
            let bytes = crate::web::cached(src);
            if bytes.is_none() {
                crate::web::want(src);
            }
            return self.put_picture(bytes?);
        }
        let src = src.strip_prefix("file://").unwrap_or(src);
        if src.is_empty() || src.contains("://") || src.starts_with("data:") {
            return None;
        }
        let src = src.split(['?', '#']).next().unwrap_or(src);
        let path = std::path::Path::new(src);
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.base?.join(path)
        };
        self.put_picture(std::fs::read(&path).ok()?)
    }

    /// Puts a picture's bytes into the world: its path there, named for its
    /// kind as Typst reads it, and its size in pixels where its header
    /// gives one.
    #[expect(
        clippy::cast_precision_loss,
        reason = "a picture's size in pixels, far below a mantissa's 52 bits"
    )]
    fn put_picture(&mut self, bytes: Vec<u8>) -> Option<(String, Option<f64>, Option<f64>)> {
        let ext = picture_kind(&bytes)?;
        let (w, h) = if ext == "svg" || ext == "pdf" {
            (None, None)
        } else {
            let size = imagesize::blob_size(&bytes).ok()?;
            (Some(size.width as f64), Some(size.height as f64))
        };
        let name = format!("/pictures/{}.{ext}", self.pictures);
        self.pictures += 1;
        self.world.put_file(&name, bytes);
        Some((name, w, h))
    }

    fn diagram(&mut self, source: &str) {
        let t = Instant::now();
        let nodes = source.lines().filter(|l| !l.trim().is_empty()).count();
        let made = self.pieces.made.borrow().get(source).cloned();
        let made = if let Some(m) = made {
            m
        } else {
            let output = self.pieces.mermaid.render(merman::RenderRequest::svg(
                source,
                merman::OperationControl::new(),
                merman::SvgRequest::default(),
            ));
            let path = match output {
                Ok(merman::RenderOutput::Svg(Some(svg))) => {
                    let n = self.pieces.made.borrow().len();
                    let path = format!("/diagrams/{n}.svg");
                    self.world.put_file(&path, svg.svg().as_bytes().to_vec());
                    Some(path)
                }
                _ => None,
            };
            self.pieces
                .made
                .borrow_mut()
                .insert(source.to_string(), path.clone());
            path
        };
        let ok = made.is_some();
        match made {
            Some(path) => {
                let _ = writeln!(self.out, "#diagram({})", literal(&path));
            }
            None => {
                let _ = writeln!(self.out, "#broken({})", literal(source));
            }
        }
        self.costs.diagrams.push((nodes, t.elapsed(), ok));
    }
}

#[cfg(test)]
mod tests {
    use super::{Costs, Pieces, is_figure, picture_kind, program};
    use crate::world::PageWorld;

    #[test]
    fn without_mitex_an_equation_shows_as_written_and_the_page_still_sets() {
        let world = PageWorld::new(&[], std::path::Path::new("/nowhere"));
        let mut costs = Costs::default();
        let typst = program(
            "Before $x^2$ after.\n",
            "page",
            None,
            &Pieces::new(),
            &world,
            &mut costs,
        );
        assert!(typst.contains("#broken(\"x^2\")"), "{typst}");
        assert!(!typst.contains("#mx("), "{typst}");
    }

    #[test]
    fn a_title_opening_the_page_has_the_eyebrow_over_it_and_its_lede_after() {
        let world = PageWorld::new(&[], std::path::Path::new("/nowhere"));
        let walk = |markdown: &str| {
            program(
                markdown,
                "notes",
                None,
                &Pieces::new(),
                &world,
                &mut Costs::default(),
            )
        };
        let titled = walk("# Title\n\nThe lede.\n\nThe words.\n");
        assert!(
            titled.contains("#about(titled: true)[#\"notes\"]"),
            "{titled}"
        );
        assert!(titled.contains("] <title>"), "{titled}");
        assert!(titled.contains("#lede[#\"The lede.\"]"), "{titled}");
        assert!(titled.contains("#par[#\"The words.\"]"), "{titled}");
        let untitled = walk("The words.\n\n# Later\n\nMore.\n");
        assert!(untitled.contains("#about(titled: false)"), "{untitled}");
        assert!(
            !untitled.contains("] <title>"),
            "a later title isn't the page's: {untitled}"
        );
        assert!(
            untitled.contains("#par[#\"More.\"]"),
            "only the page's title has a lede: {untitled}"
        );
    }

    #[test]
    fn a_task_list_hangs_its_boxes_and_holds_each_tasks_words() {
        let world = PageWorld::new(&[], std::path::Path::new("/nowhere"));
        let typst = program(
            "- [x] Done it\n- [ ] Still to do\n",
            "notes",
            None,
            &Pieces::new(),
            &world,
            &mut Costs::default(),
        );
        assert!(typst.contains("#list(marker: no-mark, "), "{typst}");
        assert!(typst.contains("#task(true)[#\"Done it\"]"), "{typst}");
        assert!(typst.contains("#task(false)[#\"Still to do\"]"), "{typst}");
    }

    #[test]
    fn a_quote_and_an_alert_hold_only_their_words() {
        let world = PageWorld::new(&[], std::path::Path::new("/nowhere"));
        let typst = program(
            "> Quoted words.\n\n> [!NOTE]\n> Noted words.\n",
            "notes",
            None,
            &Pieces::new(),
            &world,
            &mut Costs::default(),
        );
        assert!(
            typst.contains("#quote(block: true)[#par[#\"Quoted words.\"]"),
            "{typst}"
        );
        assert!(
            typst.contains("#alert(\"note\")[#par[#\"Noted words.\"]"),
            "the alert's kind is its eyebrow, not a paragraph too: {typst}"
        );
        assert!(
            !typst.contains("#\" \""),
            "a blank line between blocks: {typst}"
        );
    }

    #[test]
    fn a_picture_alone_is_a_figure_and_one_among_words_stays_in_its_line() {
        let world = PageWorld::new(&[], std::path::Path::new("/nowhere"));
        let walk = |markdown: &str| {
            program(
                markdown,
                "notes",
                None,
                &Pieces::new(),
                &world,
                &mut Costs::default(),
            )
        };
        let alone = walk("![What it shows](missing.png)\n");
        assert!(
            alone.contains("#alone[#picture(\"What it shows\""),
            "{alone}"
        );
        assert!(
            alone.contains("pt)][]"),
            "a box says its words, so it has no caption: {alone}"
        );
        let among = walk("Words ![What it shows](missing.png) and more.\n");
        assert!(!among.contains("#alone"), "{among}");
        assert!(among.contains("#par["), "{among}");
    }

    #[test]
    fn a_picture_is_known_by_its_first_bytes_not_its_name() {
        assert_eq!(picture_kind(b"\x89PNG\r\n\x1a\n...."), Some("png"));
        assert_eq!(picture_kind(b"\xff\xd8\xff\xe0...."), Some("jpg"));
        assert_eq!(picture_kind(b"GIF89a...."), Some("gif"));
        assert_eq!(picture_kind(b"RIFF\0\0\0\0WEBPVP8 "), Some("webp"));
        assert_eq!(picture_kind(b"%PDF-1.7\n"), Some("pdf"));
        assert_eq!(
            picture_kind(b"<?xml version=\"1.0\"?>\n<svg xmlns="),
            Some("svg")
        );
        assert_eq!(picture_kind(b"not a picture"), None);
        assert_eq!(picture_kind(b""), None);
    }

    #[test]
    fn only_a_cell_of_figures_is_a_figure() {
        for figure in ["42", "3.5", "$1,200", "-4%", "12:30", "2 \u{d7} 3"] {
            assert!(is_figure(figure), "{figure} is a figure");
        }
        for words in ["none", "v2", "", "-", "12 ms"] {
            assert!(!is_figure(words), "{words} isn't a figure");
        }
    }
}
