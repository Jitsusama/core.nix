//! What watches.rs is held to: how folders are watched, waited for and
//! heard, through the stand-in inotify and, on Linux, the kernel's own.

use super::*;
use crate::inotify::{IN_CLOSE_WRITE, IN_DELETE, IN_Q_OVERFLOW, names};
use crate::tests::*;

use crate::fake_inotify::{Fake, NOT_INOTIFY};

#[cfg(target_os = "linux")]
use std::fs;

/// What the folders say, where nothing refuses one for good.
fn told<K: Kernel>(watches: &mut Watches<K>) -> (Vec<u8>, bool) {
    watches
        .read()
        .expect("a folder was refused for good where none should be")
}

#[test]
fn only_a_missing_folder_or_no_room_waits() {
    let cases = [
        (2, Some(Later::Missing), "ENOENT"),
        (20, Some(Later::Missing), "ENOTDIR"),
        (28, Some(Later::Full), "ENOSPC"),
        (12, Some(Later::Full), "ENOMEM"),
        (23, Some(Later::Full), "ENFILE"),
        (24, Some(Later::Full), "EMFILE"),
        (13, Some(Later::Refused), "EACCES"),
        (1, Some(Later::Refused), "EPERM"),
        (40, Some(Later::Refused), "ELOOP"),
        (5, Some(Later::Refused), "EIO"),
        (22, None, "EINVAL"),
        (9, None, "EBADF"),
        (14, None, "EFAULT"),
    ];
    for (code, expected, name) in cases {
        assert_eq!(
            later(&io::Error::from_raw_os_error(code)),
            expected,
            "{name}"
        );
    }
}

#[test]
fn a_folder_made_later_is_watched_and_the_watch_above_let_go_of() {
    let fake = Fake::default();
    fake.make("/a");
    let mut watches = fake.watches(&[("/a/b", IN_CLOSE_WRITE)]).unwrap();
    let waiting = (fake.watched(), told(&mut watches), watches.waits());
    fake.make("/a/b");
    let made = (told(&mut watches), fake.watched(), watches.waits());
    // The watch above, let go of, says so; that isn't the folder's.
    let after = told(&mut watches);
    assert_eq!(
        (waiting, made, after),
        (
            (vec![("/a".into(), MADE)], (vec![], false), false),
            ((vec![], true), vec![("/a/b".into(), IN_CLOSE_WRITE)], false),
            (vec![], false),
        ),
        "a folder made later wasn't watched for from above, without waking the loop, \
         then watched with its own mask, the watch above let go of and unheard"
    );
}

#[test]
fn what_is_made_beside_a_waiting_folder_isnt_heard_as_its_own() {
    let fake = Fake::default();
    fake.make("/a");
    let mut watches = fake.watches(&[("/a/b", IN_CLOSE_WRITE)]).unwrap();
    fake.make("/a/other");
    assert_eq!(
        (told(&mut watches), watches.waits()),
        ((vec![], false), false),
        "a folder made beside one waited for was heard as the one's own"
    );
}

#[test]
fn a_watch_above_two_waiting_folders_is_kept_till_both_are_watched() {
    let fake = Fake::default();
    fake.make("/a");
    let mut watches = fake
        .watches(&[("/a/b", IN_CLOSE_WRITE), ("/a/c", IN_CLOSE_WRITE)])
        .unwrap();
    fake.make("/a/b");
    let one = (told(&mut watches).1, fake.watched());
    fake.make("/a/c");
    let both = (told(&mut watches).1, fake.watched());
    assert_eq!(
        (one, both),
        (
            (
                true,
                vec![("/a".into(), MADE), ("/a/b".into(), IN_CLOSE_WRITE)]
            ),
            (
                true,
                vec![
                    ("/a/b".into(), IN_CLOSE_WRITE),
                    ("/a/c".into(), IN_CLOSE_WRITE)
                ]
            ),
        ),
        "the watch above two folders wasn't kept till both were watched"
    );
}

#[test]
fn a_folder_watched_that_is_above_a_waiting_one_keeps_its_own_watch() {
    let fake = Fake::default();
    fake.make("/a");
    let own = IN_MOVED_TO | IN_DELETE;
    let mut watches = fake
        .watches(&[("/a", own), ("/a/b", IN_CLOSE_WRITE)])
        .unwrap();
    let waiting = fake.watched();
    fake.make("/a/b");
    // Its being made is heard on the folder's own watch, so is its own.
    let (events, heard) = told(&mut watches);
    assert_eq!(
        (
            waiting,
            names(&events, |name| name == b"b"),
            heard,
            fake.watched()
        ),
        (
            vec![("/a".into(), own | MADE)],
            true,
            true,
            vec![("/a".into(), own), ("/a/b".into(), IN_CLOSE_WRITE)],
        ),
        "watching for a folder from one watched took the one's own watch, or left it \
         asking for what's made"
    );
}

