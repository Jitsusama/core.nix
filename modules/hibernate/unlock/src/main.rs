//! Lets the lock go after a resume from hibernation, since the disk prompt
//! has just asked for Joel's password, as he chose: one prompt, not two. A
//! wake from sleep keeps its lock. And a hibernation that rolled back, with
//! the machine never off, is never left on in silence.
//!
//! ```text
//! hibernate-unlock pre|post <action> [<disk>...]
//! hibernate-unlock rescue [<why>...]
//! hibernate-unlock fits
//! hibernate-unlock restoring
//! hibernate-unlock lost
//! ```
//!
//! It takes the shape every command of ours does (docs/commands.md),
//! through the terminal kit: `fits` answers in words fitting the terminal,
//! or as `hibernate-unlock.fits/1` with `--json`, and exits 1 when the
//! image wouldn't fit.
//!
//! systemd-sleep runs it before and after each sleep, naming the action
//! (`suspend`, `hibernate`, `hybrid-sleep` or `suspend-then-hibernate`), and
//! the disks are those the initrd opens. Before anything that writes an
//! image it keeps the TPM's reset and restart counts in `/run`, which the
//! image keeps too, and the number of the kernel's last log record. After,
//! it lets every session's lock go only when both hold (lock.rs):
//!
//! - A count went up, so the firmware started the machine: the image was
//!   read back, through the initrd, rather than a hybrid sleep waking from
//!   its sleep. Linux tells the TPM to save its state going down, so the
//!   firmware's start counts as a restart, not a reset; a wake from sleep
//!   moves neither.
//! - Every disk can only be opened with something typed: each TPM token
//!   asks for a PIN, each FIDO2 token for its PIN, a recovery key is typed,
//!   and a keyslot with no token is a password. Any other token keeps the
//!   lock, and the log names it. A disk the TPM opens alone would let anyone
//!   through, and with no disk named nothing is let go.
//!
//! It asks `tpm2_readclock` for the counts, `cryptsetup` for each disk's
//! tokens and `loginctl` to let the locks go, each from `PATH`. Anything it
//! can't read keeps the lock, and it never fails the sleep.
//!
//! A hibernation back with neither count moved rolled back: the kernel
//! couldn't make the image, couldn't write it, or a device refused to power
//! off once it was written, when the kernel tells systemd it worked. The
//! kernel's log since the sleep began says which (kmsg.rs, why.rs), and the
//! rescue is told. Ten seconds later, once the sleep is over, `rescue` runs
//! on its own (rescue.rs). With the lid open, a notice tells whoever is
//! signed in why, that nothing was lost, and what to do (notice.rs). With
//! the lid shut nobody is looking, and a laptop left on in a bag runs flat
//! and hot. Short of memory or swap, a second try fails the same way and
//! then powers off, losing what's open, so the laptop goes to sleep
//! instead, the notice waiting for the lid to open. Otherwise it hibernates
//! once more, and powers off if that rolls back too.
//!
//! Before an image is made it sets the hard memory floors aside, and puts
//! them back after, or the kernel can't take back what they guard to make
//! room (floors.rs). It switches zswap off too, and on again after, or
//! what's swapped out for the image can stay in memory (zswap.rs). `fits`
//! says whether the image would fit, by the kernel's sums, before anything
//! is asked to hibernate (fits.rs), and exits 1 when it wouldn't. When the
//! machine starts another kernel, initrd or kernel parameters by default
//! than the running system did, it sets the next start, once, to the boot
//! entry this one came from, so the image is read back by the kernel that
//! wrote it, and unsets it after (oneshot.rs).
//!
//! `restoring` runs in the initrd just before the image is read, keeping
//! when in an EFI variable, so that once the machine is back the hook says
//! how long the restore took (restored.rs).
//!
//! `lost` runs at each start, and when the last image written never came
//! back, tells whoever signs in, once, that their work couldn't come back
//! and why (lost.rs).

mod fits;
mod floors;
mod kmsg;
mod lock;
mod lost;
mod notice;
mod oneshot;
mod rescue;
mod restored;
mod why;
mod zswap;

