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
                let setter = kept.get_or_insert_with(|| page::Setter::new(&file));
                setter.open(&file);
                setter.fit(width);
                let (set, whole) = guarded(|| {
                    setter.set().map(|(page, ms)| {
                        let lines = text::lines(text::runs(&page.frame));
                        (page, ms, lines, setter.title().to_string(), setter.words())
                    })
                });
                // A setter unwound part-way through a set may hold what
                // that set left half-made, so the next ask makes its own,
                // reading the fonts again once.
                if !whole {
                    kept = None;
                }
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
    use super::guarded;

    #[test]
    fn a_set_that_panics_is_told_as_failing_and_the_next_still_sets() {
        // The panic's own message goes to stderr, as the default hook
        // writes it; what's asserted is what the reloader hears.
        let (first, whole) = guarded::<u8>(|| panic!("a page that can't be laid out"));
        assert_eq!(first, Err("it panicked".to_string()));
        assert!(!whole, "a panic says the setter didn't run whole");
        assert_eq!(guarded(|| Ok(7)), (Ok(7), true));
    }

    #[test]
    fn a_set_that_fails_without_a_panic_ran_whole() {
        let failed = guarded::<u8>(|| Err("can't read it".to_string()));
        assert_eq!(failed, (Err("can't read it".to_string()), true));
    }
}
