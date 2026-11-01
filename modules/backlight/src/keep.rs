//! Keeps the panel's saved level current while the lid is open, so the next
//! boot comes up at the level last set.
//!
//! systemd-backlight saves each level at shutdown, but the panel's only
//! while the lid is open (saves-while-open.sh), since a shut lid's panel
//! reads dark. A machine shut down with its lid shut then came up at the
//! level saved at the last shutdown with it open, whatever the light and
//! Joel had set since: 38 of 100 rather than 43, on 2026-10-31. So run as
//! root with `--keeps-the-open-level <saver>`, the daemon saves through the
//! saver once the panel's brightness has held for a moment after changing,
//! and the saver passes over the panel while a lid is shut, as at shutdown.
//! It waits on the brightness being written, never on a timer.

use std::io;
use std::time::Duration;

/// How long the brightness holds after a change before it's saved, so a
/// glide from one level to another saves once, where it ends.
pub const HOLD: Duration = Duration::from_secs(1);

/// Saves each time the brightness has held for `hold` after changing, until
/// hearing it fails.
pub async fn keep(
    hold: Duration,
    mut changed: impl AsyncFnMut() -> io::Result<()>,
    mut save: impl FnMut(),
) -> io::Result<()> {
    loop {
        changed().await?;
        while let Ok(again) = tokio::time::timeout(hold, changed()).await {
            again?;
        }
        save();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::cell::Cell;

    use tokio::sync::mpsc;
    use tokio::time::sleep;

    // Wide of the gaps either way, so a busy machine's late wakes don't
    // turn a glide into two levels or two levels into one.
    const HELD: Duration = Duration::from_millis(200);

    /// Runs `keep` over changes sent with the gaps given, a gap before each,
    /// and gives how many times it saved once the last has held.
    async fn saves_for(gaps: &[u64]) -> u32 {
        let (tell, mut heard) = mpsc::unbounded_channel::<()>();
        let saved = Cell::new(0);
        let changes = async {
            for &gap in gaps {
                sleep(Duration::from_millis(gap)).await;
                tell.send(()).unwrap();
            }
            sleep(HELD * 3).await;
        };
        let kept = keep(
            HELD,
            async || heard.recv().await.ok_or(io::ErrorKind::BrokenPipe.into()),
            || saved.set(saved.get() + 1),
        );
        tokio::select! {
            () = changes => (),
            _ = kept => panic!("keeping stopped while the brightness was heard"),
        }
        saved.get()
    }

    #[tokio::test]
    async fn a_glide_is_saved_once_where_it_ends() {
        assert_eq!(saves_for(&[0, 5, 5, 5, 5, 5]).await, 1);
    }

    #[tokio::test]
    async fn each_level_that_holds_is_saved() {
        assert_eq!(saves_for(&[0, 800, 800]).await, 3);
    }

    #[tokio::test]
    async fn nothing_changed_is_nothing_saved() {
        assert_eq!(saves_for(&[]).await, 0);
    }

    #[tokio::test]
    async fn a_brightness_that_cant_be_heard_stops_it() {
        let kept = keep(HELD, async || Err(io::ErrorKind::NotFound.into()), || ()).await;
        assert!(kept.is_err());
    }
}
