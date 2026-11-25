//! What the page is set at first and what it knows when it's set again:
//! the first width, from the window kept last time, and the words each set
//! read, which say whether the file was edited between two sets, which may
//! renumber its blocks.

use crate::page::{Setter, Width, set_first};

#[test]
fn the_first_width_is_the_kept_windows_or_else_an_unnamed_ones() {
    // Two windows on different measure steps, so a swap would show.
    let (kept_pt, unnamed_pt) = (360.0, 720.0);
    assert_ne!(
        Width::for_window(kept_pt),
        Width::for_window(unnamed_pt),
        "two steps"
    );
    assert_eq!(
        Width::first(Some(kept_pt), unnamed_pt),
        Width::for_window(kept_pt),
        "the window kept last time"
    );
    assert_eq!(
        Width::first(None, unnamed_pt),
        Width::for_window(unnamed_pt),
        "measuring, or with none kept, the unnamed window's"
    );
}

#[test]
fn a_sets_words_change_only_when_the_file_does() {
    let dir = std::env::temp_dir().join(format!("reader-edited-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("page.md");
    std::fs::write(&file, "# One\n\nWords.\n").expect("the file");
    let path = file.to_str().expect("a path");
    // The first screen's set, which the window holds the reloader's against.
    let first = set_first(path, None, Width::for_window(720.0)).expect("the first screen");
    let mut setter = Setter::new(path);
    let mut words = |change: &dyn Fn(&mut Setter)| {
        change(&mut setter);
        setter.set().expect("set");
        setter.words()
    };
    let (again, wider) = (
        words(&|_| ()),
        words(&|setter| setter.fit(Width::for_window(360.0))),
    );
    let saved = words(&|_| std::fs::write(&file, "# Two\n\nOne\n\nWords.\n").expect("saved"));
    std::fs::remove_dir_all(&dir).expect("cleaned");
    assert_eq!(
        first.words, again,
        "the first screen and the reloader read alike"
    );
    assert_eq!(again, wider, "another width is the same words");
    assert_ne!(again, saved, "a save that changed them");
}
