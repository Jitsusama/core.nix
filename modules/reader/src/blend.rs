//! How a mark recolours the page under it: in one blend where that
//! reaches the colours asked for, else in a recolour's passes, which Gl
//! draws (gl.rs).

/// How a mark recolours the page under it, as Neovim's Search colours a
/// match: the page's ground becomes the mark's ground and its text the
/// mark's text, with what lies between, a glyph's soft edge, in proportion.
/// Each channel is a line through those two points, `source + constant *
/// under`, or, for a mark darker under light text, `source * (1 - under) +
/// constant * under`, which GL's blend draws in one pass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Blend {
    pub source: [f32; 3],
    pub constant: [f32; 3],
    pub flipped: bool,
}

impl Blend {
    pub fn between(
        page_ground: [f32; 3],
        page_ink: [f32; 3],
        ground: [f32; 3],
        ink: [f32; 3],
    ) -> Blend {
        let slope: [f32; 3] =
            std::array::from_fn(|c| (ink[c] - ground[c]) / (page_ink[c] - page_ground[c]));
        let flipped = slope.iter().any(|s| *s < 0.0);
        let source: [f32; 3] = std::array::from_fn(|c| ground[c] - slope[c] * page_ground[c]);
        let constant: [f32; 3] = std::array::from_fn(|c| {
            if flipped {
                source[c] + slope[c]
            } else {
                slope[c]
            }
        });
        Blend {
            source: source.map(|v| v.clamp(0.0, 1.0)),
            constant: constant.map(|v| v.clamp(0.0, 1.0)),
            flipped,
        }
    }

    /// What the blend makes of a colour under it.
    #[cfg(test)]
    pub fn over(&self, under: [f32; 3]) -> [f32; 3] {
        std::array::from_fn(|c| {
            let kept = if self.flipped { 1.0 - under[c] } else { 1.0 };
            (self.source[c] * kept + self.constant[c] * under[c]).clamp(0.0, 1.0)
        })
    }
}

/// The least a colour's channel may part from the ground under a mark for
/// the mark to read a glyph's cover from it: nearer, a step of the screen's
/// 256 would be a tenth of the way.
const LEAST_SPREAD: f32 = 0.05;
/// How far towards the ink under a mark a pixel is covered whole: a
/// title's strong lies a little short of the page's text in a channel, as
/// the theme's blue does, and still takes the mark's ink exactly.
const WHOLE_COVER: f32 = 0.9;
/// A quarter of a screen's step, which the last pass lands each pixel
/// above where it's asked to: a driver drawing in software cuts a colour
/// down to its step where a GPU rounds it, and either then lands on it.
const QUARTER_STEP: f32 = 0.25 / 255.0;

/// How a selection or the cursor recolours what lies under it exactly,
/// where one blend clamps short of the colours asked for: each pixel is
/// read as how far it lies from the ground under the mark towards the ink
/// there, a glyph's cover of it, and set as far from the mark's ground
/// towards its ink. Ink beyond the one under it, a title's strong, is
/// covered whole, so the cursor turns any glyph its own colour, as
/// kitty's does. A pass takes away the ground under it, passes scale
/// what's left to the cover, and a last draws the mark's colours from it.
#[derive(Clone, Debug, PartialEq)]
pub struct Recolour {
    pub under: [f32; 3],
    /// Whether the ink under the mark is lighter than its ground.
    pub lifted: bool,
    /// Each scaling pass's gain on what's under it, times one and that.
    pub gains: Vec<[f32; 3]>,
    pub ground: [f32; 3],
    /// How far the mark's ink lies from its ground, as a size.
    pub step: [f32; 3],
    /// Whether the mark's ink is lighter than its ground.
    pub rises: bool,
}

impl Recolour {
    /// None where a channel goes the other way from the rest, or barely
    /// at all, which no blend's passes can follow; one blend does there.
    pub fn between(
        under_ground: [f32; 3],
        under_ink: [f32; 3],
        ground: [f32; 3],
        ink: [f32; 3],
    ) -> Option<Recolour> {
        let spread: [f32; 3] = std::array::from_fn(|c| under_ink[c] - under_ground[c]);
        let lifted = spread.iter().all(|s| *s >= LEAST_SPREAD);
        if !lifted && !spread.iter().all(|s| *s <= -LEAST_SPREAD) {
            return None;
        }
        let step: [f32; 3] = std::array::from_fn(|c| ink[c] - ground[c]);
        let rises = step.iter().all(|s| *s >= 0.0);
        if !rises && !step.iter().all(|s| *s <= 0.0) {
            return None;
        }
        // A blend's constant is at most one, so each pass at most doubles.
        let mut left = spread.map(|s| 1.0 / (s.abs() * WHOLE_COVER));
        let mut gains = Vec::new();
        while left.iter().any(|f| *f > 1.0 + 1e-4) {
            let gain = left.map(|f| (f - 1.0).clamp(0.0, 1.0));
            left = std::array::from_fn(|c| left[c] / (1.0 + gain[c]));
            gains.push(gain);
        }
        Some(Recolour {
            under: under_ground,
            lifted,
            gains,
            ground,
            step: step.map(|s| {
                if rises {
                    s + QUARTER_STEP
                } else {
                    -s - QUARTER_STEP
                }
                .clamp(0.0, 1.0)
            }),
            rises,
        })
    }

