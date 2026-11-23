//! What the reloader takes from the asks that came while it set, and when
//! a page set again comes of an edit, which lets the marks kept from the
//! last time land again from what was kept only when it doesn't.

use super::{Reload, edited, newest};
use crate::page::Width;

#[test]
fn the_newest_file_and_width_are_taken_and_setting_again_adds_nothing() {
    let (narrow, wide) = (Width::for_window(360.0), Width::for_window(1200.0));
    assert_eq!(
        newest([Reload::Again, Reload::Again]),
        (None, None),
        "a save and a fetch"
    );
    assert_eq!(
        newest([
            Reload::Open("one.md".into()),
            Reload::Fit(narrow),
            Reload::Again,
            Reload::Open("two.md".into()),
            Reload::Fit(wide),
        ]),
        (Some("two.md".into()), Some(wide)),
        "the newest of each"
    );
}

/// What `edited` holds: each set against the words last taken, which then
/// it holds. That a set let go for a width the window left is never taken
/// is `App::saved`'s, which returns before `take_incoming` asks; this
/// holds `edited` alone.
#[test]
fn an_edit_is_held_against_the_words_last_taken_and_then_holds_them() {
    let (before, after) = (1, 2);
    let mut shown = before;
    assert!(
        !edited(&mut shown, before),
        "pictures come or a width: the same words"
    );
    // The save's words, first taken in the set at the new width.
    assert!(
        edited(&mut shown, after),
        "the save's words, set at the new width"
    );
    assert!(!edited(&mut shown, after), "set again once they show");
}
