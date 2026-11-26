//! The reloader: the thread that sets the file again as it is saved, and
//! sets the files a link or a jump opens in the window.

use crate::app::{self, Reload};
use crate::{App, heard, page, text, watch, web};
use smithay_client_toolkit::reexports::calloop::LoopHandle;

/// What asks the reloader to set again, and the watch on the file's saves.
type Started = (std::sync::mpsc::Sender<Reload>, Option<watch::Watch>);

/// Sets the file again on a thread of its own at each save, or once a
/// batch of web pictures has come, and sets another file when a link or a
/// jump opens one in the window. The thread's setter is made at the first
/// ask, so it costs the first screen nothing, and asks that come while one
/// sets are taken as one: the newest file asked for, as it is now, at the
/// newest width. Gives back what asks it, and the watch on the file's
/// saves, which moves with the file shown.
pub(crate) fn reload_on_save(
    handle: &LoopHandle<'static, App>,
    file: &str,
    width: page::Width,
) -> Result<Started, Box<dyn std::error::Error>> {
    let reset_to = heard(handle, App::saved)?;
    let (reload_to, asks) = std::sync::mpsc::channel::<Reload>();
    let first = file.to_string();
    std::thread::Builder::new()
        .name("reloader".into())
        .spawn(move || {
            // The setter, and the file and width it's on, so one made again
            // after a panic takes them up where it left off.
            let mut kept: Option<page::Setter> = None;
            let mut asked = (first, width);
            while let Ok(ask) = asks.recv() {
                asked = asked_for(
                    asked,
                    std::iter::once(ask).chain(std::iter::from_fn(|| asks.try_recv().ok())),
                );
                let set = step(
                    &mut kept,
                    || page::Setter::new(&asked.0),
                    |setter| readied(setter, &asked),
                    |setter| {
                        setter.set().map(|(page, ms)| {
                            let lines = text::lines(text::runs(&page.frame));
                            (page, ms, lines, setter.title().to_string(), setter.words())
                        })
                    },
                );
                if reset_to.send((asked.0.clone(), set)).is_err() {
                    return;
                }
            }
        })?;
    // Pictures fetched from the web set the page again, the file unedited.
    let fetched_to = reload_to.clone();
    web::when_fetched(move || {
        let _ = fetched_to.send(Reload::Again);
    });
    let save_to = reload_to.clone();
    let watch = watch::saves(std::path::Path::new(file), move || {
        save_to.send(Reload::Again).is_ok()
    })
    .map_err(|error| eprintln!("reader: can't watch {file} for saves: {error}"))
    .ok();
    Ok((reload_to, watch))
}

/// The file and width a set is for: the newest of each that `asks` asks
/// for, else what the set before was for, so a save after a link opened
/// another file sets that one, on a setter kept or made again.
fn asked_for(
    was: (String, page::Width),
    asks: impl IntoIterator<Item = Reload>,
) -> (String, page::Width) {
    let (open, fit) = app::newest(asks);
    (open.unwrap_or(was.0), fit.unwrap_or(was.1))
}

/// What a setter is readied with at each ask: the file to set, and the
/// width to set it at.
trait Readied {
    fn open(&mut self, file: &str);
    fn fit(&mut self, width: page::Width);
}

impl Readied for page::Setter {
    fn open(&mut self, file: &str) {
        page::Setter::open(self, file);
    }

    fn fit(&mut self, width: page::Width) {
        page::Setter::fit(self, width);
    }
}

/// Readies a setter for the file and width `asked`: one kept was readied
/// for the ask before, which may have been another file or width.
fn readied(setter: &mut impl Readied, asked: &(String, page::Width)) {
    setter.open(&asked.0);
    setter.fit(asked.1);
}

/// One ask's set: on the setter `kept`, or one `make` makes, readied for
/// the file and width asked for, with a panic anywhere in it told as the
/// set failing. A setter unwound part-way may hold what that set left
/// half-made, so it's dropped and the next ask makes its own. A file that panics at every set
/// makes one at every ask, its fonts read again each time; nothing sets it
/// again but a save or the pictures it waits on.
fn step<S, T>(
    kept: &mut Option<S>,
    make: impl FnOnce() -> S,
    ready: impl FnOnce(&mut S),
    set: impl FnOnce(&mut S) -> Result<T, String>,
) -> Result<T, String> {
    let (done, whole) = guarded(|| {
        let setter = kept.get_or_insert_with(make);
        ready(setter);
        set(setter)
    });
    if !whole {
        *kept = None;
    }
    done
}

