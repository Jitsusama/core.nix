//! What the page is set at first and what it knows when it's set again:
//! the first width, from the window kept last time, and whether the file
//! was edited between two sets, which may renumber its blocks.

use super::{Setter, Width};

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
fn a_set_says_the_file_was_edited_only_when_its_words_changed() {
    let dir = std::env::temp_dir().join(format!("reader-edited-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let (file, other) = (dir.join("page.md"), dir.join("other.md"));
    std::fs::write(&file, "# One\n\nWords.\n").expect("the file");
    std::fs::write(&other, "# Other\n").expect("the other");
    let mut setter = Setter::new(file.to_str().expect("a path"));
    let mut edited = |change: &dyn Fn(&mut Setter)| {
        change(&mut setter);
        setter.set().expect("set");
        setter.edited()
    };
    let seen = [
        edited(&|_| ()),
        edited(&|setter| setter.fit(Width::for_window(360.0))),
        edited(&|_| std::fs::write(&file, "# Two\n\nOne\n\nWords.\n").expect("saved")),
        edited(&|setter| setter.open(other.to_str().expect("a path"))),
    ];
    std::fs::remove_dir_all(&dir).expect("cleaned");
    assert_eq!(
        seen,
        [false, false, true, false],
        "the first set, a width, a save, another file"
    );
}
