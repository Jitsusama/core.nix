//! What the reloader takes from the asks that came while it set, and when
//! a page set again comes of an edit, which lets the marks kept from the
//! last time land again from what was kept only when it doesn't. The
//! sequences below call `Shown` as `App::saved` and `App::take_incoming`
//! do, in their order, since an `App` needs a window to make: each set
//! is `sets`, and each page that shows is `shows`.

use super::{Reload, Shown, newest};
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

/// A save, then its pictures come from the web before its page shows: the
/// set from the fetch takes the save's place coming, and though its words
/// are the save's, it still says the edit, since the page shown kept its
/// own until a page showed.
#[test]
fn an_edit_whose_set_a_fetch_replaced_before_it_showed_is_said_by_the_fetchs() {
    let (opened, saved) = (1, 2);
    let mut shown = Shown::of(opened);
    assert_eq!(shown.sets(500.0, 500.0, saved), Some(true), "the save");
    assert_eq!(
        shown.sets(500.0, 500.0, saved),
        Some(true),
        "the fetch, set before the save showed, still says the edit"
    );
    assert!(shown.shows(), "the page that shows comes of the edit");
    assert_eq!(
        shown.sets(500.0, 500.0, saved),
        Some(false),
        "once its words show, the same words set again aren't an edit"
    );
    assert!(!shown.shows(), "and the page that shows comes of none");
}

/// A save set at the width the window has since left is let go unjudged,
/// and the set again at the new width says the edit.
#[test]
fn an_edit_whose_set_was_let_go_for_a_width_left_is_said_by_the_next() {
    let (opened, saved) = (1, 2);
    let (left, now) = (300.0, 500.0);
    let mut shown = Shown::of(opened);
    assert_eq!(
        shown.sets(left, now, saved),
        None,
        "the save, set at the width the window left, is let go"
    );
    assert!(!shown.shows(), "nothing it let go comes");
    assert_eq!(
        shown.sets(now, now, saved),
        Some(true),
        "set again at the new width, it says the edit"
    );
    assert!(shown.shows(), "the page that shows comes of the edit");
}
