//! What hearing.rs is held to: what's read again as the desk is heard, as
//! poll's wait runs out and as the hearing starts and ends, through the
//! stand-in inotify.

use super::*;
use crate::fake_inotify::{FAKE_FD, Fake};
use crate::inotify::IN_Q_OVERFLOW;
use crate::tests::*;

/// The folders as run watches them, through `fake`: /snd for sound,
/// /holders for cameras and /said for the monitors.
fn folders(fake: &Fake) -> Folders<Fake> {
    let watch = |path| {
        fake.watches(&[(path, IN_CLOSE_WRITE)])
            .expect("a folder missing is waited for")
    };
    Folders {
        changes: watch("/snd"),
        held: watch("/holders"),
        said: watch("/said"),
    }
}

#[test]
fn a_folder_nothing_wakes_has_its_kind_read_as_poll_times_out_and_settles_once_watched() {
    let fake = Fake::default();
    // No room to watch them, or anything above them.
    fake.0.borrow_mut().room = Some(0);
    let mut folders = folders(&fake);
    let (now, mut settling) = (Instant::now(), Settling::default());
    let full = timed_out(&mut folders, &mut settling, now).expect("nothing refused for good");
    let unsettled = settling.since;
    for made in ["/snd", "/holders", "/said"] {
        fake.make(made);
    }
    fake.0.borrow_mut().room = None;
    let made = timed_out(&mut folders, &mut settling, now).expect("nothing refused for good");
    assert_eq!(
        (full, unsettled, made, settling.since, folders.wait()),
        (
            SOUND | CAMERAS | DISPLAYS,
            None,
            SOUND | CAMERAS | DISPLAYS,
            Some((now, 0)),
            false
        ),
        "a folder nothing could wake didn't have its kind read as poll's wait ran out, \
         or settled what it hadn't heard change, or wasn't tried again, its kind read and \
         settled, once it could be watched"
    );
}

#[test]
fn a_folder_refused_for_now_has_only_its_own_kind_read_each_retry() {
    let fake = Fake::default();
    for made in ["/snd", "/holders", "/said"] {
        fake.make(made);
    }
    fake.0.borrow_mut().refusing = Some(("/holders".into(), 13));
    let mut folders = folders(&fake);
    let now = Instant::now();
    let first = timed_out(&mut folders, &mut Settling::default(), now);
    let next = timed_out(&mut folders, &mut Settling::default(), now + RETRY);
    assert_eq!(
        (
            first.map_err(|error| error.to_string()),
            next.map_err(|error| error.to_string())
        ),
        (Ok(CAMERAS), Ok(CAMERAS)),
        "a folder refused for a while didn't have its kind read each time poll's wait ran \
         out, so its facts went stale, or the folders watched beside it were read too"
    );
}

#[test]
fn a_fault_met_as_poll_times_out_ends_the_hearing_with_its_facts_removed() {
    let fake = Fake::default();
    let mut folders = folders(&fake);
    fake.0.borrow_mut().refusing = Some(("/snd".into(), 22));
    fake.make("/snd");
    let fault = timed_out(&mut folders, &mut Settling::default(), Instant::now())
        .map_err(|error| error.to_string());
    let sys = Sys::new("ended");
    let desk = Desk::new(&sys.0, PathBuf::new(), Vec::new());
    for file in &desk.files {
        fs::write(file, "{}").expect("a fact is written");
    }
    let code = end(&desk, ExitCode::FAILURE);
    let left: Vec<_> = desk.files.iter().filter(|file| file.exists()).collect();
    assert_eq!(
        (fault, format!("{code:?}"), left),
        (
            Err("can't watch /snd: Invalid argument (os error 22)".to_string()),
            format!("{:?}", ExitCode::FAILURE),
            Vec::<&PathBuf>::new()
        ),
        "a fault met as poll's wait ran out was passed over, or the hearing ended \
         another way than it was told, or left a fact to be read stale"
    );
}