use notice::{Notice, say_to_each_session, signed_in};
use serde_json::{Value, json};
use std::fs;
use std::path::Path;
use std::process::{Command, ExitCode};
use terminal_kit::{Exit, Failure, Help};
use why::Why;

/// Where the counts from before the image was written are kept.
const KEPT: &str = "/run/hibernate-unlock";

/// Where the hard memory floors set aside for an image are kept until
/// they're put back.
const FLOORS: &str = "/run/hibernate-unlock-floors";

/// Where the cgroups are, the floors in them.
const CGROUPS: &str = "/sys/fs/cgroup";

/// Kept while zswap is switched off for an image.
const ZSWAP_OFF: &str = "/run/hibernate-unlock-zswap";

/// Kept while the next start is held to this one's boot entry.
const ONESHOT: &str = "/run/hibernate-unlock-oneshot";

/// Whether a sleep writes an image the machine can start again from.
fn writes_an_image(action: &str) -> bool {
    matches!(
        action,
        "hibernate" | "hybrid-sleep" | "suspend-then-hibernate"
    )
}

/// What a command printed, or nothing if it couldn't run or failed.
fn said(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Set the next start's boot entry, once, with `bootctl set-oneshot`, or
/// unset it given nothing. Says whether it did.
fn set_oneshot(entry: &str) -> bool {
    said("bootctl", &["set-oneshot", entry]).is_some()
}

/// Before an image is written, keep the TPM's counts and the kernel log's
/// place. Before a sleep that writes none, forget any kept from a hibernate
/// that never happened.
fn before(action: &str) {
    let _ = fs::remove_file(KEPT);
    let _ = fs::remove_file(kmsg::LOGGED);
    if !writes_an_image(action) {
        return;
    }
    let boot = fs::read_to_string(lost::BOOT_ID).unwrap_or_default();
    if let Err(error) = lost::keep(Path::new(lost::MARK), &boot) {
        eprintln!("hibernate-unlock: couldn't mark the image as written: {error}");
    }
    match floors::lift(Path::new(CGROUPS), Path::new(FLOORS)) {
        Ok(0) => {}
        Ok(lifted) => eprintln!("hibernate-unlock: set {lifted} memory floors aside for the image"),
        Err(error) => eprintln!("hibernate-unlock: couldn't set the memory floors aside: {error}"),
    }
    match zswap::set_aside(Path::new(zswap::ENABLED), Path::new(ZSWAP_OFF)) {
        Ok(false) => {}
        Ok(true) => eprintln!("hibernate-unlock: switched zswap off for the image"),
        Err(error) => eprintln!("hibernate-unlock: couldn't switch zswap off: {error}"),
    }
    let (booted, default) = (Path::new(oneshot::BOOTED), Path::new(oneshot::DEFAULT));
    match oneshot::hold(booted, default, Path::new(ONESHOT), set_oneshot) {
        Ok(false) => {}
        Ok(true) => eprintln!(
            "hibernate-unlock: the default system starts another kernel or initrd, so the \
             next start takes this one's entry, once, to read the image back"
        ),
        Err(error) => {
            eprintln!("hibernate-unlock: couldn't hold the next start to its entry: {error}");
        }
    }
    // What the graphics drivers hold as the image is sized: they copy it
    // out as they stop, and the reserve has to leave room for that.
    for card in graphics_memory() {
        eprintln!("hibernate-unlock: {card}");
    }
    match lock::starts() {
        Some(now) => {
            if let Err(error) = fs::write(KEPT, now.kept()) {
                eprintln!("hibernate-unlock: couldn't keep the TPM's counts: {error}");
            }
        }
        None => eprintln!("hibernate-unlock: couldn't read the TPM's counts; the lock stays"),
    }
    match kmsg::kernel_log().last() {
        Some(last) => {
            if let Err(error) = fs::write(kmsg::LOGGED, last.number.to_string()) {
                eprintln!("hibernate-unlock: couldn't keep the kernel log's place: {error}");
            }
        }
        None => eprintln!("hibernate-unlock: couldn't read the kernel's log"),
    }
}

/// After the machine is back, let the locks go if it came back through the
/// disk prompt, and rescue a hibernation that never went off.
fn after(action: &str, disks: &[String]) {
    if let Err(error) = lost::forget(Path::new(lost::MARK)) {
        eprintln!("hibernate-unlock: couldn't take the image's mark away: {error}");
    }
    match floors::restore(Path::new(CGROUPS), Path::new(FLOORS)) {
        Ok(0) => {}
        Ok(restored) => eprintln!("hibernate-unlock: put {restored} memory floors back"),
        Err(error) => eprintln!("hibernate-unlock: couldn't put the memory floors back: {error}"),
    }
    match zswap::put_back(Path::new(zswap::ENABLED), Path::new(ZSWAP_OFF)) {
        Ok(false) => {}
        Ok(true) => eprintln!("hibernate-unlock: switched zswap on again"),
        Err(error) => eprintln!("hibernate-unlock: couldn't switch zswap on again: {error}"),
    }
    match oneshot::let_go(Path::new(ONESHOT), set_oneshot) {
        Ok(false) => {}
        Ok(true) => eprintln!("hibernate-unlock: the next start takes the default system again"),
        Err(error) => eprintln!("hibernate-unlock: couldn't let the next start go: {error}"),
    }
    let logged = kmsg::logged_since_before();
    let _ = fs::remove_file(kmsg::LOGGED);
    let Some(then) = fs::read_to_string(KEPT)
        .ok()
        .and_then(|t| lock::Starts::unkept(&t))
    else {
        return;
    };
    let _ = fs::remove_file(KEPT);
    let Some(now) = lock::starts() else {
        eprintln!("hibernate-unlock: couldn't read the TPM's counts; the lock stays");
        return;
    };
    if !then.started_since(now) {
        if action == "hibernate" {
            rescue::rolled_back(&Why::from_log(&logged));
        } else {
            eprintln!(
                "hibernate-unlock: woke without a start, so through no prompt; the lock stays"
            );
        }
        return;
    }
    let _ = fs::remove_file(rescue::TRIED_AGAIN);
    say_how_long_the_restore_took();
    if disks.is_empty() {
        eprintln!("hibernate-unlock: no disk asked at the prompt; the lock stays");
        return;
    }
    if let Some((disk, why)) = disks
        .iter()
        .find_map(|disk| lock::disk_unasked(disk).map(|why| (disk, why)))
    {
        eprintln!("hibernate-unlock: {disk}: {why}; the lock stays");
        return;
    }
    match said("loginctl", &["unlock-sessions"]) {
        Some(_) => eprintln!("hibernate-unlock: back through the disk prompt; the lock is let go"),
        None => eprintln!("hibernate-unlock: loginctl couldn't let the lock go"),
    }
}

/// Says how long the restore took, when this start's initrd kept when it
/// began.
fn say_how_long_the_restore_took() {
    let variable = Path::new(restored::VARIABLE);
    let Some(then) = restored::kept(variable) else {
        eprintln!("hibernate-unlock: the initrd kept no count, so how long it took is unsaid");
        return;
    };
    // The firmware keeps it across starts, so a later restore whose initrd
    // couldn't keep one would read this one.
    if let Err(error) = restored::forget(variable) {
        eprintln!("hibernate-unlock: couldn't remove the restore's count: {error}");
    }
    let said = restored::counted()
        .zip(restored::rate())
        .and_then(|(now, rate)| restored::said(then, now, rate));
    match said {
        Some(said) => eprintln!("hibernate-unlock: {said}"),
        None => eprintln!("hibernate-unlock: couldn't count how long the restore took"),
    }
}

/// Keeps when the restore began, in the initrd, never failing it.
fn restoring() {
    let Some(now) = restored::counted() else {
        eprintln!("hibernate-unlock: no counter to keep when the restore began");
        return;
    };
    if let Err(error) = restored::keep(Path::new(restored::VARIABLE), now) {
        eprintln!("hibernate-unlock: couldn't keep when the restore began: {error}");
    }
}

/// At a start, when the last image written never came back, says why at
/// once, then tells each seated user once a graphical session is up, and
/// takes the mark away. It waits as long as the start runs, and stops
/// waiting should a new hibernation's return take the mark first.
fn tell_of_a_lost_image() {
    let mark = Path::new(lost::MARK);
    let boot = fs::read_to_string(lost::BOOT_ID).unwrap_or_default();
    if !lost::lost(mark, &boot) {
        return;
    }
    let logged: Vec<String> = said("journalctl", &["--boot", "--output=cat", "--no-pager"])
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect();
    let why = lost::Why::from_log(&logged);
    eprintln!(
        "hibernate-unlock: the last hibernation's image never came back: {}",
        why.word()
    );
    while mark.exists() && !signed_in() {
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
    if !mark.exists() {
        return;
    }
    // The session's own services, its notices among them, start just
    // after it does.
    std::thread::sleep(std::time::Duration::from_secs(5));
    let (title, body) = lost::said(why);
    say_to_each_session(&Notice { title, body });
    if let Err(error) = lost::forget(mark) {
        eprintln!("hibernate-unlock: couldn't take the image's mark away: {error}");
    }
}

/// Says whether an image would fit now, exiting 1 when it wouldn't. What
/// it can't read it doesn't stand in the way of.
/// Says whether the image would fit, as words or as
/// `hibernate-unlock.fits/1`, failing when it wouldn't.
fn fits_now(context: &terminal_kit::Context) -> Result<(), Failure> {
    let read = |path: &str| fs::read_to_string(path).ok();
    let memory = read("/proc/meminfo").and_then(|text| fits::Memory::parse(&text));
    let reserve = read("/sys/power/reserved_size").and_then(|text| text.trim().parse().ok());
    let swaps = read("/proc/swaps");
    let zones = read("/proc/zoneinfo");
    let (Some(memory), Some(reserve), Some(swaps), Some(zones)) = (memory, reserve, swaps, zones)
    else {
        return say(
            context,
            &json!({"schema": "hibernate-unlock.fits/1", "fits": null}),
            "It couldn't tell whether the image fits, so it doesn't stand in the way.",
        );
    };
    let fit = fits::Fit::of(
        memory,
        fits::Zones::parse(&zones),
        reserve,
        fits::swap_room(&swaps),
    );
    let facts = json!({
        "schema": "hibernate-unlock.fits/1",
        "fits": fit.fits(),
        "to_save_bytes": fit.least,
        "room_bytes": fit.most,
    });
    if fit.fits() {
        return say(context, &facts, &fit.said());
    }
    if context.format().is_json() {
        context.answer(&facts).map_err(|error| unsaid(&error))?;
    }
    Err(Failure::new(Exit::Failed, "there isn't enough memory to hibernate").because(fit.short()))
}

/// Says `facts` with `--json`, else `words` set to the terminal's measure.
fn say(context: &terminal_kit::Context, facts: &Value, words: &str) -> Result<(), Failure> {
    let written = if context.format().is_json() {
        context.answer(facts)
    } else {
        let lines = terminal_kit::layout::paragraph(words, 0, context.room.for_prose());
        context.say(&(lines.join("\n") + "\n"))
    };
    written.map_err(|error| unsaid(&error))
}

fn unsaid(error: &std::io::Error) -> Failure {
    Failure::new(Exit::Failed, "the answer couldn't be said").because(error.to_string())
}

/// The video memory and the system memory it maps (GTT) in use on each card
/// whose driver says, one line apiece. amdgpu says; others, xe among them,
/// log nothing.
fn graphics_memory() -> Vec<String> {
    let mut cards: Vec<String> = fs::read_dir("/sys/class/drm")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|card| {
            let name = card.file_name().into_string().ok()?;
            let device = card.path().join("device");
            let read = |file: &str| fs::read_to_string(device.join(file)).ok();
            let vram = read("mem_info_vram_used")?;
            let gtt = read("mem_info_gtt_used")?;
            (!name.contains('-')).then(|| {
                format!(
                    "{name} holds {} bytes of video memory and {} of GTT",
                    vram.trim(),
                    gtt.trim()
                )
            })
        })
        .collect();
    cards.sort();
    cards
}

fn help() -> Help {
    Help::new(
        "hibernate-unlock",
        "lets the lock go after a resume from hibernation",
    )
    .example(
        "hibernate-unlock fits",
        "whether memory's image would fit now",
    )
    .subcommand(
        Help::new("fits", "says whether a hibernation's image would fit")
            .example(
                "hibernate-unlock fits",
                "whether memory's image would fit now",
            )
            .example("hibernate-unlock fits --json", "the same, as JSON")
            .exit(Exit::Failed, "the image wouldn't fit"),
    )
    .subcommand(sleeps("pre", "what systemd-sleep runs before a sleep"))
    .subcommand(sleeps("post", "what systemd-sleep runs after a sleep"))
    .subcommand(
        Help::new("rescue", "tells of a hibernation that rolled back")
            .argument("[why...]", "memory, swap-gone, device DRIVER...")
            .example("hibernate-unlock rescue memory", "memory ran short")
            .exit(Exit::Done, NEVER_FAILS),
    )
    .subcommand(
        Help::new(
            "restoring",
            "what the initrd runs before it reads the image back",
        )
        .example("hibernate-unlock restoring", "what the initrd runs")
        .exit(Exit::Done, NEVER_FAILS),
    )
    .subcommand(
        Help::new(
            "lost",
            "tells whoever signs in of an image that never came back",
        )
        .example("hibernate-unlock lost", "what each start runs")
        .exit(Exit::Done, NEVER_FAILS),
    )
    .exit(Exit::Failed, "the image wouldn't fit")
}

/// What a step that never fails what it runs within says of its exit.
const NEVER_FAILS: &str = "always; what it found goes to the journal";

/// One of the steps systemd-sleep runs around a sleep, which logs what it
/// found and never fails the sleep.
fn sleeps(when: &str, about: &str) -> Help {
    Help::new(when, about)
        .argument("<action>", "the sleep, as hibernate or suspend")
        .argument("[disk...]", "the encrypted disks the lock asks for")
        .example(
            &format!("hibernate-unlock {when} hibernate"),
            "what systemd-sleep runs",
        )
        .exit(Exit::Done, NEVER_FAILS)
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        // Each of the sleep's own steps logs what it found and never fails
        // the sleep, so only `fits` answers with a failing exit.
        match context.args.as_slice() {
            [when, action, ..] if when == "pre" => before(action),
            [when, action, disks @ ..] if when == "post" => after(action, disks),
            [what, why @ ..] if what == "rescue" => rescue::rescue_now(&Why::from_args(why)),
            [what] if what == "fits" => return fits_now(context),
            [what] if what == "restoring" => restoring(),
            [what] if what == "lost" => tell_of_a_lost_image(),
            _ => {
                return Err(Failure::new(Exit::Usage, "that isn't one of its commands")
                    .try_this("hibernate-unlock --help"));
            }
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::{help, writes_an_image};
    use crate::why::tests::{cases, lines};

    /// The words a rollback hands the rescue, and the disks the sleep
    /// hands `post`, are each taken by the command line, never refused as
    /// one too many: a refused rescue never retries or powers off.
    #[test]
    fn the_command_line_takes_every_why_and_every_disk_its_callers_give() {
        let told = |words: Vec<String>| {
            terminal_kit::tree::walk(&help(), &words)
                .map(|walked| walked.path())
                .map_err(|failure| failure.what)
        };
        for (why, _) in cases() {
            let words = std::iter::once("rescue".to_owned())
                .chain(why.args())
                .collect();
            assert_eq!(
                told(words),
                Ok("hibernate-unlock rescue".to_owned()),
                "{why:?} was refused"
            );
        }
        let disks = lines(&["post", "hibernate", "root", "home"]);
        assert_eq!(
            told(disks),
            Ok("hibernate-unlock post".to_owned()),
            "a second disk was refused"
        );
    }

    #[test]
    fn only_sleeps_that_write_an_image_are_kept() {
        for action in ["hibernate", "hybrid-sleep", "suspend-then-hibernate"] {
            assert!(writes_an_image(action), "{action}");
        }
        assert!(
            !writes_an_image("suspend"),
            "a suspend was taken for a sleep that writes an image"
        );
    }
}
