//! Lists and tables: each item and row marked as a piece selected whole,
//! a task's box, and a table's cells, the words of each column measured.

use std::fmt::Write;

use markup5ever_rcdom::{Handle, NodeData};

use super::{BLOCKS, Walker, attr, tag, text_of, unit, written_on};

impl Walker<'_> {
    pub(super) fn list(&mut self, open: &str, node: &Handle) {
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
                // An item of words is one piece, down to the first block in
                // it, a list of its own or code; an item of paragraphs is
                // its paragraphs, each marked as one.
                let children = child.children.borrow();
                let paragraphs = children.iter().any(|it| tag(it).as_deref() == Some("p"));
                if !paragraphs && let Some((first, last)) = written_on(child) {
                    let inner = children
                        .iter()
                        .filter(|it| tag(it).is_some_and(|name| BLOCKS.contains(&name.as_str())))
                        .find_map(written_on);
                    let last =
                        inner.map_or(last, |(starts, _)| starts.saturating_sub(1).max(first));
                    unit(self.out, (first, last));
                }
                drop(children);
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

    pub(super) fn table(&mut self, node: &Handle) {
        type Row = (bool, Vec<Handle>, Option<(usize, usize)>);
        fn collect(n: &Handle, rows: &mut Vec<Row>) {
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
                        rows.push((head, cells, written_on(c)));
                    }
                    Some(_) => collect(c, rows),
                    None => {}
                }
            }
        }
        let mut rows: Vec<Row> = Vec::new();
        collect(node, &mut rows);
        let columns = rows.iter().map(|r| r.1.len()).max().unwrap_or(1);
        // What the page's sheet needs to lay the table out: each column's
        // longest words, to measure the least it may be, and whether it holds
        // figures alone, to set them right-aligned.
        let mut words: Vec<Vec<(String, bool)>> = vec![Vec::new(); columns];
        let mut figures = vec![true; columns];
        let mut filled = vec![false; columns];
        for (head, cells, _) in &rows {
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
        if rows.peek().is_some_and(|(head, _, _)| *head)
            && let Some((_, cells, lines)) = rows.next()
        {
            self.out.push_str("head: (");
            self.cells(&cells, columns, lines);
            self.out.push_str("), ");
        }
        self.out.push_str("rows: (");
        for (_, cells, lines) in rows {
            self.out.push('(');
            self.cells(&cells, columns, lines);
            self.out.push_str("), ");
        }
        self.out.push_str("))\n");
    }

    /// A row's cells as Typst content, one for every column, the first
    /// marking the row as a piece selected whole.
    fn cells(&mut self, cells: &[Handle], columns: usize, lines: Option<(usize, usize)>) {
        for i in 0..columns {
            self.out.push('[');
            if i == 0
                && let Some(lines) = lines
            {
                unit(self.out, lines);
            }
            if let Some(cell) = cells.get(i) {
                self.children(cell);
            }
            self.out.push_str("], ");
        }
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
pub(super) fn is_figure(text: &str) -> bool {
    text.chars().any(|c| c.is_ascii_digit())
        && text
            .chars()
            .all(|c| c.is_ascii_digit() || " .,%$\u{20ac}\u{a3}+-\u{2212}\u{d7}:/~".contains(c))
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