#[test]
fn a_start_refused_ends_the_hearing_with_its_facts_removed_and_one_made_keeps_them() {
    let sys = Sys::new("starting");
    let desk = Desk::new(&sys.0, PathBuf::new(), Vec::new());
    let write = || {
        for file in &desk.files {
            fs::write(file, "{}").expect("a fact is written");
        }
    };
    let left = || desk.files.iter().filter(|file| file.exists()).count();
    write();
    let refused = started::<()>(&desk, Err("can't hear the desk's devices".into()))
        .map_err(|code| format!("{code:?}"));
    let after_refused = left();
    write();
    let made = started(&desk, Ok(7)).map_err(|code| format!("{code:?}"));
    assert_eq!(
        (refused, after_refused, made, left()),
        (Err(format!("{:?}", ExitCode::FAILURE)), 0, Ok(7), 5),
        "a start refused didn't fail the unit with the facts a killed run left removed, \
         or a start made didn't give what it opened, or removed the facts"
    );
}

#[test]
fn poll_waits_a_retry_for_inotify_to_start_then_on_it_and_for_settling() {
    let fake = Fake::default();
    fake.0.borrow_mut().unopenable = Some(24);
    for made in ["/snd", "/holders", "/said"] {
        fake.make(made);
    }
    let mut folders = folders(&fake);
    let (now, mut settling) = (Instant::now(), Settling::default());
    let mut slots = [-1; 3].map(PollFd::reading);
    let fds = |slots: &[PollFd; 3]| slots.each_ref().map(|slot| slot.fd);
    let unstarted = (
        asked(&folders, &settling, &mut slots, now, now),
        fds(&slots),
    );
    fake.0.borrow_mut().unopenable = None;
    let due = timed_out(&mut folders, &mut settling, now).expect("nothing refused for good");
    let settles = (
        asked(&folders, &settling, &mut slots, now, now),
        fds(&slots),
    );
    let settled = asked(&folders, &Settling::default(), &mut slots, now, now);
    let retry = c_int::try_from(RETRY.as_millis()).expect("the retry fits poll's wait");
    let first = c_int::try_from(SETTLING[0].as_millis()).expect("a settling read fits");
    assert_eq!(
        (unstarted, due, settles, settled),
        (
            (retry, [-1; 3]),
            SOUND | CAMERAS | DISPLAYS,
            (first, [FAKE_FD; 3]),
            -1
        ),
        "poll didn't wait a retry while inotify wouldn't start, then on inotify once it \
         did, till the first settling read, and then till something was heard"
    );
}

// Woken every 5 s, as by uevents or a control's events, a folder refused
// for now still has its kind read each `RETRY`, and poll waits no later
// than the retry past the last, rather than a fresh `RETRY` each wake.
#[test]
fn wakeups_sooner_than_a_retry_dont_keep_a_waiting_folders_kind_unread() {
    let fake = Fake::default();
    for made in ["/snd", "/holders", "/said"] {
        fake.make(made);
    }
    fake.0.borrow_mut().refusing = Some(("/holders".into(), 13));
    let mut folders = folders(&fake);
    let (since, mut settling) = (Instant::now(), Settling::default());
    let mut retried = since;
    let mut slots = [-1; 3].map(PollFd::reading);
    let mut each_wake = Vec::new();
    for wake in 1..=4 {
        let now = since + Duration::from_secs(5 * wake);
        let due = woken(&mut folders, &mut settling, &mut retried, false, now)
            .expect("nothing refused for good");
        each_wake.push((due, asked(&folders, &settling, &mut slots, retried, now)));
    }
    assert_eq!(
        each_wake,
        [(0, 5000), (CAMERAS, 10_000), (0, 5000), (CAMERAS, 10_000)],
        "a wake sooner than a retry read the waiting folder's kind, or wakes every 5 s \
         kept it unread, or poll's wait didn't run to the retry past the last"
    );
}