/// A set and what's read of it, with a panic in either told as the set
/// failing, as the rest's setter tells one, so the reloader lives on to
/// set the next save, and a file being opened is let go rather than
/// waited on all session; and whether it ran whole, without one.
fn guarded<T>(set: impl FnOnce() -> Result<T, String>) -> (Result<T, String>, bool) {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(set)).map_or_else(
        |_| (Err("it panicked".to_string()), false),
        |set| (set, true),
    )
}

#[cfg(test)]
mod tests {
    use super::{Readied, asked_for, guarded, readied, step};
    use crate::app::Reload;
    use crate::page::Width;

    #[test]
    fn a_set_that_panics_is_told_as_failing_and_not_whole() {
        // The panic's own message goes to stderr, as the default hook
        // writes it; what's asserted is what the reloader hears.
        let (first, whole) = guarded::<u8>(|| panic!("a page that can't be laid out"));
        assert_eq!(first, Err("it panicked".to_string()));
        assert!(!whole, "a panic says the setter didn't run whole");
    }

    /// A setter as `step` keeps one: which it is, by the order it was made
    /// in, and the file and width it was readied for.
    #[derive(Debug, Default)]
    struct Setter {
        made: u8,
        file: Option<String>,
        width: Option<Width>,
    }

    impl Readied for Setter {
        fn open(&mut self, file: &str) {
            self.file = Some(file.to_string());
        }

        fn fit(&mut self, width: Width) {
            self.width = Some(width);
        }
    }

    /// Which setter a set ran on, and the file and width it was readied for.
    type Ran = (u8, Option<String>, Option<Width>);

    #[test]
    fn each_set_is_readied_for_the_newest_asked_on_a_setter_kept_or_made_again() {
        let (narrow, wide) = (Width::for_window(360.0), Width::for_window(1200.0));
        let mut kept = None;
        let make = |made| {
            move || Setter {
                made,
                ..Setter::default()
            }
        };
        let read = |setter: &mut Setter| Ok((setter.made, setter.file.clone(), setter.width));
        let on = |made, file: &str, width| Ok((made, Some(file.to_string()), Some(width)));
        // Opened on notes.md: the first set makes its setter.
        let mut asked = asked_for(("notes.md".into(), narrow), [Reload::Again]);
        let first = step(
            &mut kept,
            make(1),
            |s: &mut Setter| readied(s, &asked),
            read,
        );
        assert_eq!(first, on(1, "notes.md", narrow), "the first set");
        // A link opens two.md: the setter kept sets it, not notes.md.
        asked = asked_for(asked, [Reload::Open("two.md".into())]);
        let linked = step(
            &mut kept,
            make(2),
            |s: &mut Setter| readied(s, &asked),
            read,
        );
        assert_eq!(
            linked,
            on(1, "two.md", narrow),
            "a setter that ran whole is kept, and readied for the file a link opened"
        );
        // Widened, and the set panics: the setter unwound goes.
        asked = asked_for(asked, [Reload::Fit(wide)]);
        let panicked = step(
            &mut kept,
            make(3),
            |s: &mut Setter| readied(s, &asked),
            |_| -> Result<Ran, String> { panic!("a page that can't be laid out") },
        );
        assert_eq!(panicked, Err("it panicked".to_string()));
        assert!(kept.is_none(), "the setter unwound is dropped");
        // A save: a new setter takes up two.md, wide, where the last left off.
        asked = asked_for(asked, [Reload::Again]);
        let saved = step(
            &mut kept,
            make(4),
            |s: &mut Setter| readied(s, &asked),
            read,
        );
        assert_eq!(
            saved,
            on(4, "two.md", wide),
            "a save after a panic sets the file and width last asked for"
        );
    }

    #[test]
    fn a_setter_that_panics_as_its_made_is_told_as_a_failing_set() {
        let mut kept: Option<Setter> = None;
        let made = step(
            &mut kept,
            || panic!("fonts that can't be read"),
            |_| {},
            |_| Ok(()),
        );
        assert_eq!(made, Err("it panicked".to_string()));
        assert!(
            kept.is_none(),
            "a setter that panicked as it was made was kept"
        );
    }

    #[test]
    fn a_set_that_fails_without_a_panic_ran_whole() {
        let failed = guarded::<u8>(|| Err("can't read it".to_string()));
        assert_eq!(failed, (Err("can't read it".to_string()), true));
    }
}
