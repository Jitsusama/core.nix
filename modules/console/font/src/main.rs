//! Draws a console font from an OpenType face: each glyph a list names is
//! rasterized into one cell and written as a PSF 2 font, with the table that
//! tells the kernel which characters each glyph stands for.
//!
//!     console-font <face> <glyphs> <out> [--cell 16x32] [--cut 100]
//!
//! The face is sized so its advance fills the cell's width, and its line,
//! ascender to descender, is centred in the cell's height. Each pixel is the
//! face's coverage, antialiased and then cut at `--cut` out of 255, since a
//! console draws in one colour: a face with even stems keeps them at the
//! cut where a rasterizer's own one-bit hinting thins some and not others.
//!
//! Box drawing is the face's own, then carried to the cell's edges where a
//! stroke stops short of them, so lines join from cell to cell. Block
//! elements are drawn exactly rather than from the face, since a half
//! block has to be half the cell.
//!
//! The list holds at most 256 glyphs. The kernel takes a ninth bit for a
//! font's 512 glyphs from the colour attribute, which costs the console its
//! bright foreground colours.
//!
//! The font always has 256 slots, and a glyph for a character below U+0100
//! sits in that character's slot, with the rest filling the free ones in
//! order. The table says what each glyph stands for, but the console clears
//! the screen with slot 32 whatever it says, so the space has to be there,
//! and kbd's setfont refuses a font whose slot 32 isn't blank.

use std::collections::HashSet;
use std::process::ExitCode;

/// The most glyphs a font may hold and keep the console's sixteen colours.
const MOST: usize = 256;
/// How far short of an edge a box-drawing stroke may stop and still be
/// carried to it.
const REACH: usize = 3;

/// One glyph: what it's drawn as, and every character it stands for.
#[derive(Debug, PartialEq)]
struct Glyph {
    drawn: char,
    stands_for: Vec<char>,
}

/// A cell's size in pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Cell {
    width: usize,
    height: usize,
}

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("console-font: {why}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let mut paths = Vec::new();
    let mut cell = Cell {
        width: 16,
        height: 32,
    };
    let mut cut = 100u8;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--cell" => cell = parse_cell(&args.next().ok_or("--cell needs a size")?)?,
            "--cut" => {
                cut = args
                    .next()
                    .ok_or("--cut needs a number")?
                    .parse()
                    .map_err(|_| "--cut is a number from 0 to 255")?;
            }
            _ => paths.push(arg),
        }
    }
    let [face, list, out] = paths.as_slice() else {
        return Err("usage: console-font <face> <glyphs> <out> [--cell 16x32] [--cut 100]".into());
    };
    let bytes = std::fs::read(face).map_err(|e| format!("can't read {face}: {e}"))?;
    let font = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default())
        .map_err(|e| format!("can't read {face} as a face: {e}"))?;
    let text = std::fs::read_to_string(list).map_err(|e| format!("can't read {list}: {e}"))?;
    let glyphs = parse_glyphs(&text)?;
    let slots = layout(&glyphs)?;
    let drawn = draw_all(&font, &glyphs, cell, cut)?;
    std::fs::write(out, psf(&glyphs, &drawn, &slots, cell))
        .map_err(|e| format!("can't write {out}: {e}"))
}

fn parse_cell(text: &str) -> Result<Cell, String> {
    let bad = || format!("a cell is a width and height such as 16x32, not {text}");
    let (width, height) = text.split_once('x').ok_or_else(bad)?;
    let cell = Cell {
        width: width.parse().map_err(|_| bad())?,
        height: height.parse().map_err(|_| bad())?,
    };
    if cell.width == 0 || cell.width > 32 || cell.height == 0 || cell.height > 64 {
        return Err(format!("the console draws cells up to 32x64, not {text}"));
    }
    Ok(cell)
}

/// Reads the list: one glyph per word, written as itself (`─`), as `U+2500`,
/// or as a range `U+0020..U+007E`, one glyph each. A word joined by `=`
/// draws its first and stands for the rest too (`-=U+2010`). A line
/// starting `# ` is a comment.
fn parse_glyphs(text: &str) -> Result<Vec<Glyph>, String> {
    let mut glyphs = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("# ") {
            continue;
        }
        for word in line.split_whitespace() {
            if let Some((from, to)) = word.split_once("..") {
                let (from, to) = (character(from)?, character(to)?);
                if from > to {
                    return Err(format!("{word} runs backwards"));
                }
                glyphs.extend((from..=to).map(|drawn| Glyph {
                    drawn,
                    stands_for: vec![drawn],
                }));
            } else if word.chars().count() > 1 && word.contains('=') {
                let stands_for = word
                    .split('=')
                    .map(character)
                    .collect::<Result<Vec<_>, _>>()?;
                glyphs.push(Glyph {
                    drawn: stands_for[0],
                    stands_for,
                });
            } else {
                let drawn = character(word)?;
                glyphs.push(Glyph {
                    drawn,
                    stands_for: vec![drawn],
                });
            }
        }
    }
    let mut seen = HashSet::new();
    for c in glyphs.iter().flat_map(|glyph| &glyph.stands_for) {
        if !seen.insert(*c) {
            return Err(format!("U+{:04X} is listed twice", u32::from(*c)));
        }
    }
    if glyphs.len() > MOST {
        return Err(format!(
            "{} glyphs is more than {MOST}, past which the console loses its bright colours",
            glyphs.len()
        ));
    }
    Ok(glyphs)
}

