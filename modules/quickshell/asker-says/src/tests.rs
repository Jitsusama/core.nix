//! What a process says, over a stand-in /proc of processes written as each
//! case needs, and once over the real one, of a child started with a secret
//! beside its quest (ISXEG8's terms, 2026-11-15).

use super::*;
use std::process::{self, Command, Stdio};

const QUEST_ID: &str = "QEST-20261110-H6EDLC";
const SESSION_ID: &str = "0f3a9c2e-4b7d-4e1a-9c3f-a1b2c3d4e5f6";
const SECRET: &str = "sk-proxy-5ecret-never-said";

/// A stand-in /proc, removed when the case ends.
struct Proc(PathBuf);

impl Proc {
    fn new(case: &str) -> Proc {
        let dir = env::temp_dir().join(format!("asker-says-{case}-{}", process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Proc(dir)
    }

    /// A process, its name, its parent and its environment's entries.
    fn process(&self, pid: u32, name: &str, up: u32, environ: &[&str]) -> &Proc {
        let dir = self.0.join(pid.to_string());
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("stat"),
            format!("{pid} ({name}) S {up} {pid} {pid} 0 -1\n"),
        )
        .unwrap();
        let mut bytes = Vec::new();
        for entry in environ {
            bytes.extend_from_slice(entry.as_bytes());
            bytes.push(0);
        }
        fs::write(dir.join("environ"), bytes).unwrap();
        self
    }

    /// The stand-in quests' folder, beside the processes, where no pid is.
    fn quests(&self) -> PathBuf {
        self.0.join("quests")
    }

    /// A quest's README, in its folder by its id.
    fn quest(&self, id: &str, readme: &str) -> &Proc {
        let dir = self.quests().join(id);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("README.md"), readme).unwrap();
        self
    }

    /// What `pid` says, over these processes and quests.
    fn says(&self, pid: u32) -> Says {
        says(&self.0, &self.quests(), pid)
    }
}