#[test]
fn a_folder_missing_two_deep_is_watched_for_from_each_folder_made_toward_it() {
    let fake = Fake::default();
    fake.make("/a");
    let mut watches = fake.watches(&[("/a/b/c", IN_CLOSE_WRITE)]).unwrap();
    fake.make("/a/b");
    let between = (told(&mut watches).1, fake.watched());
    fake.make("/a/b/c");
    let made = (told(&mut watches).1, fake.watched());
    assert_eq!(
        (between, made),
        (
            (false, vec![("/a/b".into(), MADE)]),
            (true, vec![("/a/b/c".into(), IN_CLOSE_WRITE)]),
        ),
        "a folder two deep wasn't watched for from the folder made between"
    );
}

#[test]
fn a_folder_made_as_the_watch_above_is_made_is_watched_at_once() {
    let fake = Fake::default();
    fake.make("/a");
    fake.0.borrow_mut().racing = Some("/a/b".into());
    let watches = fake.watches(&[("/a/b", IN_CLOSE_WRITE)]).unwrap();
    assert_eq!(
        (watches.waits(), fake.watched()),
        (false, vec![("/a/b".into(), IN_CLOSE_WRITE)]),
        "a folder made before its watch above was waited for regardless"
    );
}

#[test]
fn a_folder_removed_once_watched_is_waited_for_again() {
    let fake = Fake::default();
    fake.make("/a");
    fake.make("/a/b");
    let mut watches = fake.watches(&[("/a/b", IN_CLOSE_WRITE)]).unwrap();
    fake.remove("/a/b");
    let removed = (told(&mut watches), fake.watched());
    fake.make("/a/b");
    let made = (told(&mut watches).1, fake.watched());
    assert_eq!(
        (removed, made),
        (
            ((vec![], true), vec![("/a".into(), MADE)]),
            (true, vec![("/a/b".into(), IN_CLOSE_WRITE)]),
        ),
        "a folder removed wasn't heard gone, waited for and watched again"
    );
}

#[test]
fn no_room_is_tried_again_till_there_is() {
    let fake = Fake::default();
    fake.make("/a");
    fake.0.borrow_mut().room = Some(0);
    let mut watches = fake.watches(&[("/a", IN_CLOSE_WRITE)]).unwrap();
    let full = (told(&mut watches).1, watches.waits());
    fake.0.borrow_mut().room = None;
    let room = (told(&mut watches).1, watches.waits(), fake.watched());
    assert_eq!(
        (full, room),
        (
            (false, true),
            (true, false, vec![("/a".into(), IN_CLOSE_WRITE)])
        ),
        "a folder refused for want of room wasn't watched once there was"
    );
}

#[test]
fn inotify_that_wouldnt_start_is_started_once_it_can() {
    let fake = Fake::default();
    fake.make("/a");
    fake.0.borrow_mut().unopenable = Some(24);
    let mut watches = fake.watches(&[("/a", IN_CLOSE_WRITE)]).unwrap();
    let unstarted = (told(&mut watches).1, watches.waits(), fake.watched());
    fake.0.borrow_mut().unopenable = None;
    let started = (told(&mut watches).1, watches.waits(), fake.watched());
    assert_eq!(
        (unstarted, started),
        (
            (false, true, vec![]),
            (true, false, vec![("/a".into(), IN_CLOSE_WRITE)]),
        ),
        "inotify refused for want of room wasn't started, and the folders watched, once \
         it could be"
    );
}

#[test]
fn a_refusal_waiting_cant_mend_fails_naming_what_was_refused() {
    let fake = Fake::default();
    fake.make("/a");
    let refused = |watches: io::Result<Watches<Fake>>| watches.err().map(|e| e.to_string());
    let mask = refused(fake.watches(&[("/a", IN_CLOSE_WRITE), ("/a", NOT_INOTIFY)]));
    fake.0.borrow_mut().unopenable = Some(22);
    let unstarted = refused(fake.watches(&[("/a", IN_CLOSE_WRITE)]));
    assert_eq!(
        (mask.as_deref(), unstarted.as_deref()),
        (
            Some("can't watch /a: Invalid argument (os error 22)"),
            Some("Invalid argument (os error 22)")
        ),
        "a refusal that waiting can't mend was let wait, or said without its folder"
    );
}

#[test]
fn a_refusal_waiting_cant_mend_met_after_the_start_fails_the_read() {
    let fake = Fake::default();
    fake.make("/a");
    let mut watches = fake.watches(&[("/a/b", IN_CLOSE_WRITE)]).unwrap();
    fake.0.borrow_mut().refusing = Some(("/a/b".into(), 22));
    fake.make("/a/b");
    assert_eq!(
        watches.read().err().map(|e| e.to_string()).as_deref(),
        Some("can't watch /a/b: Invalid argument (os error 22)"),
        "a refusal waiting can't mend, met after the start, was let wait with the folder \
         unheard, or said without its folder"
    );
}

