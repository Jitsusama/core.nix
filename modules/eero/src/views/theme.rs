//! The ink a terminal is given: the terminal kit's pen, which reads the
//! account's theme for each role's colour and each mark as every
//! command's does. eero reads no theme of its own.

use terminal_kit::ink::Pen;

use super::text::Ink;

/// The ink for standard output, drawn with `pen`: whether it's coloured,
/// and whether its marks are the console's stand-ins, is the terminal
/// kit's to decide, from `--color`, `NO_COLOR`, `FORCE_COLOR`, `TERM` and
/// whether it's a terminal.
pub fn ink(pen: &Pen) -> Ink {
    super::marks::take(pen);
    Ink::new(pen.clone())
}
