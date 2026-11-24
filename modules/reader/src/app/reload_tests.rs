//! What the reloader takes from the asks that came while it set, and when
//! a page set again comes of an edit, which lets the marks kept from the
//! last time land again from what was kept only when it doesn't.

use super::{Reload, newest, taken};
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

/// What `taken` holds, as `App::saved` asks it of each set: a save set for
/// a width the window has left is let go unjudged, and the page shown keeps
/// its words until a page shows, so the set at the new width says the edit.
#[test]
fn an_edit_whose_set_was_let_go_for_a_width_left_is_said_by_the_next() {
    let (shown, saved) = (1, 2);
    let (left, now) = (300.0, 500.0);
    assert_eq!(
        taken(left, now, shown, saved),
        None,
        "the save, set at the width the window left, is let go"
    );
    assert_eq!(
        taken(now, now, shown, saved),
        Some(true),
        "set again at the new width, it says the edit"
    );
    assert_eq!(
        taken(now, now, saved, saved),
        Some(false),
        "once its words show, the same words set again aren't an edit"
    );
    assert_eq!(
        taken(now, now, shown, shown),
        Some(false),
        "pictures come or a width: the same words"
    );
}