#[test]
fn a_folder_the_system_refuses_waits_on_the_clock_and_costs_no_other() {
    let fake = Fake::default();
    for made in ["/a", "/a/b", "/other"] {
        fake.make(made);
    }
    fake.0.borrow_mut().refusing = Some(("/a/b".into(), 13));
    let asked = [
        ("/a/b", IN_CLOSE_WRITE),
        ("/a/b/c", IN_CLOSE_WRITE),
        ("/other", IN_DELETE),
    ];
    let mut watches = fake
        .watches(&asked)
        .expect("a folder the system refuses was taken for the program's fault");
    let refused = (watches.waits(), fake.watched());
    fake.0.borrow_mut().refusing = None;
    fake.make("/a/b/c");
    let lifted = (told(&mut watches).1, watches.waits(), fake.watched().len());
    assert_eq!(
        (refused, lifted),
        ((true, vec![("/other".into(), IN_DELETE)]), (true, false, 3)),
        "a refused folder, or one below it, was left to a watch further up that can't \
         wake it, cost the folder beside it, or wasn't watched once the refusal lifted"
    );
}

#[test]
fn a_read_inotify_fails_ends_the_read_but_an_interrupted_one_reads_on() {
    let cases = [
        (
            5,
            Err("can't read inotify: Input/output error (os error 5)".to_string()),
        ),
        (4, Ok(true)),
    ];
    for (code, expected) in cases {
        let fake = Fake::default();
        fake.make("/a");
        let mut watches = fake.watches(&[("/a", IN_CREATE)]).unwrap();
        fake.0.borrow_mut().faulting = Some(code);
        fake.make("/a/b");
        let read = watches
            .read()
            .map(|(events, _)| names(&events, |name| name == b"b"))
            .map_err(|error| error.to_string());
        assert_eq!(
            read, expected,
            "a read of inotify failing with os error {code}"
        );
    }
}

#[cfg(target_os = "linux")]
#[test]
fn the_kernel_s_inotify_watches_a_folder_once_it_s_made() {
    let sys = Sys::new("watch-later");
    let later = sys.0.join("later");
    assert!(
        Watches::new(&[(sys.0.clone(), NOT_INOTIFY)]).is_err(),
        "the kernel's refusal of a mask it doesn't take was let wait"
    );
    let mut watches = Watches::new(&[(later.clone(), IN_CLOSE_WRITE)]).unwrap();
    let before = told(&mut watches);
    fs::create_dir(&later).unwrap();
    let made = told(&mut watches);
    fs::write(later.join("file"), "x").unwrap();
    let (written, _) = told(&mut watches);
    fs::create_dir(sys.0.join("other")).unwrap();
    let unheard = told(&mut watches);
    assert_eq!(
        (
            before,
            made,
            names(&written, |name| name == b"file"),
            unheard
        ),
        ((vec![], false), (vec![], true), true, (vec![], false)),
        "a folder made later wasn't heard made, watched, then heard from, with the folder \
         above let go of"
    );
}

#[test]
fn a_folder_made_as_its_watch_above_is_but_refused_for_room_waits_with_nothing_above() {
    let fake = Fake::default();
    fake.make("/a");
    fake.0.borrow_mut().racing = Some("/a/b".into());
    fake.0.borrow_mut().room = Some(1);
    let own = IN_MOVED_TO;
    let mut watches = fake
        .watches(&[("/a", own), ("/a/b", IN_CLOSE_WRITE)])
        .unwrap();
    let full = (watches.waits(), fake.watched());
    fake.0.borrow_mut().room = None;
    let room = (told(&mut watches).1, watches.waits(), fake.watched());
    assert_eq!(
        (full, room),
        (
            (true, vec![("/a".into(), own)]),
            (
                true,
                false,
                vec![("/a".into(), own), ("/a/b".into(), IN_CLOSE_WRITE)]
            ),
        ),
        "a folder there but refused for want of room was left to the watch above, \
         which can't wake it, or that watch kept asking for what's made"
    );
}

#[test]
fn a_folder_above_one_waiting_removed_is_looked_past_for_the_next_one_up() {
    let fake = Fake::default();
    fake.make("/a");
    fake.make("/a/b");
    let mut watches = fake.watches(&[("/a/b/c", IN_CLOSE_WRITE)]).unwrap();
    let first = fake.watched();
    fake.remove("/a/b");
    let removed = (told(&mut watches), watches.waits(), fake.watched());
    fake.make("/a/b");
    let again = (told(&mut watches).1, fake.watched());
    assert_eq!(
        (
            first,
            removed,
            again,
            fake.0.borrow().dropped_let_go.clone()
        ),
        (
            vec![("/a/b".into(), MADE)],
            ((vec![], false), false, vec![("/a".into(), MADE)]),
            (false, vec![("/a/b".into(), MADE)]),
            vec![],
        ),
        "a folder waiting on a watch above that was removed wasn't watched for from \
         the next folder up, and from the one made again, or the removed watch was \
         let go of after the kernel dropped it"
    );
}

