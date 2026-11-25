//! What the reloader takes from the asks that came while it set, and when
//! a page set again comes of an edit, which lets the marks kept from the
//! last time land again from what was kept only when it doesn't. The
//! sequences below call `Shown` as `App::saved` and `shows` do, in their
//! order, since an `App` needs a window to make; that wiring itself is
//! read, not run.

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

/// What the page shown holds, as `App::saved` judges each set and a set
/// shows: a save set for a width the window has left is let go unjudged,
/// and the page shown keeps its words until a page shows, so the set at
/// the new width says the edit.
#[test]
fn an_edit_whose_set_was_let_go_for_a_width_left_is_said_by_the_next() {
    let (opened, saved) = (1, 2);
    let (left, now) = (300.0, 500.0);
    let mut shown = Shown::of(opened);
    assert_eq!(
        shown.takes(left, now, saved),
        None,
        "the save, set at the width the window left, is let go"
    );
    assert_eq!(
        shown.takes(now, now, saved),
        Some(true),
        "set again at the new width, it says the edit"
    );
    shown.shows(saved);
    assert_eq!(
        shown.takes(now, now, saved),
        Some(false),
        "once its words show, the same words set again aren't an edit"
    );
}

/// A set replaced by a newer one before it shows leaves the page shown as
/// it was, so the newer still says the edit the first carried, though its
/// own words are the same as the first's.
#[test]
fn an_edit_whose_set_a_newer_replaced_before_it_showed_is_said_by_the_newer() {
    let (opened, saved) = (1, 2);
    let shown = Shown::of(opened);
    assert_eq!(shown.takes(500.0, 500.0, saved), Some(true), "the save");
    assert_eq!(
        shown.takes(500.0, 500.0, saved),
        Some(true),
        "pictures come before it shows: set again, it still says the edit"
    );
    assert_eq!(shown, Shown::of(opened), "judging moved nothing");
    assert_eq!(
        Shown::of(opened).takes(500.0, 500.0, opened),
        Some(false),
        "pictures come or a width: the same words"
    );
}
