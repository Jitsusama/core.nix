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
            let mut setter: Option<page::Setter> = None;
            while let Ok(ask) = asks.recv() {
                let (open, fit) = app::newest(
                    std::iter::once(ask).chain(std::iter::from_fn(|| asks.try_recv().ok())),
                );
                let setter = setter.get_or_insert_with(|| {
                    let mut setter = page::Setter::new(&first);
                    setter.fit(width);
                    setter
                });
                if let Some(file) = open {
                    setter.open(&file);
                }
                if let Some(width) = fit {
                    setter.fit(width);
                }
                let set = setter.set().map(|(page, ms)| {
                    let lines = text::lines(text::runs(&page.frame));
                    (page, ms, lines, setter.title().to_string(), setter.words())
                });
                if reset_to.send((setter.file().to_string(), set)).is_err() {
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