#[test]
fn a_folder_s_own_watch_dropped_while_one_above_is_let_go_of_no_more() {
    let fake = Fake::default();
    fake.make("/a");
    let asked = [("/a", IN_MOVED_TO), ("/a/b", IN_CLOSE_WRITE)];
    let mut watches = fake.watches(&asked).unwrap();
    fake.remove("/a");
    let removed = (told(&mut watches).1, watches.waits(), fake.watched());
    assert_eq!(
        (removed, fake.0.borrow().dropped_let_go.clone()),
        ((true, true, vec![]), vec![]),
        "a folder's own watch, held above another too, wasn't heard removed, or was \
         let go of after the kernel dropped it"
    );
}

#[test]
fn a_watch_refused_its_own_mask_back_is_set_back_later_unless_the_fault_is_ours() {
    let set_back = |code| {
        let fake = Fake::default();
        fake.make("/a");
        let asked = [("/a", IN_MOVED_TO), ("/a/b", IN_CLOSE_WRITE)];
        let mut watches = fake.watches(&asked).unwrap();
        fake.0.borrow_mut().refusing = Some(("/a".into(), code));
        fake.make("/a/b");
        watches
            .read()
            .map(|_| fake.watched()[0].1)
            .map_err(|error| error.to_string())
    };
    assert_eq!(
        (set_back(13), set_back(22)),
        (
            Ok(IN_MOVED_TO | MADE),
            Err("Invalid argument (os error 22)".to_string())
        ),
        "a watch refused its own mask back wasn't kept hearing what was added, or a \
         refusal that's the program's fault passed"
    );
    let fake = Fake::default();
    fake.make("/a");
    let mut watches = fake
        .watches(&[("/a", IN_MOVED_TO), ("/a/b", IN_CLOSE_WRITE)])
        .unwrap();
    fake.0.borrow_mut().refusing = Some(("/a".into(), 13));
    fake.make("/a/b");
    told(&mut watches);
    fake.0.borrow_mut().refusing = None;
    told(&mut watches);
    assert_eq!(
        fake.watched()[0],
        ("/a".into(), IN_MOVED_TO),
        "a watch refused its own mask back wasn't set back once it could be"
    );
}

#[test]
fn an_overflow_watches_each_folder_afresh_for_a_removal_it_lost() {
    let fake = Fake::default();
    fake.make("/a");
    fake.make("/a/b");
    let mut watches = fake
        .watches(&[("/a", IN_CLOSE_WRITE), ("/a/b", IN_CLOSE_WRITE)])
        .unwrap();
    // /a/b removed and made again, its removal lost in an overflow.
    fake.remove("/a/b");
    fake.0.borrow_mut().events = event_on(-1, IN_Q_OVERFLOW, "");
    fake.make("/a/b");
    let (events, _) = told(&mut watches);
    fake.0.borrow_mut().events = event_on(3, IN_CLOSE_WRITE, "file");
    let (heard, _) = told(&mut watches);
    assert_eq!(
        (
            events,
            fake.watched(),
            names(&heard, |name| name == b"file")
        ),
        (
            event_on(-1, IN_Q_OVERFLOW, ""),
            vec![
                ("/a".into(), IN_CLOSE_WRITE),
                ("/a/b".into(), IN_CLOSE_WRITE)
            ],
            true,
        ),
        "a folder whose removal an overflow lost stayed on its dead watch"
    );
}

#[test]
fn only_the_folders_own_events_and_an_overflow_are_kept() {
    let fake = Fake::default();
    fake.make("/a");
    let mut watches = fake.watches(&[("/a", IN_CLOSE_WRITE)]).unwrap();
    let mut said = event_on(1, IN_CLOSE_WRITE, "own");
    said.extend(event_on(7, IN_CLOSE_WRITE, "stray"));
    said.extend(event_on(-1, IN_Q_OVERFLOW, ""));
    let cut = event_on(1, IN_CLOSE_WRITE, "cut");
    said.extend(&cut[..cut.len() - 1]);
    let mut kept = event_on(1, IN_CLOSE_WRITE, "own");
    kept.extend(event_on(-1, IN_Q_OVERFLOW, ""));
    kept.extend(&cut[..cut.len() - 1]);
    assert_eq!(
        watches.own_events(&said),
        (kept, false),
        "a stray watch's event was kept, or an overflow or the rest cut short dropped"
    );
}
