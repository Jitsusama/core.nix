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
use std::path::{Path, PathBuf};

/// Where Chromium on Linux reads its machine policy.
pub const POLICIES: &str = "/etc/chromium/policies";

/// The policies that enrol a browser or install into it, any of which in
/// Chromium's machine policy keeps the call from starting.
const ENROLLING: [&str; 3] = [
    "CloudManagementEnrollmentToken",
    "ExtensionInstallForcelist",
    "ExtensionSettings",
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

/// Keeps what an enrolled browser's profile was given, its `Policy`
/// folder, with `why`, so the test stays off and the evidence outlives the
/// profile the call's end removes. Writing is best effort: the `why` goes
/// first, since it alone keeps the test off.
pub fn keep(state: &Path, why: &str, profile: &Path) {
    let into = kept(state);
    if fs::create_dir_all(&into).is_err() {
        return;
    }
    let _ = fs::write(into.join("why"), format!("{why}\n"));
    copy(&profile.join("Policy"), &into.join("Policy"));
}

fn copy(from: &Path, to: &Path) {
    let Ok(entries) = fs::read_dir(from) else {
        return;
    };
    let _ = fs::create_dir_all(to);
    for entry in entries.flatten() {
        let (path, into) = (entry.path(), to.join(entry.file_name()));
        if path.is_dir() {
            copy(&path, &into);
        } else {
            let _ = fs::copy(&path, &into);
        }
    }
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
        keep(&state, "the near end enrolled", &profile);
        let off = stays_off(&state);
        let token = fs::read_to_string(kept(&state).join("Policy/Enrollment/token"));
        let _ = (fs::remove_dir_all(&state), fs::remove_dir_all(&profile));
        assert_eq!(
            off.as_deref(),
            Some("the near end enrolled\n"),
            "the test went on"
        );
        assert_eq!(
            token.ok().as_deref(),
            Some("t"),
            "the enrolment wasn't kept"
        );
    }
}