#[test]
fn a_retry_reads_nothing_settling_before_its_time() {
    let fake = Fake::default();
    for made in ["/snd", "/holders", "/said"] {
        fake.make(made);
    }
    // Only the cameras' folder waits, so only they're read as it's retried.
    fake.0.borrow_mut().refusing = Some(("/holders".into(), 13));
    let mut folders = folders(&fake);
    let (since, mut settling) = (Instant::now(), Settling::default());
    settling.begin(SOUND, since);
    let early = (
        timed_out(&mut folders, &mut settling, since).expect("nothing refused for good"),
        settling.since,
    );
    let later = since + Duration::from_secs(1);
    let due = (
        timed_out(&mut folders, &mut settling, later).expect("nothing refused for good"),
        settling.since,
    );
    assert_eq!(
        (early, due),
        (
            (CAMERAS, Some((since, 0))),
            (SOUND | CAMERAS, Some((since, 1)))
        ),
        "a wait run out for a retry read what's settling before its time, or not once it was"
    );
}

#[test]
fn a_kind_done_settling_isnt_read_again_with_the_next_to_settle() {
    let (since, mut settling) = (Instant::now(), Settling::default());
    settling.begin(SOUND, since);
    let sound: Vec<u8> = SETTLING
        .iter()
        .map(|after| settling.step(since + *after))
        .collect();
    let later = since + Duration::from_secs(10);
    settling.begin(CAMERAS, later);
    let cameras = settling.step(later + SETTLING[0]);
    assert_eq!(
        (sound, settling.since.is_none(), cameras),
        (vec![SOUND; SETTLING.len()], false, CAMERAS),
        "sound, done settling, was read again with the cameras that settle after it"
    );
}

/// A uevent as the kernel sends it: fields split by NULs.
fn uevent(fields: &[&str]) -> Vec<u8> {
    fields.join("\0").into_bytes()
}

/// A descriptor's reads, as the kernel answers them in turn, and then that
/// nothing more waits.
struct Answers(Vec<io::Result<Vec<u8>>>);

impl Read for Answers {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.0.is_empty() {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        let answer = self.0.remove(0)?;
        buffer[..answer.len()].copy_from_slice(&answer);
        Ok(answer.len())
    }
}

#[test]
fn a_read_interrupted_reads_on_one_that_lost_uevents_reads_every_kind_and_a_fault_is_said() {
    let drm = uevent(&["change@/devices/.../drm/card0", "SUBSYSTEM=drm"]);
    let card = uevent(&["add@/devices/.../sound/card1", "SUBSYSTEM=sound"]);
    let refused = |code| -> io::Result<Vec<u8>> { Err(io::Error::from_raw_os_error(code)) };
    let cases = [
        (
            vec![Ok(drm.clone()), Ok(card.clone())],
            Ok(DISPLAYS | SOUND),
        ),
        (
            vec![Ok(drm.clone()), refused(4), Ok(card.clone())],
            Ok(DISPLAYS | SOUND),
        ),
        (vec![refused(ENOBUFS), Ok(drm.clone())], Ok(EVERY)),
        (
            vec![Ok(drm.clone()), Ok(Vec::new()), Ok(card)],
            Ok(DISPLAYS),
        ),
        (
            vec![Ok(drm), refused(5)],
            Err("Input/output error (os error 5)".to_string()),
        ),
    ];
    for (answers, expected) in cases {
        let said = heard(&mut Answers(answers), &matters).map_err(|error| error.to_string());
        assert_eq!(
            said, expected,
            "a read interrupted ended the read, or one that lost uevents didn't read every \
             kind again, or the end of the file was read past, or a fault went unsaid"
        );
    }
}