impl Drop for Proc {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn quest() -> String {
    format!("{QUEST}={QUEST_ID}")
}

fn session() -> String {
    format!("{SESSION}={SESSION_ID}")
}

fn secret() -> String {
    format!("PI_PROXY_API_KEY={SECRET}")
}

#[test]
fn says_the_quest_and_session_a_process_holds() {
    let proc = Proc::new("holds");
    proc.process(
        40,
        "run0",
        30,
        &["HOME=/home/someone", &quest(), &session()],
    )
    .quest(QUEST_ID, "---\nid: x\n---\n# Launcher Speed\n");
    assert_eq!(
        proc.says(40),
        Says {
            quest: Some(QUEST_ID.to_owned()),
            session: Some(SESSION_ID.to_owned()),
            by: Some(40),
            title: Some("Launcher Speed".to_owned()),
        }
    );
}

#[test]
fn names_a_quest_by_its_short_title_else_its_heading() {
    let short = "---\nid: x\nshort: Launcher\n---\n# A Launcher That Fits\n";
    assert_eq!(title(short).as_deref(), Some("Launcher"));
    let blank = "---\nshort: \"\"\n---\n# The Loop\n";
    assert_eq!(title(blank).as_deref(), Some("The Loop"));
    let yaml = "---\nid: x\n# not a heading in yaml\n---\n\n# The Loop\n";
    assert_eq!(title(yaml).as_deref(), Some("The Loop"));
    assert_eq!(title("no heading"), None);
    assert_eq!(title("# \n"), None);
}

#[test]
fn a_quest_with_no_readme_says_no_title() {
    let proc = Proc::new("untitled");
    proc.process(40, "run0", 30, &[&quest()]);
    let said = proc.says(40);
    assert_eq!(said.quest.as_deref(), Some(QUEST_ID));
    assert_eq!(said.title, None);
}

#[test]
fn reads_no_readme_for_a_quest_out_of_shape() {
    let proc = Proc::new("climbs");
    let climbing = format!("{QUEST}=../quests/{QUEST_ID}");
    proc.process(40, "run0", 30, &[&climbing])
        .quest(QUEST_ID, "# Launcher Speed\n");
    assert_eq!(proc.says(40), Says::default());
}

#[test]
fn never_says_anything_beside_the_two_names() {
    let proc = Proc::new("secret");
    let quested_secret = format!("PI_QUEST_KEY={SECRET}");
    let misshapen = format!("{QUEST}={SECRET}");
    proc.process(40, "run0", 30, &[&secret(), &quested_secret, &session()])
        .process(41, "run0", 30, &[&secret(), &misshapen]);
    for pid in [40, 41, 42] {
        let said = proc.says(pid);
        let answered = said.answer().to_string();
        assert!(
            !answered.contains(SECRET),
            "{pid}'s answer said the secret: {answered}"
        );
        assert!(
            !said.said().contains(SECRET),
            "{pid}'s words said the secret"
        );
    }
    assert_eq!(proc.says(41), Says::default());
}

#[test]
fn asks_each_parent_up_to_the_first_that_says() {
    let proc = Proc::new("parents");
    proc.process(10, MANAGER, 1, &[&quest()])
        .process(20, "pi", 10, &[&quest(), &session()])
        .process(30, "bash", 20, &[&quest(), &session()])
        .process(35, "sh", 30, &["PATH=/bin"])
        .process(40, "run0", 35, &[&secret()]);
    assert_eq!(proc.says(40).by, Some(30));
}

#[test]
fn stops_at_the_accounts_manager() {
    let proc = Proc::new("manager");
    proc.process(10, MANAGER, 1, &[&quest(), &session()])
        .process(20, "niri", 10, &["PATH=/bin"])
        .process(40, "pkexec", 20, &["PATH=/bin"]);
    assert_eq!(proc.says(40), Says::default());
}

#[test]
fn stops_at_a_process_it_cant_read() {
    let proc = Proc::new("unread");
    proc.process(20, "sudo", 10, &[&quest()])
        .process(30, "bash", 20, &["PATH=/bin"])
        .process(40, "run0", 30, &["PATH=/bin"]);
    fs::remove_file(proc.0.join("30").join("environ")).unwrap();
    assert_eq!(proc.says(40), Says::default());
}

#[test]
fn a_process_gone_says_nothing() {
    let proc = Proc::new("gone");
    assert_eq!(proc.says(40), Says::default());
    assert_eq!(proc.says(40).said(), "it says no quest or session\n");
}

#[test]
fn reads_a_name_holding_spaces_and_brackets() {
    assert_eq!(
        parent("40 (a (b) c) S 30 40 40 0 -1"),
        Some(("a (b) c", 30))
    );
    assert_eq!(parent("40 (run0"), None);
}

#[test]
fn says_only_values_in_the_harness_shape() {
    assert!(quest_shaped(QUEST_ID));
    assert!(session_shaped(SESSION_ID));
    for words in [
        "QEST-20261110-H6EDLC verified by joel",
        "QEST-2026111-H6EDLC",
        "QEST-20261110-h6edlc",
        "QEST-20261110-H6EDLC\n",
        "",
    ] {
        assert!(!quest_shaped(words), "{words:?} read as a quest");
    }
    for words in [
        "0F3A9C2E-4B7D-4E1A-9C3F-A1B2C3D4E5F6",
        "0f3a9c2e-4b7d-4e1a-9c3f",
        "0f3a9c2e-4b7d-4e1a-9c3f-a1b2c3d4e5fz",
    ] {
        assert!(!session_shaped(words), "{words:?} read as a session");
    }
}

#[test]
fn reads_a_real_process_and_none_of_its_secret() {
    let mut child = Command::new("sleep")
        .arg("30")
        .env_clear()
        .env("PATH", env::var_os("PATH").unwrap_or_default())
        .env("PI_PROXY_API_KEY", SECRET)
        .env(QUEST, QUEST_ID)
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    let quests = Proc::new("real");
    quests.quest(QUEST_ID, "# Launcher Speed\n");
    let said = says(Path::new("/proc"), &quests.quests(), child.id());
    child.kill().unwrap();
    child.wait().unwrap();
    assert_eq!(said.quest.as_deref(), Some(QUEST_ID));
    assert_eq!(said.session, None);
    assert_eq!(said.title.as_deref(), Some("Launcher Speed"));
    assert!(!said.answer().to_string().contains(SECRET));
}
