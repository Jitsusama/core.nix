use super::*;
use crate::screen::plain;

fn theme() -> Theme {
    Theme::from(
        "text=37 strong=97 muted=90 accent=34 alert=31 warning=93 success=32 action=93 machine=91 voice=36",
        "cursor ▸ >\ndone ✓ v\nfailed ✕ x\ncaution ! !\nlocked L %\nhidden • *\nrunning → >",
        true,
    )
}

const DONE: &[String] = &[];

fn history() -> Vec<String> {
    vec![
        "the keyboard types Colemak".into(),
        "the vault is open, holding 12 files".into(),
    ]
}

fn at() -> Where<'static> {
    Where {
        journey: "Install hedgehog",
        step: 3,
        steps: Some(13),
        done: DONE,
    }
}

fn text(drawn: &Drawn) -> Vec<String> {
    drawn
        .lines
        .iter()
        .map(plain)
        .map(|line| line.trim_end().to_string())
        .collect()
}

#[test]
fn a_secret_shows_its_name_and_only_its_length() {
    let ask = Ask::Secret {
        id: "pin",
        name: "the Nano's PIN".into(),
    };
    let moment = Moment::Asking {
        title: "Enrol a YubiKey",
        ask: &ask,
        typed: "123456",
        chosen: 0,
        refused: None,
        back: false,
    };
    let drawn = draw(&theme(), &at(), &moment, 80, 24);
    let shown = text(&drawn);
    assert!(
        shown.contains(&"  Type the Nano's PIN, then press Return.".to_string()),
        "{shown:#?}"
    );
    assert!(shown.contains(&"  % ******".to_string()), "{shown:#?}");
    assert!(
        !shown.concat().contains("123456"),
        "the PIN showed as typed: {shown:#?}"
    );
    assert_eq!(
        drawn.caret,
        Some((
            u16::try_from(
                shown
                    .iter()
                    .position(|line| line.contains("******"))
                    .unwrap()
            )
            .unwrap(),
            10
        ))
    );
    assert_eq!(shown.last().unwrap(), "  Return enter it    Ctrl+C leave");
    assert_eq!(shown.len(), 23, "the keys sit a line above the foot");
}

#[test]
fn a_confirmation_says_what_return_does() {
    let ask = Ask::Confirm {
        id: "stick",
        what: "The stick is the one to write.".into(),
        verb: "write it",
    };
    let moment = Moment::Asking {
        title: "Write the stick",
        ask: &ask,
        typed: "",
        chosen: 0,
        refused: None,
        back: false,
    };
    let shown = text(&draw(&theme(), &at(), &moment, 80, 24));
    assert!(
        shown
            .iter()
            .any(|line| line.contains("The stick is the one to write. Press Return to write it.")),
        "{shown:#?}"
    );
    assert_eq!(shown.last().unwrap(), "  Return write it    Ctrl+C leave");
}

#[test]
fn the_header_says_where_and_how_far_along() {
    let ask = Ask::Act {
        id: "plugged",
        what: "Plug in the Nano.".into(),
    };
    let moment = Moment::Asking {
        title: "Enrol a YubiKey",
        ask: &ask,
        typed: "",
        chosen: 0,
        refused: None,
        back: false,
    };
    let shown = text(&draw(&theme(), &at(), &moment, 40, 24));
    assert_eq!(
        shown[1],
        format!("  Install hedgehog{}3 of 13", " ".repeat(36 - 16 - 7))
    );
}

#[test]
fn every_step_done_stays_on_the_screen_the_latest_nearest() {
    let ask = Ask::Act {
        id: "plugged",
        what: "Plug in the Nano.".into(),
    };
    let moment = Moment::Asking {
        title: "Enrol a YubiKey",
        ask: &ask,
        typed: "",
        chosen: 0,
        refused: None,
        back: false,
    };
    let done = history();
    let at = Where {
        done: &done,
        ..at()
    };
    let shown = text(&draw(&theme(), &at, &moment, 60, 24));
    assert_eq!(shown[3], "  v The keyboard types Colemak", "{shown:#?}");
    assert_eq!(
        shown[4], "  v The vault is open, holding 12 files",
        "{shown:#?}"
    );
    assert_eq!(shown[5], "");
    assert_eq!(shown[6], "  Enrol a YubiKey");
    // A short screen gives up the earliest first.
    let short = text(&draw(&theme(), &at, &moment, 60, 11));
    assert!(!short.concat().contains("Colemak"), "{short:#?}");
    assert!(short.concat().contains("The vault is open"), "{short:#?}");
}

