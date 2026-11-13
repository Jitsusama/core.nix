//! The broker end to end, as its callers run it, against a stand-in for
//! gcloud that records what it's run with and answers as it's told.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

const BROKER: &str = env!("CARGO_BIN_EXE_gcloud-broker");
const ACCOUNT: &str = "agents@a-lab.iam.gserviceaccount.com";
const TOKEN: &str = "ya29.c.b0-made_by.the.stand-in";

/// A folder of a test's own: a stand-in gcloud, the runtime folder tokens
/// are kept in, and what the stand-in recorded.
struct Place {
    root: PathBuf,
}

impl Place {
    /// A place whose gcloud runs `body` after recording its arguments and
    /// the token variable it was handed, one line each run.
    fn new(body: &str) -> Place {
        static MADE: AtomicUsize = AtomicUsize::new(0);
        let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
            "broker-{}-{}",
            std::process::id(),
            MADE.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(root.join("runtime")).unwrap();
        let gcloud = root.join("gcloud");
        let record = root.join("asked");
        std::fs::write(
            &gcloud,
            format!(
                "#!/bin/sh\necho \"$* token=${{CLOUDSDK_AUTH_ACCESS_TOKEN:-none}}\" >> '{}'\n{body}\n",
                record.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&gcloud, std::fs::Permissions::from_mode(0o755)).unwrap();
        Place { root }
    }

    /// Runs the broker with these arguments, the account and the stand-in,
    /// and the caller's own token in its environment.
    fn ask(&self, args: &[&str]) -> Output {
        Command::new(BROKER)
            .args(args)
            .args(["--account", ACCOUNT, "--gcloud"])
            .arg(self.root.join("gcloud"))
            .env("XDG_RUNTIME_DIR", self.root.join("runtime"))
            .env("CLOUDSDK_AUTH_ACCESS_TOKEN", "ya29.the-callers-own-token")
            .output()
            .unwrap()
    }

    /// Each line the stand-in recorded, one a run.
    fn asked(&self) -> Vec<String> {
        std::fs::read_to_string(self.root.join("asked"))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    /// The folder tokens are kept in.
    fn held(&self) -> PathBuf {
        self.root.join("runtime/gcloud-broker")
    }
}

fn printed(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn complaint(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn an_access_token_is_minted_as_the_account_and_printed_alone() {
    let place = Place::new(&format!("echo {TOKEN}"));
    let output = place.ask(&["access"]);
    assert!(output.status.success(), "{}", complaint(&output));
    assert_eq!(
        printed(&output),
        format!("{TOKEN}\n"),
        "the token alone is printed"
    );
    assert_eq!(
        place.asked(),
        [format!(
            "auth print-access-token --impersonate-service-account={ACCOUNT} \
             --verbosity=error --quiet token=none"
        )],
        "gcloud is asked once, as the account, and never handed the caller's token"
    );
}

#[test]
fn a_token_is_kept_where_only_the_account_reads_it() {
    let place = Place::new(&format!("echo {TOKEN}"));
    for _ in 0..3 {
        let output = place.ask(&["access"]);
        assert_eq!(
            printed(&output),
            format!("{TOKEN}\n"),
            "{}",
            complaint(&output)
        );
    }
    assert_eq!(place.asked().len(), 1, "three asks mint once");
    let folder = std::fs::metadata(place.held())
        .unwrap()
        .permissions()
        .mode();
    let file = std::fs::metadata(place.held().join(format!("{ACCOUNT}.access")))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(folder & 0o777, 0o700, "the folder is the account's alone");
    assert_eq!(
        file & 0o777,
        0o600,
        "the token's file is the account's alone"
    );
}

#[test]
fn a_token_past_its_keeping_is_minted_again() {
    let place = Place::new(&format!("echo {TOKEN}"));
    std::fs::create_dir_all(place.held()).unwrap();
    let hour_ago = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        - 3600;
    std::fs::write(
        place.held().join(format!("{ACCOUNT}.access")),
        format!("{hour_ago} access ya29.an-hour-old-token-now-spent\n"),
    )
    .unwrap();
    let output = place.ask(&["access"]);
    assert_eq!(
        printed(&output),
        format!("{TOKEN}\n"),
        "{}",
        complaint(&output)
    );
    assert_eq!(place.asked().len(), 1, "a spent token is minted again");
}

#[test]
fn an_identity_token_names_its_audience_and_is_kept_apart() {
    // Each identity token names its audience, without the characters a
    // token can't hold, so a token for one can't pass for another's.
    let place = Place::new(&format!(
        "case $2 in print-identity-token) echo {TOKEN}-for-\"${{3#--audiences=}}\" | tr -d :/ ;; \
         *) echo {TOKEN} ;; esac"
    ));
    let run = place.ask(&["identity", "--audience", "https://run.example"]);
    let lab = place.ask(&["identity", "--audience", "https://lab.example"]);
    let access = place.ask(&["access"]);
    assert_eq!(
        printed(&run),
        format!("{TOKEN}-for-httpsrun.example\n"),
        "{}",
        complaint(&run)
    );
    assert_eq!(
        printed(&lab),
        format!("{TOKEN}-for-httpslab.example\n"),
        "{}",
        complaint(&lab)
    );
    assert!(access.status.success(), "{}", complaint(&access));
    assert_eq!(
        place.asked().len(),
        3,
        "each audience and access are minted apart"
    );
    assert!(
        place.asked()[0].starts_with(
            "auth print-identity-token --audiences=https://run.example --include-email "
        ),
        "{:?}",
        place.asked()
    );
}

#[test]
fn a_refusal_says_gclouds_reason_and_keeps_nothing() {
    let place = Place::new(
        "echo 'ERROR: (gcloud.auth.print-access-token) Reauthentication failed.' >&2; exit 1",
    );
    let output = place.ask(&["access"]);
    assert!(!output.status.success(), "a refusal fails");
    assert_eq!(printed(&output), "", "nothing is printed");
    assert!(
        complaint(&output).contains("Reauthentication failed."),
        "{}",
        complaint(&output)
    );
    assert!(
        !place.held().join(format!("{ACCOUNT}.access")).exists(),
        "nothing is kept"
    );
}

#[test]
fn what_isnt_a_token_is_never_printed() {
    let place = Place::new("echo 'ya29.a-token-and then; a command'");
    let output = place.ask(&["access"]);
    assert!(!output.status.success(), "it fails");
    assert_eq!(printed(&output), "", "nothing is printed");
    assert!(
        complaint(&output).contains("without a token"),
        "{}",
        complaint(&output)
    );
}

#[test]
fn the_project_is_the_accounts() {
    let place = Place::new("exit 1");
    let output = place.ask(&["project"]);
    assert_eq!(printed(&output), "a-lab\n", "{}", complaint(&output));
    assert!(place.asked().is_empty(), "gcloud isn't run for it");
}

#[test]
fn an_account_given_twice_is_refused() {
    let place = Place::new(&format!("echo {TOKEN}"));
    let output = place.ask(&["access", "--account", "other@a-lab.iam.gserviceaccount.com"]);
    assert_eq!(output.status.code(), Some(64), "{}", complaint(&output));
    assert!(
        complaint(&output).contains("given twice"),
        "{}",
        complaint(&output)
    );
    assert!(place.asked().is_empty(), "nothing is minted");
}