    /// What the passes make of a colour under them, each held to what a
    /// screen's pixel can hold, as GL's are.
    #[cfg(test)]
    pub fn over(&self, under: [f32; 3]) -> [f32; 3] {
        let mut cover: [f32; 3] = std::array::from_fn(|c| {
            let off = under[c] - self.under[c];
            (if self.lifted { off } else { -off }).clamp(0.0, 1.0)
        });
        for gain in &self.gains {
            cover = std::array::from_fn(|c| (cover[c] * (1.0 + gain[c])).clamp(0.0, 1.0));
        }
        std::array::from_fn(|c| {
            let by = self.step[c] * cover[c];
            (if self.rises {
                self.ground[c] + by
            } else {
                self.ground[c] - by
            })
            .clamp(0.0, 1.0)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Blend, Recolour};

    const FOREST: [f32; 3] = [0.067, 0.110, 0.094];
    const INK: [f32; 3] = [0.902, 0.875, 0.784];

    fn near(a: [f32; 3], b: [f32; 3], within: f32) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() <= within)
    }

    #[test]
    fn a_lighter_mark_takes_ground_and_text_where_neovim_puts_them() {
        let (fern, cream) = ([0.137, 0.216, 0.169], [0.969, 0.910, 0.698]);
        let blend = Blend::between(FOREST, INK, fern, cream);
        assert!(
            !blend.flipped,
            "a lighter mark needs no flipped blend: {blend:?}"
        );
        assert!(
            near(blend.over(FOREST), fern, 0.01),
            "the blend made {:?} of {FOREST:?}, not {fern:?}",
            blend.over(FOREST)
        );
        assert!(
            near(blend.over(INK), cream, 0.01),
            "the blend made {:?} of {INK:?}, not {cream:?}",
            blend.over(INK)
        );
    }

    #[test]
    fn a_mark_under_dark_text_turns_light_text_dark() {
        let (jade, night) = ([0.314, 0.580, 0.459], [0.035, 0.059, 0.051]);
        let blend = Blend::between(FOREST, INK, jade, night);
        assert!(
            blend.flipped,
            "a mark darker under light text flips: {blend:?}"
        );
        assert!(
            near(blend.over(FOREST), jade, 0.03),
            "the blend made {:?} of {FOREST:?}, not {jade:?}",
            blend.over(FOREST)
        );
        // Its text goes as dark as one blend can take it: within a tenth.
        assert!(
            near(blend.over(INK), night, 0.1),
            "the blend made {:?} of {INK:?}, not {night:?}",
            blend.over(INK)
        );
    }

    const TEXT: [f32; 3] = [0.839, 0.835, 0.737];
    const MOSS: [f32; 3] = [0.196, 0.278, 0.231];
    const CREAM: [f32; 3] = [0.969, 0.910, 0.698];

    #[test]
    fn a_selection_takes_its_ground_and_turns_the_page_s_text_and_a_title_strong() {
        let recolour = Recolour::between(FOREST, TEXT, MOSS, CREAM).expect("one way");
        assert!(
            near(recolour.over(FOREST), MOSS, 0.002),
            "the recolour made {:?} of {FOREST:?}, not {MOSS:?}",
            recolour.over(FOREST)
        );
        assert!(
            near(recolour.over(TEXT), CREAM, 0.002),
            "the recolour made {:?} of {TEXT:?}, not {CREAM:?}",
            recolour.over(TEXT)
        );
        // A title already in strong stays strong, where one blend went
        // past it, though its blue is a little darker than the text's.
        assert!(
            near(recolour.over(CREAM), CREAM, 0.002),
            "the recolour made {:?} of {CREAM:?}, not {CREAM:?}",
            recolour.over(CREAM)
        );
        let half: [f32; 3] = std::array::from_fn(|c| f32::midpoint(FOREST[c], TEXT[c]));
        let between: [f32; 3] =
            std::array::from_fn(|c| MOSS[c] + (CREAM[c] - MOSS[c]) * 0.5 / super::WHOLE_COVER);
        assert!(
            near(recolour.over(half), between, 0.002),
            "a glyph's soft edge"
        );
    }

    #[test]
    fn the_cursor_on_a_selection_is_its_ground_with_its_glyph_dark() {
        // Its passes double at most, so a spread of less than a half takes
        // more than one.
        let recolour = Recolour::between(MOSS, CREAM, CREAM, FOREST).expect("one way");
        assert!(
            recolour.gains.len() > 1,
            "a spread under a half takes more than one pass: {:?}",
            recolour.gains
        );
        assert!(
            near(recolour.over(MOSS), CREAM, 0.002),
            "the recolour made {:?} of {MOSS:?}, not {CREAM:?}",
            recolour.over(MOSS)
        );
        assert!(
            near(recolour.over(CREAM), FOREST, 0.002),
            "the recolour made {:?} of {CREAM:?}, not {FOREST:?}",
            recolour.over(CREAM)
        );
        assert!(
            near(recolour.over([1.0; 3]), FOREST, 0.002),
            "the recolour made {:?} of white, not {FOREST:?}",
            recolour.over([1.0; 3])
        );
    }

    #[test]
    fn a_light_page_is_read_from_its_ground_down() {
        let (paper, ink) = ([0.95, 0.93, 0.88], [0.15, 0.14, 0.12]);
        let recolour = Recolour::between(paper, ink, ink, paper).expect("one way");
        assert!(
            near(recolour.over(paper), ink, 0.002),
            "the recolour made {:?} of {paper:?}, not {ink:?}",
            recolour.over(paper)
        );
        assert!(
            near(recolour.over(ink), paper, 0.002),
            "the recolour made {:?} of {ink:?}, not {paper:?}",
            recolour.over(ink)
        );
    }

    #[test]
    fn colours_that_part_two_ways_are_left_to_one_blend() {
        let crossed = [0.8, 0.05, 0.8];
        assert_eq!(Recolour::between(FOREST, crossed, MOSS, CREAM), None);
        assert_eq!(Recolour::between(FOREST, INK, MOSS, [0.9, 0.1, 0.9]), None);
    }
}
