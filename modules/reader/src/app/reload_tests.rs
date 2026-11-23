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

#[test]
fn an_edit_counts_until_its_words_show_however_many_sets_go_unshown() {
    let (before, after) = (1, 2);
    let mut shown = before;
    assert!(
        !edited(&mut shown, before),
        "pictures come or a width: the same words"
    );
    // A save's set let go for a width the window has left is never held
    // against the page shown, so the set at the new width still says it.
    assert!(
        edited(&mut shown, after),
        "the save's words, set at the new width"
    );
    assert!(!edited(&mut shown, after), "set again once they show");
}
