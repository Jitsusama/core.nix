//! A spell of failed writes on its own, as a flapping disk's: what of it
//! health says, and when, on the awake clock and the wall's.

use super::*;

/// What a spell said and told, at each of `tries`: a write's error, or
/// none, so many seconds in on the awake clock and on the wall's.
fn heard(tries: &[(u64, u64, Option<&io::Error>)]) -> Vec<(u64, &'static str, String)> {
    let step = Cell::new(0);
    let heard = RefCell::new(Vec::new());
    let say = |words: &str| {
        heard
            .borrow_mut()
            .push((step.get(), "said", words.to_owned()));
    };
    let notice = |words: &str| {
        heard
            .borrow_mut()
            .push((step.get(), "told", words.to_owned()));
    };
    let first = Instant::now();
    let mut spell = Spell::default();
    for &(after, on_the_wall, error) in tries {
        step.set(after);
        spell.tried(
            error,
            first + Duration::from_secs(after),
            wall(on_the_wall),
            Path::new("days"),
            &say,
            &notice,
        );
    }
    heard.take()
}

#[test]
fn a_flapping_disk_is_said_at_most_once_in_ten_minutes() {
    let full = io::Error::new(io::ErrorKind::StorageFull, "no space left on device");
    let unreadable = io::Error::new(io::ErrorKind::PermissionDenied, "permission denied");
    let tries = [
        // A short spell, said and told.
        (0, Some(&full)),
        (30, None),
        // A flap soon after goes unsaid, and so does the next, until ten
        // minutes have passed since the end told, when it's said with how
        // long it has lasted, its error then, not its first, the folder's
        // and no longer the disk's, and the short spell unsaid before it.
        (60, Some(&full)),
        (90, None),
        (120, Some(&full)),
        (600, Some(&unreadable)),
        (630, Some(&unreadable)),
        (690, None),
        // An unsaid flap's end doesn't hold off the next spell: ten
        // minutes from the end told, one is said at once, with it.
        (900, Some(&full)),
        (930, None),
        (1290, Some(&full)),
    ]
    .map(|(after, error)| (after, after, error));
    let heard = heard(&tries);
    let when: Vec<(u64, &str)> = heard.iter().map(|(at, kind, _)| (*at, *kind)).collect();
    assert_eq!(
        when,
        [
            (0, "said"),
            (30, "told"),
            (630, "said"),
            (690, "told"),
            (1290, "said")
        ],
        "a flapping disk was said more than once in ten minutes, or a spell at the wrong time"
    );
    let words = |at: usize| heard[at].2.as_str();
    assert!(
        words(1).ends_with("after trying for less than a minute")
            && words(2).contains(
                ": permission denied, having tried for 8 minutes, after a short spell it didn't \
                 say, so"
            )
            && words(2).contains("for up to 51 minutes")
            && words(3).ends_with("after trying for 9 minutes")
            && words(4).contains("no space left on device, after a short spell it didn't say, so")
            && words(4).contains("for up to 60 minutes"),
        "a spell's cause, how long it lasted, the time it has left or the short spells before \
         it went unsaid: {heard:?}"
    );
}

#[test]
fn short_spells_gone_unsaid_are_counted_in_the_next_said() {
    let full = io::Error::new(io::ErrorKind::StorageFull, "no space left on device");
    let mut tries = vec![(0, Some(&full)), (30, None)];
    for flap in 1..=3 {
        tries.extend([(flap * 60, Some(&full)), (flap * 60 + 30, None)]);
    }
    tries.push((630, Some(&full)));
    let tries: Vec<_> = tries
        .into_iter()
        .map(|(after, error)| (after, after, error))
        .collect();
    let heard = heard(&tries);
    assert!(
        heard.len() == 3
            && heard[2]
                .2
                .contains("device, after 3 short spells it didn't say, so"),
        "the short spells kept unsaid weren't counted in the next said: {heard:?}"
    );
}

#[test]
fn a_sleep_or_a_clock_set_back_ends_a_flapping_disks_quiet() {
    // The awake clock stands still while the machine sleeps, so a wake on
    // a disk still full is a minute on there and twenty on the wall; the
    // wall clock may be set back, too, an hour here.
    let full = io::Error::new(io::ErrorKind::StorageFull, "no space left on device");
    for (woke, on_the_wall) in [(60, 1200), (60, 0)] {
        let set_back = on_the_wall == 0;
        let told = if set_back { 3600 } else { 0 };
        let heard = heard(&[
            (0, told, Some(&full)),
            (30, told + 30, None),
            (woke, on_the_wall, Some(&full)),
        ]);
        assert!(
            heard.len() == 3 && heard[2].1 == "said",
            "a spell after a sleep, or the clock set back ({set_back}), went unsaid: {heard:?}"
        );
    }
}

#[test]
fn how_long_a_spell_lasted_reads_to_the_minute() {
    let read = |seconds| spell::lasting(Duration::from_secs(seconds));
    assert_eq!(
        [read(59), read(60), read(119), read(120)],
        ["less than a minute", "a minute", "a minute", "2 minutes"],
        "how long a spell lasted read wrong"
    );
}