#[test]
fn escape_is_shown_only_where_it_goes_back() {
    let ask = Ask::Secret {
        id: "again",
        name: "the same password again".into(),
    };
    let moment = |back| Moment::Asking {
        title: "Choose the account's password",
        ask: &ask,
        typed: "",
        chosen: 0,
        refused: None,
        back,
    };
    let first = text(&draw(&theme(), &at(), &moment(false), 80, 24));
    assert_eq!(first.last().unwrap(), "  Return enter it    Ctrl+C leave");
    let later = text(&draw(&theme(), &at(), &moment(true), 80, 24));
    assert_eq!(
        later.last().unwrap(),
        "  Return enter it    Esc back    Ctrl+C leave"
    );
}

#[test]
fn a_secret_with_another_way_in_says_both_on_one_screen() {
    let ask = Ask::SecretOr {
        id: "passphrase",
        name: "the stick's passphrase, the seven words in 1Password on your phone".into(),
        or: "plug in the Nano or the 5Ci, type its PIN and press Return, then touch the key when it blinks".into(),
    };
    let moment = Moment::Asking {
        title: "Open the stick's vault",
        ask: &ask,
        typed: "ab",
        chosen: 0,
        refused: None,
        back: false,
    };
    let drawn = draw(&theme(), &at(), &moment, 120, 24);
    let shown = text(&drawn);
    assert!(shown.contains(&"  Type the stick's passphrase, the seven words in 1Password on your phone, then press Return.".to_string()), "{shown:#?}");
    assert!(shown.contains(&"  Or plug in the Nano or the 5Ci, type its PIN and press Return, then touch the key when it blinks.".to_string()), "{shown:#?}");
    let field = shown
        .iter()
        .position(|line| line == "  % **")
        .expect("one field");
    assert_eq!(drawn.caret, Some((u16::try_from(field).unwrap(), 6)));
}

#[test]
fn a_step_at_work_says_how_far_along_it_is() {
    let moment = Moment::Working {
        title: "Install the system",
        meanwhile: "",
        progress: Some("3.1 GB of 9.5 GB, 15.5 MB a second, about 7 minutes left"),
    };
    let shown = text(&draw(&theme(), &at(), &moment, 80, 24));
    assert_eq!(shown[3], "  Install the system");
    assert_eq!(
        shown[5],
        "  > 3.1 GB of 9.5 GB, 15.5 MB a second, about 7 minutes left"
    );
}

#[test]
fn keys_are_named_the_same_on_a_console_as_in_a_terminal() {
    let console = Theme::from("", "", true);
    let terminal = Theme::from("", "", false);
    for key in [Key::Return, Key::Tab, Key::Escape, Key::Arrows, Key::Leave] {
        assert_eq!(console.key(key), terminal.key(key));
    }
    assert_eq!(terminal.key(Key::Return), "Return");
}

#[test]
fn a_short_screen_gives_up_what_came_before_then_the_count() {
    let ask = Ask::Act {
        id: "plugged",
        what: "Plug in the Nano.".into(),
    };
    let moment = Moment::Asking {
        title: "Enrol a YubiKey",
        ask: &ask,
        typed: "",
        chosen: 0,
        refused: None,
        back: false,
    };
    let medium = text(&draw(&theme(), &at(), &moment, 80, 14));
    assert!(
        !medium.concat().contains("the vault is open"),
        "{medium:#?}"
    );
    assert!(
        medium[1].ends_with("3 of 13"),
        "the step count left its line: {medium:#?}"
    );
    let short = text(&draw(&theme(), &at(), &moment, 80, 10));
    assert_eq!(short[1], "  Install hedgehog");
}

#[test]
fn what_to_type_back_stands_alone_above_its_field() {
    let ask = Ask::TypeBack {
        id: "machine",
        expected: "hedgehog".into(),
        why: "installing wipes SK hynix HFM128GDGTNG-87A0A, 128 GB, and everything on it".into(),
    };
    let moment = Moment::Asking {
        title: "Confirm the disk to wipe",
        ask: &ask,
        typed: "pen",
        chosen: 0,
        refused: Some("\"hedgehoge\" isn't hedgehog, so nothing was wiped"),
        back: false,
    };
    let drawn = draw(&theme(), &at(), &moment, 60, 30);
    let shown = text(&drawn);
    let expected = shown
        .iter()
        .position(|line| line == "  hedgehog")
        .expect("hedgehog alone");
    assert_eq!(shown[expected + 2], "  > pen");
    assert!(
        shown.contains(&"  x \"hedgehoge\" isn't hedgehog, so nothing was wiped".to_string()),
        "{shown:#?}"
    );
    let strong = &drawn.lines[expected][1];
    assert_eq!(strong.role, Role::Strong);
}

