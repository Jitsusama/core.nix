//! Code, equations and diagrams: a block of code highlighted line by line,
//! an equation's LaTeX set in Typst, and a Mermaid diagram drawn once and
//! kept for the next set.

use std::fmt::Write;
use std::time::Instant;

use markup5ever_rcdom::Handle;
use syntect::easy::HighlightLines;

use super::{Walker, attr, literal, text_of, unit, written_on};

impl Walker<'_> {
    pub(super) fn pre(&mut self, node: &Handle) {
        let lang = attr(node, "lang").unwrap_or_default();
        let code = text_of(node);
        let code_math = node
            .children
            .borrow()
            .iter()
            .find_map(|c| attr(c, "data-math-style"))
            .is_some();
        let lines = written_on(node);
        if code_math || lang == "math" {
            if let Some(lines) = lines {
                unit(self.out, lines);
            }
            self.equation(&code, true);
        } else if lang == "mermaid" {
            self.diagram(&code);
        } else {
            // A fenced block's code starts on the line after its fence.
            let first = lines.map(|(first, _)| {
                let opens = self.source.get(first.wrapping_sub(1)).is_some_and(|line| {
                    let line = line.trim_start();
                    line.starts_with("```") || line.starts_with("~~~")
                });
                first + usize::from(opens)
            });
            self.code(&code, &lang, first);
        }
    }

    /// A block of code, each of its lines marked as a piece selected whole,
    /// written on the file's lines from `written`.
    fn code(&mut self, code: &str, lang: &str, written: Option<usize>) {
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
        for (at, line) in syntect::util::LinesWithEndings::from(code).enumerate() {
            // In an empty box, so the mark stands at its line's start: one
            // alone after a line break is set at the end of the line before.
            if let Some(written) = written {
                self.out.push_str("#box[");
                unit(self.out, (written + at, written + at));
                self.out.push(']');
            }
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

    pub(super) fn equation(&mut self, latex: &str, block: bool) {
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