/// Which glyph each of the font's slots holds: a glyph standing for a
/// character below U+0100 in that character's slot, the rest in the free
/// slots, lowest first.
fn layout(glyphs: &[Glyph]) -> Result<Vec<Option<usize>>, String> {
    let own = |glyph: &Glyph| {
        glyph
            .stands_for
            .iter()
            .map(|c| u32::from(*c) as usize)
            .find(|code| *code < MOST)
    };
    let mut slots = vec![None; MOST];
    for (i, glyph) in glyphs.iter().enumerate() {
        if let Some(code) = own(glyph) {
            slots[code] = Some(i);
        }
    }
    if slots[0x20].is_none() {
        return Err("the list needs U+0020, the space the console clears the screen with".into());
    }
    let mut free = (0..MOST)
        .filter(|slot| slots[*slot].is_none())
        .collect::<Vec<_>>();
    free.reverse();
    for (i, glyph) in glyphs.iter().enumerate() {
        if own(glyph).is_none() {
            let slot = free
                .pop()
                .ok_or("the list has more glyphs than the font has slots")?;
            slots[slot] = Some(i);
        }
    }
    Ok(slots)
}

fn character(word: &str) -> Result<char, String> {
    if let Some(hex) = word.strip_prefix("U+") {
        let code =
            u32::from_str_radix(hex, 16).map_err(|_| format!("{word} isn't a code point"))?;
        return char::from_u32(code).ok_or_else(|| format!("{word} isn't a character"));
    }
    let mut chars = word.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Ok(c),
        _ => Err(format!(
            "{word} is neither one character nor U+ and its code"
        )),
    }
}

fn draw_all(
    font: &fontdue::Font,
    glyphs: &[Glyph],
    cell: Cell,
    cut: u8,
) -> Result<Vec<Vec<bool>>, String> {
    let advance = font.metrics('0', 1000.0).advance_width / 1000.0;
    #[expect(
        clippy::cast_precision_loss,
        reason = "a cell is a few dozen pixels, which a float holds exactly"
    )]
    let px = cell.width as f32 / advance;
    let line = font
        .horizontal_line_metrics(px)
        .ok_or("the face has no line metrics")?;
    #[expect(
        clippy::cast_precision_loss,
        reason = "a cell is a few dozen pixels, which a float holds exactly"
    )]
    let baseline =
        ((cell.height as f32 - (line.ascent - line.descent)) / 2.0 + line.ascent).round();
    glyphs
        .iter()
        .map(|glyph| {
            if let Some(block) = block(glyph.drawn, cell) {
                return Ok(block);
            }
            if font.lookup_glyph_index(glyph.drawn) == 0 {
                return Err(format!(
                    "the face has no glyph for U+{:04X}",
                    u32::from(glyph.drawn)
                ));
            }
            let mut pixels = raster(font, glyph.drawn, px, baseline, cell, cut);
            if ('\u{2500}'..='\u{257f}').contains(&glyph.drawn) {
                carry_to_edges(&mut pixels, cell);
            }
            Ok(pixels)
        })
        .collect()
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "a glyph and its cell are a few dozen pixels, and only a point inside the cell \
              becomes an index"
)]
fn raster(font: &fontdue::Font, c: char, px: f32, baseline: f32, cell: Cell, cut: u8) -> Vec<bool> {
    let (metrics, coverage) = font.rasterize(c, px);
    let mut pixels = vec![false; cell.width * cell.height];
    let top = baseline as i32 - metrics.ymin - metrics.height as i32;
    for y in 0..metrics.height {
        for x in 0..metrics.width {
            let (cx, cy) = (metrics.xmin + x as i32, top + y as i32);
            let inside =
                (0..cell.width as i32).contains(&cx) && (0..cell.height as i32).contains(&cy);
            if inside && coverage[y * metrics.width + x] >= cut {
                pixels[cy as usize * cell.width + cx as usize] = true;
            }
        }
    }
    pixels
}