#[test]
fn a_control_that_cant_be_read_is_let_go_of_as_one_gone() {
    let sys = Sys::new("controls");
    let mut desk = Desk::new(&sys.0, PathBuf::new(), Vec::new());
    // A folder, which refuses to be read, in a card's control's place.
    let control = File::open(&sys.0).expect("a folder opens to be read");
    let mut waiting = [PollFd::reading(control.as_raw_fd())];
    waiting[0].revents = POLLIN;
    desk.controls.push(("1".into(), control));
    let due = desk.heard_from_controls(&mut waiting);
    assert_eq!(
        (due, waiting[0].fd),
        (SOUND, -1),
        "a control whose read was refused was polled again, or sound wasn't read again"
    );
}

#[test]
fn each_uevent_reads_again_only_the_kinds_it_changes() {
    let display = uevent(&[
        "change@/devices/pci0000:00/0000:00:02.0/drm/card0",
        "ACTION=change",
        "SUBSYSTEM=drm",
        "HOTPLUG=1",
    ]);
    assert_eq!(matters(&display), DISPLAYS);
    let card = uevent(&["add@/devices/.../sound/card1", "SUBSYSTEM=sound"]);
    assert_eq!(matters(&card), SOUND);
    let camera = uevent(&[
        "add@/devices/.../video4linux/video1",
        "SUBSYSTEM=video4linux",
    ]);
    assert_eq!(matters(&camera), CAMERAS);
    let plugged = uevent(&["add@/devices/.../usb3/3-4", "SUBSYSTEM=usb"]);
    assert_eq!(matters(&plugged), SOUND | CAMERAS);
    let battery = uevent(&["change@/devices/.../BAT0", "SUBSYSTEM=power_supply"]);
    assert_eq!(matters(&battery), 0);
    let mouse = uevent(&[
        "add@/devices/virtual/misc/uhid/0005:25A7:FAA0.0001",
        "SUBSYSTEM=hid",
    ]);
    assert_eq!(matters(&mouse), BLUETOOTH);
    let input = uevent(&["add@/devices/.../input/input42", "SUBSYSTEM=input"]);
    assert_eq!(matters(&input), BLUETOOTH | KEYBOARDS);
}

#[test]
fn sound_is_read_again_as_it_settles_and_then_only_when_heard() {
    let since = Instant::now();
    assert_eq!(
        waiting_for(None, None, since),
        -1,
        "nothing settling waits for events"
    );
    assert_eq!(waiting_for(Some((since, 0)), None, since), 250);
    assert_eq!(waiting_for(Some((since, 3)), None, since), 2000);
    assert_eq!(
        waiting_for(Some((since, 1)), None, since + Duration::from_secs(9)),
        0,
        "one overdue reads at once"
    );
    assert_eq!(
        [
            waiting_for(None, Some(since + RETRY), since),
            waiting_for(Some((since, 3)), Some(since + RETRY), since),
            waiting_for(Some((since, 0)), Some(since + RETRY), since),
            waiting_for(None, Some(since + RETRY), since + Duration::from_secs(4)),
        ],
        [10_000, 2000, 250, 6000],
        "a folder waiting to be watched didn't bound the wait at its retry, or overrode \
         settling's sooner one"
    );
    let mut settling = Some((since, 0));
    let mut reads = 0;
    while settling.is_some() {
        settling = settled_once(settling);
        reads += 1;
    }
    assert_eq!(reads, SETTLING.len());
}

#[test]
fn a_sound_device_opening_or_the_lighting_changing_is_heard() {
    assert!(
        names_one(&event(IN_OPEN, "pcmC1D0p"), "pebble-lights"),
        "a sound device opening"
    );
    assert!(
        names_one(&event(IN_MOVED_TO, "pebble-lights"), "pebble-lights"),
        "the lighting written whole"
    );
    let mut other = event(IN_CLOSE_WRITE, "quickshell-locked");
    other.extend(event(IN_CLOSE_WRITE, "pebble-lights.new"));
    assert!(
        !names_one(&other, "pebble-lights"),
        "neither the lock nor the lighting's copy"
    );
    assert!(
        names_one(&event(IN_Q_OVERFLOW, ""), "pebble-lights"),
        "an overflow, which may have hidden either"
    );
}
