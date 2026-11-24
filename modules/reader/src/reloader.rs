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
            let (mut file, mut width) = (first, width);
            while let Ok(ask) = asks.recv() {
                let (open, fit) = app::newest(
                    std::iter::once(ask).chain(std::iter::from_fn(|| asks.try_recv().ok())),
                );
                file = open.unwrap_or(file);
                width = fit.unwrap_or(width);
                let set = step(
                    &mut kept,
                    || page::Setter::new(&file),
                    |setter| {
                        setter.open(&file);
                        setter.fit(width);
                    },
                    |setter| {
                        setter.set().map(|(page, ms)| {
                            let lines = text::lines(text::runs(&page.frame));
                            (page, ms, lines, setter.title().to_string(), setter.words())
                        })
                    },
                );
                if reset_to.send((file.clone(), set)).is_err() {
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

/// One ask's set: on the setter `kept`, or one `make` makes, readied for
/// the file and width asked for, with a panic anywhere in it told as the
/// set failing. A setter unwound part-way may hold what that set left
/// half-made, its fonts' files behind a lock the panic poisoned, so it's
/// dropped and the next ask makes its own. A file that panics at every set
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
    use super::{guarded, step};

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
        on: Option<(&'static str, u32)>,
    }

    #[test]
    fn after_a_set_that_panics_the_next_sets_on_a_new_setter_readied_as_asked() {
        let mut kept = None;
        let make = |made| move || Setter { made, on: None };
        let ready = |setter: &mut Setter| setter.on = Some(("notes.md", 400));
        let panicked = step(&mut kept, make(1), ready, |_| -> Result<u8, String> {
            panic!("a page that can't be laid out")
        });
        assert_eq!(panicked, Err("it panicked".to_string()));
        assert!(kept.is_none(), "the setter unwound is dropped");
        let read = |setter: &mut Setter| Ok((setter.made, setter.on));
        assert_eq!(
            step(&mut kept, make(2), ready, read),
            Ok((2, Some(("notes.md", 400))))
        );
        assert_eq!(
            step(&mut kept, make(3), ready, read),
            Ok((2, Some(("notes.md", 400)))),
            "a setter that ran whole is kept"
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
