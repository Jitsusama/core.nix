//! How bright the screen should be in each light, and how that learns from
//! the brightness Joel picks.
//!
//! The eye feels light by its ratio, not its difference, so the curve runs
//! along ln(1 + lux): a step from 10 to 20 lux counts for as much as one from
//! 1000 to 2000.

/// One point the curve passes through: in this much light, the screen this
/// bright, as a share of its brightest.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub lux: f64,
    pub level: f64,
}

/// The dimmest the screen is ever set, so following the dark never leaves
/// it black.
pub const DIMMEST: f64 = 0.02;

/// How close along the curve a pick has to be to an old point to replace it
/// rather than sit beside it.
const NEAR: f64 = 0.35;

/// Points in order of light, never dimmer in brighter light.
#[derive(Clone, Debug, PartialEq)]
pub struct Curve {
    points: Vec<Point>,
}

/// Where a light level sits along the curve.
fn along(lux: f64) -> f64 {
    lux.max(0.0).ln_1p()
}

impl Curve {
    /// A curve through these points, put in order of light. A point dimmer
    /// than one in less light is raised to it, so the screen never dims as
    /// the room brightens.
    pub fn through(points: impl IntoIterator<Item = Point>) -> Self {
        let mut points: Vec<Point> = points
            .into_iter()
            .map(|point| Point {
                lux: point.lux.max(0.0),
                level: point.level.clamp(DIMMEST, 1.0),
            })
            .collect();
        points.sort_by(|a, b| a.lux.total_cmp(&b.lux));
        for at in 1..points.len() {
            points[at].level = points[at].level.max(points[at - 1].level);
        }
        Curve { points }
    }

    /// How bright the screen should be in this light: between two points,
    /// the share of the way along from one to the next; past either end, the
    /// end's.
    pub fn level_at(&self, lux: f64) -> f64 {
        let (Some(first), Some(last)) = (self.points.first(), self.points.last()) else {
            return 1.0;
        };
        let here = along(lux);
        if here <= along(first.lux) {
            return first.level;
        }
        for pair in self.points.windows(2) {
            let (from, to) = (pair[0], pair[1]);
            let (start, end) = (along(from.lux), along(to.lux));
            if here <= end {
                let share = if end > start {
                    (here - start) / (end - start)
                } else {
                    1.0
                };
                return from.level + (to.level - from.level) * share;
            }
        }
        last.level
    }

    /// Joel picked this brightness in this light. The pick replaces any point
    /// near it, and any point it contradicts: brighter in less light, or
    /// dimmer in more. The rest of the curve stays as it was.
    pub fn learn(&mut self, lux: f64, level: f64) {
        let lux = lux.max(0.0);
        let level = level.clamp(DIMMEST, 1.0);
        let here = along(lux);
        self.points.retain(|point| {
            (along(point.lux) - here).abs() >= NEAR
                && !(point.lux < lux && point.level > level)
                && !(point.lux > lux && point.level < level)
        });
        let at = self.points.partition_point(|point| point.lux < lux);
        self.points.insert(at, Point { lux, level });
    }

