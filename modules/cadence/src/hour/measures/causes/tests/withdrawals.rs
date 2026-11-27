//! A sender taking back its cause line: what it withdraws, whatever its
//! stamp, and how long the withdrawal is kept between hours.

use super::*;

/// A line relayed here after its sender's withdrawal was read, and let go
/// by the mesh, still stays out.
#[test]
fn a_withdrawal_is_kept_for_a_line_written_here_after_it() {
    let home = tempfile::tempdir().expect("a temporary home");
    let folder = tempfile::tempdir().expect("a temporary folder");
    let messages = home
        .path()
        .join("pi/agentic-harness.pi/mesh-workflow/messages");
    fs::create_dir_all(&messages).expect("the messages folder");
    let first = format!("{}-SENDER-1-00000001", (NOW - 1200) * 1000);
    let mut withdraws = sent("no cause here after all", 600, "@1JQPPC");
    withdraws.supersedes = Some(first.clone());
    write(&messages, &withdraws, 600);
    let heard = read(home.path(), folder.path(), NOW, true);
    assert_eq!(
        heard.now, 0.0,
        "a withdrawal alone read as a cause: {:?}",
        heard.behind
    );
    fs::remove_dir_all(&messages).expect("the mesh lets the withdrawal go");
    fs::create_dir_all(&messages).expect("an empty messages folder");
    for n in 1..=3 {
        let line = format!("cause: flaky gate | change: core.nix#{n}");
        let mut m = sent(&line, 1200, "@1JQPPC");
        m.id = format!("{}-SENDER-1-0000000{n}", (NOW - 1200) * 1000);
        write(&messages, &m, -7200);
    }
    let later = read(home.path(), folder.path(), NOW + 7300, true);
    assert_eq!(
        later.now, 0.0,
        "a line written here after its withdrawal was let go counted: {:?}",
        later.behind
    );
}

/// A sender whose clock runs fast still withdraws its line. A withdrawal
/// stamped up to a day ahead is kept, so its line relayed here late
/// still stays out; one stamped further ahead isn't kept, though it
/// withdraws its line this hour all the same.
#[test]
fn a_withdrawal_stamped_ahead_still_withdraws_its_line() {
    let home = tempfile::tempdir().expect("a temporary home");
    let folder = tempfile::tempdir().expect("a temporary folder");
    let messages = home
        .path()
        .join("pi/agentic-harness.pi/mesh-workflow/messages");
    fs::create_dir_all(&messages).expect("the messages folder");
    let within = format!("{}-SENDER-1-00000001", (NOW + DAY - 60) * 1000);
    let past = format!("{}-SENDER-1-00000002", (NOW + DAY + 60) * 1000);
    // Each line is over the bar alone, so the class counts while either
    // stands.
    for (id, named) in [
        (&within, "core.nix#1, #2, #3"),
        (&past, "core.nix#4, #5, #6"),
    ] {
        let mut line = sent(
            &format!("cause: flaky gate | change: {named}"),
            0,
            "@1JQPPC",
        );
        line.id.clone_from(id);
        write(&messages, &line, 600);
    }
    let look = read(home.path(), folder.path(), NOW, false);
    assert_eq!(
        look.now, 1.0,
        "a line stamped ahead didn't count before its withdrawal: {:?}",
        look.behind
    );
    assert!(
        look.behind.len() == 1 && look.behind[0].contains("on 6 changes"),
        "the two lines stamped ahead weren't counted as one class on six changes: {:?}",
        look.behind
    );
    for (n, id) in [&within, &past].into_iter().enumerate() {
        let mut withdraws = sent("no cause here after all", 0, "@1JQPPC");
        withdraws.id = format!("{}-SENDER-1-0000001{n}", (NOW + DAY) * 1000);
        withdraws.supersedes = Some(id.clone());
        write(&messages, &withdraws, 300);
    }
    let hour = read(home.path(), folder.path(), NOW, true);
    assert_eq!(
        hour.now, 0.0,
        "a line stamped ahead still counted once its sender withdrew it: {:?}",
        hour.behind
    );
    let kept = fs::read_to_string(folder.path().join("causes.json")).expect("a kept reading");
    assert!(
        kept.contains(&within),
        "a withdrawal stamped within a day ahead wasn't kept: {kept}"
    );
    assert!(
        !kept.contains(&past),
        "a withdrawal stamped over a day ahead was kept: {kept}"
    );
}

/// A withdrawal is kept only while the line it withdraws could still
/// count, and one stamped over a day ahead or running long isn't kept at
/// all, so the kept file can't grow without end.
#[test]
fn a_kept_withdrawal_is_let_go_after_its_line_and_a_false_one_isnt_kept() {
    let home = tempfile::tempdir().expect("a temporary home");
    let folder = tempfile::tempdir().expect("a temporary folder");
    let messages = home
        .path()
        .join("pi/agentic-harness.pi/mesh-workflow/messages");
    fs::create_dir_all(&messages).expect("the messages folder");
    let real = format!("{}-SENDER-1-00000001", (NOW - 1200) * 1000);
    let ahead = format!("{}-SENDER-1-00000002", (NOW + 2 * DAY) * 1000);
    let long = format!("{}-SENDER-1-{}", (NOW - 1200) * 1000, "0".repeat(200));
    for (n, id) in [&real, &ahead, &long].into_iter().enumerate() {
        let mut m = sent("no cause here after all", 600, "@1JQPPC");
        m.id = format!("{}-SENDER-1-0000001{n}", (NOW - 600) * 1000);
        m.supersedes = Some(id.clone());
        write(&messages, &m, 600);
    }
    let first = read(home.path(), folder.path(), NOW, true);
    assert_eq!(
        first.now, 0.0,
        "withdrawals alone read as a cause: {:?}",
        first.behind
    );
    let kept = fs::read_to_string(folder.path().join("causes.json")).expect("a kept reading");
    assert!(
        kept.contains(&real) && !kept.contains(&ahead) && !kept.contains(&long),
        "a real withdrawal wasn't kept, or one stamped ahead or running long was: {kept}"
    );
    fs::remove_dir_all(&messages).expect("the mesh lets the withdrawals go");
    fs::create_dir_all(&messages).expect("an empty messages folder");
    let late = read(home.path(), folder.path(), NOW + KEPT_FOR, true);
    assert_eq!(
        late.now, 0.0,
        "an empty mesh read as a cause: {:?}",
        late.behind
    );
    let kept = fs::read_to_string(folder.path().join("causes.json")).expect("a kept reading");
    assert!(
        !kept.contains(&real),
        "a withdrawal was kept past the line it withdraws: {kept}"
    );
}
