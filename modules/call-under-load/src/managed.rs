//! Keeps the test call's browsers out of anyone's management. A managed
//! machine's Google Chrome enrols every fresh profile with its owner's
//! browser management and force-installs the extensions it names, so an
//! hourly test on it would put two more of Joel's browsers on the owner's
//! books each hour. The call runs on Chromium, which reads its own machine
//! policy, and that is checked before any browser starts: a call never
//! starts where Chromium's own policy could enrol it or install anything.
//! Should a browser enrol all the same, what it was given is kept and the
//! test stays off until someone has read it, rather than enrolling two
//! more the next hour.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Where Chromium on Linux reads its machine policy.
pub const POLICIES: &str = "/etc/chromium/policies";

/// The policies that enrol a browser or install into it, any of which in
/// Chromium's machine policy keeps the call from starting.
const ENROLLING: [&str; 4] = [
    "CloudManagementEnrollmentToken",
    "ExtensionInstallForcelist",
    "ExtensionSettings",
    "WebAppInstallForceList",
];

/// Why Chromium's machine policy under `root` could enrol a test browser
/// or install into it, if it could: an enrolment token, or any policy
/// file naming one of `ENROLLING`. A file that can't be read counts, since
/// what it holds can't be told.
pub fn could_enrol(root: &Path) -> Option<String> {
    let token = root.join("enrollment");
    if fs::read_dir(&token).is_ok_and(|mut files| files.next().is_some()) {
        return Some(format!("{} holds an enrolment token", token.display()));
    }
    for kind in ["managed", "recommended"] {
        let Ok(files) = fs::read_dir(root.join(kind)) else {
            continue;
        };
        for file in files.flatten() {
            let path = file.path();
            let Ok(text) = fs::read_to_string(&path) else {
                return Some(format!("{} can't be read", path.display()));
            };
            if let Some(policy) = ENROLLING.iter().find(|policy| text.contains(*policy)) {
                return Some(format!("{} sets {policy}", path.display()));
            }
        }
    }
    None
}

/// Where a browser's enrolment is kept once one has enrolled.
pub fn kept(state: &Path) -> PathBuf {
    state.join("call-under-load/enrolled")
}

/// Why the test stays off, if a browser has enrolled: what `keep` wrote.
pub fn stays_off(state: &Path) -> Option<String> {
    fs::read_to_string(kept(state).join("why")).ok()
}

/// Where `keep` puts what `profile` was given: a folder of its own, named
/// as the profile is, so a near and a far end that both enrolled are both
/// kept.
pub fn kept_from(state: &Path, profile: &Path) -> PathBuf {
    let name = profile.file_name().unwrap_or(profile.as_os_str());
    kept(state).join(name)
}

/// Keeps what an enrolled browser's profile was given, its `Policy`
/// folder, so the evidence outlives the profile the call's end removes,
/// and adds why to the reasons the test stays off. The reason is written
/// once the copy is tried, so it says truly where the evidence is: kept
/// beside it, or still in the profile. Ok only when the copy is whole and
/// noted, so a caller removes the profile only then; a profile already
/// kept whole is not copied or noted again.
pub fn keep(state: &Path, profile: &Path) -> Result<String, String> {
    let into = kept_from(state, profile);
    let kept_here = format!(
        "a test browser enrolled with a management service; what it was given is kept in {}",
        into.display()
    );
    if into.join(WHOLE).exists() {
        return Ok(kept_here);
    }
    let whole =
        fs::create_dir_all(&into).is_ok() && copy(&profile.join("Policy"), &into.join("Policy"));
    let why = if whole {
        kept_here
    } else {
        format!(
            "a test browser enrolled with a management service and it couldn't be kept, so \
             its profile stays at {}",
            profile.display()
        )
    };
    let reasons = fs::create_dir_all(kept(state)).and_then(|()| {
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(kept(state).join("why"))
    });
    let noted = reasons.is_ok_and(|mut file| writeln!(file, "{why}").is_ok());
    if whole && noted && fs::write(into.join(WHOLE), "").is_ok() {
        Ok(why)
    } else {
        Err(why)
    }
}

/// Marks a profile's evidence as kept whole and noted, once.
const WHOLE: &str = "whole";