/// Carries each stroke that stops within `REACH` of an edge out to it.
fn carry_to_edges(pixels: &mut [bool], cell: Cell) {
    let at = |x: usize, y: usize| y * cell.width + x;
    for x in 0..cell.width {
        if let Some(top) = (0..cell.height)
            .find(|&y| pixels[at(x, y)])
            .filter(|&y| y <= REACH)
        {
            (0..top).for_each(|y| pixels[at(x, y)] = true);
        }
        let bottom = (0..cell.height).rev().find(|&y| pixels[at(x, y)]);
        if let Some(bottom) = bottom.filter(|&y| y + REACH >= cell.height - 1) {
            (bottom..cell.height).for_each(|y| pixels[at(x, y)] = true);
        }
    }
    for y in 0..cell.height {
        if let Some(left) = (0..cell.width)
            .find(|&x| pixels[at(x, y)])
            .filter(|&x| x <= REACH)
        {
            (0..left).for_each(|x| pixels[at(x, y)] = true);
        }
        let right = (0..cell.width).rev().find(|&x| pixels[at(x, y)]);
        if let Some(right) = right.filter(|&x| x + REACH >= cell.width - 1) {
            (right..cell.width).for_each(|x| pixels[at(x, y)] = true);
        }
    }
}

/// The block elements, drawn to the cell: the full and half blocks, and the
/// three shades at a quarter, a half and three quarters.
fn block(c: char, cell: Cell) -> Option<Vec<bool>> {
    if !matches!(c, '█' | '▀' | '▄' | '▌' | '▐' | '░' | '▒' | '▓') {
        return None;
    }
    let (width, height) = (cell.width, cell.height);
    let light = |x: usize, y: usize| {
        (x.is_multiple_of(2) && y.is_multiple_of(4)) || (!x.is_multiple_of(2) && y % 4 == 2)
    };
    let on = |x: usize, y: usize| match c {
        '█' => true,
        '▀' => y < height / 2,
        '▄' => y >= height / 2,
        '▌' => x < width / 2,
        '▐' => x >= width / 2,
        '░' => light(x, y),
        '▒' => (x + y).is_multiple_of(2),
        _ => !light(x, y),
    };
    Some(
        (0..height)
            .flat_map(|y| (0..width).map(move |x| on(x, y)))
            .collect(),
    )
}

