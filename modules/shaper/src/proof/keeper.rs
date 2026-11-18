//! The keeper beside a proof: a scope of its own that outlives the
//! proof, however it goes, and puts the shaper back as the marker a
//! window claimed says, so a proof killed outright leaves the shaper as
//! it found it within about a second.

use super::stop::tell;
use std::path::Path;
use std::process::Stdio;
use std::thread;
use std::time::Duration;

/// Starts a keeper beside this proof, in a scope of its own named apart
/// from the proof's, which the proof's stops leave alone. Once the proof
/// is gone, however it went, the keeper puts the shaper back as `marker`
/// says and forgets it. A proof that put it back has forgotten the
/// marker already, and its keeper does nothing.
pub fn keep(marker: &Path) {
    let Some(at) = marker.to_str() else {
        return;
    };
    let me = std::process::id();
    let script = keeper(me, &super::load::began());
    let unit = format!("shaper-keeper-{me}");
    let command = ["sh", "-c", &script, "sh", at, "systemctl"];
    let unkept = "a proof killed outright would leave the shaper off";
    match super::scoped_as("app.slice", &unit, &command)
        .stdout(Stdio::null())
        .spawn()
    {
        Err(_) => tell(&format!("couldn't start the keeper: {unkept}")),
        // It ends only once the proof is gone, so one ended already is
        // systemd-run refusing the scope.
        Ok(mut keeper) => {
            thread::sleep(Duration::from_millis(300));
            if keeper.try_wait().is_ok_and(|ended| ended.is_some()) {
                tell(&format!("the keeper's scope was refused: {unkept}"));
            }
        }
    }
}

/// The keeper's shell for the proof `pid` that began at `began`: given
/// the marker and systemctl as its arguments, it waits for the proof to
/// go, then starts or stops the shaper as the marker says, and forgets
/// it once that's done, or, failing, marks it as written now, so the
/// next proof's 30 minutes start from here. A marker saying neither is
/// nobody's word, and one another proof wrote is that proof's, which
/// this one's keeper leaves: read again before it's forgotten, since the
/// next proof may have written its own meanwhile.
fn keeper(pid: u32, began: &str) -> String {
    let mine = format!("v=; who=; read -r v who 2>/dev/null <\"$1\"; [ \"$who\" = {pid} ]");
    super::load::once_gone_for(
        pid,
        began,
        &format!(
            "{mine} || exit 0; case $v in on) v=start;; off) v=stop;; *) exit 0;; esac; \
             if \"$2\" --no-ask-password \"$v\" shaper.service; then {mine} && rm -f \"$1\"; \
             else {mine} && touch \"$1\"; exit 1; fi"
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::super::stop::forget;
    use super::super::stop::tests::{age, aged};
    use super::*;
    use std::fs;
    use std::process::Command;

    #[test]
    fn the_keeper_puts_the_shaper_back_once_its_proof_is_gone() {
        let dir = std::env::temp_dir().join(format!("shaper-keeper-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("a folder");
        let marker = dir.join("was-on");
        let at = marker.to_str().expect("a path");
        // What it asks, with echo standing in for systemctl.
        let kept = |pid: u32, began: &str| {
            let out = Command::new("sh")
                .args(["-c", &keeper(pid, began), "sh", at, "echo"])
                .output()
                .expect("a shell");
            String::from_utf8_lossy(&out.stdout).into_owned()
        };
        let mut gone = Command::new("true").spawn().expect("true");
        let pid = gone.id();
        let began = super::super::load::began_of(&pid.to_string());
        gone.wait().expect("true to end");
        for (was, asks) in [
            ("on", "--no-ask-password start shaper.service\n"),
            ("off", "--no-ask-password stop shaper.service\n"),
        ] {
            fs::write(&marker, format!("{was} {pid}\n")).expect("a marker");
            assert_eq!(kept(pid, &began), asks, "the shaper wasn't put back {was}");
            assert!(!marker.exists(), "the marker outlived putting it back");
        }
        // The next proof's marker, written once this proof was gone, is
        // that proof's to put back, with a keeper of its own.
        let next = pid + 1;
        fs::write(&marker, format!("off {next}\n")).expect("a marker");
        assert_eq!(kept(pid, &began), "", "acted on the next proof's marker");
        assert!(marker.exists(), "forgot the next proof's marker");
        // And one the next proof writes while this keeper's systemctl runs
        // is read again, and left.
        fs::write(&marker, format!("on {pid}\n")).expect("a marker");
        let racing = format!(
            "m=$1; next() {{ echo \"off {next}\" >\"$m\"; }}; {}",
            keeper(pid, &began)
        );
        Command::new("sh")
            .args(["-c", &racing, "sh", at, "next"])
            .status()
            .expect("a shell");
        assert_eq!(
            fs::read_to_string(&marker).ok(),
            Some(format!("off {next}\n")),
            "forgot the marker the next proof wrote meanwhile"
        );
        // A shaper that wouldn't go back keeps its marker, for the next,
        // marked as written now, as its proof died.
        fs::write(&marker, format!("on {pid}\n")).expect("a marker");
        age(&marker, Duration::from_mins(20));
        let failing = Command::new("sh")
            .args(["-c", &keeper(pid, &began), "sh", at, "false"])
            .status()
            .expect("a shell");
        assert!(!failing.success(), "a keeper that failed said it hadn't");
        assert!(marker.exists(), "forgot a shaper it couldn't put back");
        assert!(
            aged(&marker) < Duration::from_mins(1),
            "aged the debt from its window, not its death"
        );
        forget(&marker);
        // A proof that put it back left nothing to do.
        assert_eq!(kept(pid, &began), "", "acted on no marker");
        fs::write(&marker, "").expect("an empty marker");
        assert_eq!(kept(pid, &began), "", "took an empty marker for a state");
        let _ = fs::remove_dir_all(&dir);
    }
}
