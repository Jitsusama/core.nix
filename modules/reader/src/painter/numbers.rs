//! The reader's conversions between number types, each said once with why
//! the numbers it takes fit what it gives, rather than a lint silenced at
//! every cast. Each is the bare cast, inlined, so it costs what the cast
//! did, and the glow's painting calls two of them once a pixel. They sit
//! under the painter for now, the pixels' module, until the crate's root
//! can name them beside the others.

/// A length or a place in points or pixels as f32, as GL and tiny-skia
/// take them: a page is a few hundred thousand pixels at most, far inside
/// f32's range, and nothing drawn needs more than its precision.
#[inline]
#[expect(
    clippy::cast_possible_truncation,
    reason = "points and pixels are within a page's few hundred thousand, far inside f32"
)]
pub fn f32_of(x: f64) -> f32 {
    x as f32
}

/// A whole number of pixels, as a texture's size or the window's, as
/// f32: through f64, which holds it exactly, so it rounds once, as the
/// cast straight to f32 would.
#[inline]
pub fn f32_of_whole(n: impl Into<f64>) -> f32 {
    f32_of(n.into())
}

/// A count or an index as f64: characters, lines, rows, a picture's
/// pixels or a table's steps, which f64 holds exactly below 2^53.
#[inline]
#[expect(
    clippy::cast_precision_loss,
    reason = "nothing the reader counts comes near 2^53, past which f64 rounds"
)]
pub fn f64_of(n: usize) -> f64 {
    n as f64
}

/// Nanoseconds as milliseconds: a set's, a frame's or the time a thread
/// waited, which never pass the 2^53 nanoseconds, over a hundred days, past
/// which f64 rounds.
#[inline]
#[expect(
    clippy::cast_precision_loss,
    reason = "a span the reader times is far below 2^53 nanoseconds"
)]
pub fn ms_of_ns(ns: u64) -> f64 {
    ns as f64 / 1e6
}

/// A number as u32, any fraction dropped as the cast drops it, most often
/// already rounded, floored or ceiled: a band's index or count, a zoom key
/// or a step of the measure's script, none negative and all within a
/// page's few hundred thousand.
#[inline]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a band, a zoom key or a step is whole, not negative, and far inside u32"
)]
pub fn u32_of(x: f64) -> u32 {
    x as u32
}

/// A number already made whole as i32, as Wayland and GL size a buffer:
/// a window's pixels at its scale, a few thousand.
#[inline]
#[expect(
    clippy::cast_possible_truncation,
    reason = "a buffer's pixels or its scale are whole and far inside i32"
)]
pub fn i32_of(x: f64) -> i32 {
    x as i32
}

/// A place in a table, any fraction dropped as the cast drops it: within
/// the table, so not negative and far inside usize. Like the cast it is,
/// it gives 0 for a negative number or NaN, and never panics.
#[inline]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a place in a table is within it, so not negative and far inside usize"
)]
pub fn usize_of(x: f64) -> usize {
    x as usize
}