    /// A curve from text, a point to a line: the lux, a space and the level.
    /// Blank lines and lines starting with # are skipped. Anything else, or
    /// no points at all, reads as nothing.
    pub fn read(text: &str) -> Option<Self> {
        let points = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.split_whitespace();
                let lux: f64 = fields.next()?.parse().ok()?;
                let level: f64 = fields.next()?.parse().ok()?;
                let whole = fields.next().is_none() && lux.is_finite() && level.is_finite();
                whole.then_some(Point { lux, level })
            })
            .collect::<Option<Vec<_>>>()?;
        (!points.is_empty()).then(|| Curve::through(points))
    }

    /// The curve as text that `read` reads back.
    pub fn write(&self) -> String {
        self.points
            .iter()
            .map(|point| format!("{} {:.3}\n", point.lux, point.level))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn standard() -> Curve {
        Curve::through([
            Point {
                lux: 0.0,
                level: 0.15,
            },
            Point {
                lux: 20.0,
                level: 0.3,
            },
            Point {
                lux: 150.0,
                level: 0.5,
            },
            Point {
                lux: 1000.0,
                level: 0.75,
            },
            Point {
                lux: 10000.0,
                level: 1.0,
            },
        ])
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn a_point_s_own_light_gives_its_level() {
        let curve = standard();
        assert!(close(curve.level_at(20.0), 0.3));
        assert!(close(curve.level_at(1000.0), 0.75));
    }

    #[test]
    fn between_points_the_level_follows_the_light_s_ratio() {
        let curve = standard();
        // Halfway along ln(1 + lux) from 20 to 150 lux is about 55 lux.
        let halfway = (along(20.0) + along(150.0)) / 2.0;
        let lux = halfway.exp() - 1.0;
        assert!(close(curve.level_at(lux), 0.4));
    }

    #[test]
    fn past_either_end_the_end_s_level_holds() {
        let curve = standard();
        assert!(close(curve.level_at(-5.0), 0.15));
        assert!(close(curve.level_at(1e6), 1.0));
    }

    #[test]
    fn a_pick_is_what_the_curve_gives_in_that_light_afterwards() {
        let mut curve = standard();
        curve.learn(60.0, 0.62);
        assert!(close(curve.level_at(60.0), 0.62));
    }

    #[test]
    fn a_pick_replaces_the_point_near_it() {
        let mut curve = standard();
        curve.learn(22.0, 0.35);
        assert!(!curve.points.iter().any(|point| close(point.lux, 20.0)));
        assert!(close(curve.level_at(22.0), 0.35));
    }

    #[test]
    fn a_pick_drops_the_points_it_contradicts_and_keeps_the_rest() {
        let mut curve = standard();
        // Bright in a dim room: the points dimmer in more light go, and the
        // one dimmer in less light stays.
        curve.learn(5.0, 0.6);
        let kept: Vec<f64> = curve.points.iter().map(|point| point.lux).collect();
        assert_eq!(kept, [0.0, 5.0, 1000.0, 10000.0]);
    }

    #[test]
    fn the_curve_never_dims_as_the_light_grows() {
        let mut curve = standard();
        curve.learn(5.0, 0.6);
        curve.learn(3000.0, 0.4);
        curve.learn(0.0, 0.9);
        let levels: Vec<f64> = (0..200)
            .map(|step| curve.level_at(f64::from(step) * 100.0))
            .collect();
        assert!(levels.windows(2).all(|pair| pair[1] >= pair[0]));
    }

    #[test]
    fn a_pick_is_never_darker_than_the_dimmest() {
        let mut curve = standard();
        curve.learn(0.0, 0.0);
        assert!(close(curve.level_at(0.0), DIMMEST));
    }

    #[test]
    fn points_given_out_of_order_are_put_in_order_and_never_dim() {
        let curve = Curve::through([
            Point {
                lux: 100.0,
                level: 0.2,
            },
            Point {
                lux: 0.0,
                level: 0.5,
            },
        ]);
        assert_eq!(
            curve.points,
            [
                Point {
                    lux: 0.0,
                    level: 0.5
                },
                Point {
                    lux: 100.0,
                    level: 0.5
                }
            ]
        );
    }

    #[test]
    fn what_is_written_reads_back_the_same() {
        let mut curve = standard();
        curve.learn(60.0, 0.625);
        assert_eq!(Curve::read(&curve.write()), Some(curve));
    }

    #[test]
    fn comments_and_blank_lines_are_skipped() {
        let curve = Curve::read("# lux level\n\n0 0.2\n100 0.8\n").unwrap();
        assert!(close(curve.level_at(100.0), 0.8));
    }

    #[test]
    fn anything_but_points_reads_as_nothing() {
        assert_eq!(Curve::read(""), None);
        assert_eq!(Curve::read("0 0.2\nbright\n"), None);
        assert_eq!(Curve::read("0 0.2 7\n"), None);
        assert_eq!(Curve::read("0 NaN\n"), None);
    }
}