#[test]
fn a_mark_leads_and_what_it_wraps_to_lines_up_under_its_ink() {
    let moment = Moment::Blocked {
        title: "Make the machine's Secure Boot keys",
        why: "the firmware's Secure Boot is in user mode, so it won't take new keys",
        remedy: "clear the firmware's Secure Boot keys in its setup, then restart from the stick",
    };
    let shown = text(&draw(&theme(), &at(), &moment, 50, 30));
    let first = shown
        .iter()
        .position(|line| line.starts_with("  ! "))
        .unwrap();
    // Under the mark's ink, not indented from it.
    for below in &shown[first + 1..first + 4] {
        assert!(
            below.starts_with("  ") && !below.starts_with("   "),
            "{shown:#?}"
        );
    }
    assert!(
        shown.iter().all(|line| line.chars().count() <= 48),
        "{shown:#?}"
    );
    assert_eq!(shown.last().unwrap(), "  Return look again    Ctrl+C leave");
}

#[test]
fn a_choice_leads_with_the_cursor_and_the_rest_line_up_under_its_words() {
    let ask = Ask::Choose {
        id: "repair",
        question: "What should be done?".into(),
        choices: vec!["Open a root shell inside it".into(), "Restart".into()],
    };
    let moment = Moment::Asking {
        title: "The machine's disk is open",
        ask: &ask,
        typed: "",
        chosen: 1,
        refused: None,
        back: false,
    };
    let drawn = draw(&theme(), &at(), &moment, 60, 24);
    let shown = text(&drawn);
    assert!(
        shown.contains(&"    Open a root shell inside it".to_string()),
        "{shown:#?}"
    );
    assert!(shown.contains(&"  > Restart".to_string()), "{shown:#?}");
    let row = shown.iter().position(|line| line == "  > Restart").unwrap();
    assert_eq!(drawn.lines[row][1].role, Role::Accent);
    assert_eq!(
        shown.last().unwrap(),
        "  ↑↓ move    Return choose    Ctrl+C leave"
    );
    assert_eq!(drawn.caret, None);
}

#[test]
fn what_a_phone_reads_is_alone_in_the_middle_of_the_screen() {
    let words = "abacus abdomen abdominal abide abiding ability ablaze";
    let drawn = draw(&theme(), &at(), &Moment::Showing { shown: words }, 80, 20);
    let shown = text(&drawn);
    let inked: Vec<&String> = shown.iter().filter(|line| !line.is_empty()).collect();
    assert_eq!(inked, [&format!("{}{words}", " ".repeat((80 - 53) / 2))]);
    assert_eq!(shown.len(), 10);
    assert_eq!(drawn.lines[9][1].role, Role::Strong);
    assert_eq!(drawn.caret, None);
}

#[test]
fn steps_for_the_phone_are_a_line_each_their_words_after_their_number() {
    let ask = Ask::Show {
        id: "shown",
        how: crate::words::keeping(
            "the recovery key",
            "Hedgehog Recovery Phrase",
            "It opens the disk",
        ),
        shown: "abacus abdomen abdominal abide abiding ability ablaze".into(),
    };
    let moment = Moment::Asking {
        title: "Keep the recovery key",
        ask: &ask,
        typed: "",
        chosen: 0,
        refused: None,
        back: false,
    };
    let shown = text(&draw(&theme(), &at(), &moment, 60, 40));
    let first = shown
        .iter()
        .position(|line| line.starts_with("  1. On your iPhone"))
        .unwrap_or_else(|| panic!("{shown:#?}"));
    assert_eq!(shown[first - 1], "", "{shown:#?}");
    assert_eq!(shown[first + 1], "     words.", "{shown:#?}");
    for number in 1..=5 {
        assert!(
            shown
                .iter()
                .any(|line| line.starts_with(&format!("  {number}. "))),
            "{shown:#?}"
        );
    }
    assert!(!shown.concat().contains("abacus"), "{shown:#?}");
    assert_eq!(shown.last().unwrap(), "  Return show it    Ctrl+C leave");
}

#[test]
fn words_wrap_between_themselves() {
    assert_eq!(wrap("one two three four", 9), ["one two", "three", "four"]);
    assert_eq!(wrap("abcdefghijk", 4), ["abcd", "efgh", "ijk"]);
    assert_eq!(
        wrap("aaaaaaaa-bbbbbbbb-cccccccc-dddddddd", 20),
        ["aaaaaaaa-bbbbbbbb-", "cccccccc-dddddddd"]
    );
}