/// A PSF 2 font: its header, each slot's rows a byte-aligned bit string,
/// blank where it holds no glyph, and the table naming what each slot's
/// glyph stands for in UTF-8, ended by 0xFF.
fn psf(glyphs: &[Glyph], drawn: &[Vec<bool>], slots: &[Option<usize>], cell: Cell) -> Vec<u8> {
    let stride = cell.width.div_ceil(8);
    let size = stride * cell.height;
    let mut out = Vec::new();
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the header's counts and sizes are a few hundred at most"
    )]
    for word in [
        0x864a_b572,
        0,
        32,
        1,
        slots.len() as u32,
        size as u32,
        cell.height as u32,
        cell.width as u32,
    ] {
        out.extend_from_slice(&u32::to_le_bytes(word));
    }
    let blank = vec![false; cell.width * cell.height];
    for slot in slots {
        let pixels = slot.map_or(&blank, |i| &drawn[i]);
        for row in pixels.chunks(cell.width) {
            let mut bytes = vec![0u8; stride];
            for (x, _) in row.iter().enumerate().filter(|(_, on)| **on) {
                bytes[x / 8] |= 0x80 >> (x % 8);
            }
            out.extend(bytes);
        }
    }
    for glyph in slots.iter().map(|slot| slot.map(|i| &glyphs[i])) {
        let mut buffer = [0u8; 4];
        for c in glyph.iter().flat_map(|glyph| &glyph.stands_for) {
            out.extend_from_slice(c.encode_utf8(&mut buffer).as_bytes());
        }
        out.push(0xff);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const CELL: Cell = Cell {
        width: 16,
        height: 32,
    };

    #[test]
    fn a_range_is_a_glyph_for_each_of_its_characters() {
        let glyphs = parse_glyphs("U+0041..U+0043").unwrap();
        let drawn: String = glyphs.iter().map(|glyph| glyph.drawn).collect();
        assert_eq!(drawn, "ABC");
    }

    #[test]
    fn a_glyph_stands_for_what_it_is_joined_to() {
        let glyphs = parse_glyphs("-=U+2010=U+2212").unwrap();
        assert_eq!(
            glyphs,
            [Glyph {
                drawn: '-',
                stands_for: vec!['-', '\u{2010}', '\u{2212}']
            }]
        );
    }

    #[test]
    fn characters_are_written_as_themselves_or_by_code() {
        let glyphs = parse_glyphs("─ U+2502 =").unwrap();
        let drawn: String = glyphs.iter().map(|glyph| glyph.drawn).collect();
        assert_eq!(drawn, "─│=");
    }

    #[test]
    fn comments_and_blank_lines_are_skipped() {
        assert_eq!(parse_glyphs("# the lines\n\n  ─\n").unwrap().len(), 1);
    }

    #[test]
    fn a_character_listed_twice_is_refused() {
        let why = parse_glyphs("U+0041..U+0042 B").unwrap_err();
        assert!(why.contains("U+0042 is listed twice"), "{why}");
    }

    #[test]
    fn more_than_256_glyphs_are_refused_for_the_colours() {
        let why = parse_glyphs("U+0100..U+0200").unwrap_err();
        assert!(why.contains("bright colours"), "{why}");
        assert!(parse_glyphs("U+0100..U+01FF").is_ok());
    }

    #[test]
    fn a_word_that_is_neither_is_refused() {
        assert!(parse_glyphs("ab").unwrap_err().contains("neither"));
        assert!(
            parse_glyphs("U+0042..U+0041")
                .unwrap_err()
                .contains("backwards")
        );
    }

    #[test]
    fn a_cell_is_read_and_held_to_what_the_console_draws() {
        assert_eq!(
            parse_cell("12x24").unwrap(),
            Cell {
                width: 12,
                height: 24
            }
        );
        assert!(parse_cell("64x64").is_err());
        assert!(parse_cell("16").is_err());
    }

    #[test]
    fn a_half_block_is_half_the_cell() {
        let top = block('▀', CELL).unwrap();
        assert!(top[..16 * 16].iter().all(|on| *on));
        assert!(top[16 * 16..].iter().all(|on| !on));
        let shade = block('▒', CELL).unwrap();
        assert_eq!(shade.iter().filter(|on| **on).count(), 16 * 32 / 2);
        assert!(block('A', CELL).is_none());
    }

    #[test]
    fn a_stroke_short_of_an_edge_is_carried_to_it() {
        let mut pixels = vec![false; 16 * 32];
        // A vertical stroke in columns 7 and 8 from row 2 to row 30, and a
        // dot in the middle well away from every edge.
        for y in 2..=30 {
            pixels[y * 16 + 7] = true;
            pixels[y * 16 + 8] = true;
        }
        pixels[16 * 16 + 10] = true;
        carry_to_edges(&mut pixels, CELL);
        assert!((0..32).all(|y| pixels[y * 16 + 7] && pixels[y * 16 + 8]));
        assert!(
            !pixels[16 * 16 + 15],
            "a dot five columns from the edge stays where it is"
        );
    }

    #[test]
    fn a_glyph_below_u_0100_sits_in_its_own_slot_and_the_rest_fill_the_free_ones() {
        let glyphs = parse_glyphs("─ A U+2010=- U+0020 █").unwrap();
        let slots = layout(&glyphs).unwrap();
        assert_eq!(slots.len(), 256);
        assert_eq!(
            [
                slots[0],
                slots[1],
                slots[2],
                slots[0x20],
                slots[0x2d],
                slots[0x41]
            ],
            [Some(0), Some(4), None, Some(3), Some(2), Some(1)]
        );
    }

    #[test]
    fn a_list_without_the_space_is_refused() {
        let why = layout(&parse_glyphs("A").unwrap()).unwrap_err();
        assert!(why.contains("U+0020"), "{why}");
    }

    #[test]
    fn the_font_is_a_psf_naming_what_each_glyph_stands_for() {
        let glyphs = parse_glyphs("-=U+2010 █ U+0020").unwrap();
        let drawn = vec![
            vec![false; 16 * 32],
            block('█', CELL).unwrap(),
            vec![false; 16 * 32],
        ];
        let out = psf(&glyphs, &drawn, &layout(&glyphs).unwrap(), CELL);
        let word = |i: usize| u32::from_le_bytes(out[i * 4..i * 4 + 4].try_into().unwrap());
        assert_eq!(
            [
                word(0),
                word(2),
                word(3),
                word(4),
                word(5),
                word(6),
                word(7)
            ],
            [0x864a_b572, 32, 1, 256, 64, 32, 16]
        );
        let bitmaps = &out[32..32 + 256 * 64];
        assert!(
            bitmaps[..64].iter().all(|byte| *byte == 0xff),
            "█ in slot 0"
        );
        assert!(bitmaps[64..].iter().all(|byte| *byte == 0));
        let table: Vec<&[u8]> = out[32 + 256 * 64..].split(|byte| *byte == 0xff).collect();
        assert_eq!(table.len(), 257, "a 0xFF ends each slot's entry");
        assert_eq!(
            [table[0], table[1], table[0x20], table[0x2d]],
            ["█".as_bytes(), b"", b" ", "-\u{2010}".as_bytes()]
        );
    }
}