/// Copies the folder `from` to `to`, true only when every file was copied.
fn copy(from: &Path, to: &Path) -> bool {
    let Ok(entries) = fs::read_dir(from) else {
        return false;
    };
    let mut whole = fs::create_dir_all(to).is_ok();
    for entry in entries {
        let Ok(entry) = entry else {
            whole = false;
            continue;
        };
        let (path, into) = (entry.path(), to.join(entry.file_name()));
        whole &= if path.is_dir() {
            copy(&path, &into)
        } else {
            fs::copy(&path, &into).is_ok()
        };
    }
    whole
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("cul-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("the test's folder is made");
        path
    }

    #[test]
    fn only_policy_that_enrols_or_installs_keeps_the_call_off() {
        let root = folder("policies");
        assert_eq!(
            could_enrol(&root),
            None,
            "no policy at all kept the call off"
        );
        fs::create_dir_all(root.join("managed")).expect("managed is made");
        fs::write(
            root.join("managed/extra.json"),
            r#"{"AutoSelectCertificateForUrls": []}"#,
        )
        .expect("a harmless policy is written");
        assert_eq!(
            could_enrol(&root),
            None,
            "a harmless policy kept the call off"
        );
        fs::write(
            root.join("managed/forced.json"),
            r#"{"ExtensionInstallForcelist": ["x"]}"#,
        )
        .expect("a forced extension is written");
        let forced = could_enrol(&root);
        fs::remove_file(root.join("managed/forced.json")).expect("it's removed");
        fs::create_dir_all(root.join("enrollment")).expect("enrollment is made");
        assert_eq!(
            could_enrol(&root),
            None,
            "an empty enrolment folder kept the call off"
        );
        fs::write(root.join("enrollment/CloudManagementEnrollmentToken"), "t")
            .expect("a token is written");
        let token = could_enrol(&root);
        let _ = fs::remove_dir_all(&root);
        assert!(
            forced.is_some_and(|why| why.contains("ExtensionInstallForcelist")),
            "a forced extension let the call start"
        );
        assert!(
            token.is_some_and(|why| why.contains("enrolment token")),
            "an enrolment token let the call start"
        );
    }

    #[test]
    fn an_enrolment_keeps_its_evidence_and_the_test_off() {
        let (state, profile) = (folder("state"), folder("enrolled"));
        assert_eq!(stays_off(&state), None, "a clean state kept the test off");
        fs::create_dir_all(profile.join("Policy/Enrollment")).expect("the folder is made");
        fs::write(profile.join("Policy/Enrollment/token"), "t").expect("the token is written");
        let far = profile.join("far");
        fs::create_dir_all(far.join("Policy/Enrollment")).expect("the far folder is made");
        fs::write(far.join("Policy/Enrollment/token"), "f").expect("the far token is written");
        let near_kept = keep(&state, &profile).is_ok();
        let far_kept = keep(&state, &far).is_ok();
        let again = keep(&state, &profile).is_ok();
        let off = stays_off(&state);
        let token = |of: &Path| {
            fs::read_to_string(kept_from(&state, of).join("Policy/Enrollment/token")).ok()
        };
        let tokens = (token(&profile), token(&far));
        let _ = (fs::remove_dir_all(&state), fs::remove_dir_all(&profile));
        assert!(near_kept && far_kept && again, "keeping said it failed");
        let reasons = off.as_deref().unwrap_or_default().lines().count();
        assert_eq!(
            reasons, 2,
            "each end's reason should be noted once: {off:?}"
        );
        assert_eq!(
            tokens,
            (Some("t".into()), Some("f".into())),
            "an end's enrolment wasn't kept"
        );
    }

    #[test]
    fn a_copy_that_fails_says_it_wasn_t_kept() {
        let (state, profile) = (folder("state-unread"), folder("enrolled-unread"));
        let token = profile.join("Policy/Enrollment/token");
        // A token that's a dangling link can't be copied.
        fs::create_dir_all(profile.join("Policy/Enrollment")).expect("the folder is made");
        std::os::unix::fs::symlink("/nonexistent/token", &token).expect("the link is made");
        let kept_it = keep(&state, &profile);
        let off = stays_off(&state);
        let _ = (fs::remove_dir_all(&state), fs::remove_dir_all(&profile));
        assert!(
            kept_it.is_err_and(|why| why.contains("stays at")),
            "a failed copy was said to be kept"
        );
        assert!(
            off.is_some_and(|why| why.contains("stays at") && !why.contains("kept in")),
            "a failed copy let the test go on, or said it was kept"
        );
    }
}
