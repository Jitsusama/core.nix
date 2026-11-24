//! What the net reads as a hold, case by case, from the holds sent on the
//! machine, and how a hold stands against its answers and the boot.

use super::*;
use serde_json::json;

fn hold(id: &str, from: &str, to: &str) -> Value {
    json!({ "id": id, "from": from, "to": [to], "intent": "hold", "text": id })
}

fn saying(text: &str) -> Value {
    json!({ "id": "h", "from": "A", "to": ["#machine"], "intent": "hold", "text": text })
}

// The four holds that stopped the net from 15:26 to past 18:35 on 19
// November, while 21 merges landed: none of them is about its work.
#[test]
fn a_hold_on_concluding_or_on_slack_leaves_the_net_alone() {
    for text in [
        "P9R1KI hold: conclude or retire no quest, on either machine, until I end this \
         hold. Concluded quests' folders are being deleted. Everything else carries on.",
        "P9R1KI, renewing the conclude hold: conclude or retire no quest, on either \
         machine, until I end this hold. Documents are fine. The quest-share hotfix \
         lands about 18:30.",
        "P9R1KI, Joel's rule from 17:29, on both work machines: read and send Slack only \
         through the Tool Gateway's slack tools (mcp__tool_gateway slack_*). Use no \
         `slack` tool from the harness, and no direct slack.com call, until I end this \
         hold.",
        // A hold that says what it leaves alone names the net's work
        // only to free it.
        "P9R1KI, renewing the Slack hold: read and send Slack only through the Tool \
         Gateway's slack tools. This covers Slack only. It doesn't hold builds, merges, \
         switches or anything else, so merge-net and the trains carry on.",
        // A sentence saying when the hold ends is its term, not what it
        // holds, so the Mac's switch named there holds nothing.
        "WCJQSG (renewing 1e21f0d9): read and send Slack only through the Tool Gateway's \
         slack tools (mcp__tool_gateway slack_*). Use no harness `slack` tool, no other \
         Slack MCP and no direct slack.com call, until I end this hold. The drafts-only \
         rule is unchanged: send nothing. This covers Slack only. It doesn't hold builds, \
         merges, switches, reboots, uploads, pi host cuts or anything else. It stays \
         until ISXEG8's Slack change is live on both machines: the laptop has the \
         config now, the Mac switches about 21:45 to 22:15, and the harness half (#1871) comes \
         at each machine's next pi update after about 21:30.",
        "WCJQSG (renewing 1e21f0d9, rewording a0c38bfb): read and send Slack only through \
         the Tool Gateway's slack tools (mcp__tool_gateway slack_*). This covers Slack \
         only, and nothing else of anyone's work. It stays until ISXEG8's Slack change is \
         live on both machines: the laptop has the config now, the Mac's update lands \
         about 21:45 to 22:15.",
        "WCJQSG (renewing 45a21014): conclude or retire no quest, on either machine, \
         until I end this hold. Concluding or retiring a document is fine. This covers \
         concluding and retiring quests only. It doesn't hold builds, merges, switches, \
         reboots, uploads, pi host cuts or anything else. It stays until the quest-share \
         hotfix is live on both machines (about 20:45) and the restore of the 16 \
         set-aside quests is checked, on the laptop by about 21:30.",
        "WCJQSG (renewing 8950e683): conclude or retire no quest, on either machine, \
         until I end this hold. Concluding or retiring a document is fine. It stays until \
         KP0L6A's restore of the set-aside quests is checked on both machines. The \
         laptop's was checked at 22:51; the Mac's comes about 23:30, after the Mac's \
         update lands. After that it narrows, not ends: no parent quest concluded or \
         retired, and no concluded quest reopened, until Fri 20 18:00.",
        "Conclude no quest. The hold stays until the Mac switches at 22:00.",
    ] {
        assert_eq!(still_held(&[saying(text)]), None, "held by: {text}");
        // A renewal passes on what it restates, even once the hold it
        // renews is gone, as it is after a reboot.
        let mut renewal = saying(text);
        renewal["supersedes"] = json!("gone");
        assert_eq!(still_held(&[renewal]), None, "held as a renewal: {text}");
    }
}

// The 20:07 renewal as sent, with a typographic apostrophe, passes.
#[test]
fn a_typographic_apostrophe_frees_as_a_plain_one_does() {
    let text = "WCJQSG: conclude or retire no quest. It doesn\u{2019}t hold builds, merges, \
                switches or reboots.";
    assert_eq!(still_held(&[saying(text)]), None);
}

// Each case is read with a subject the net never touches beside it too,
// so what holds it is the net's work it names, not a hold naming none.
#[test]
fn a_hold_on_the_nets_work_still_stops_it() {
    let subjected = |text: &str| format!("Conclude no quest. {text}");
    for text in [
        // A freeing phrase frees only its own clause, and only a list
        // after "doesn't hold" frees a reboot.
        "It doesn't hold Slack, and the laptop reboots at 21:00.",
        "It doesn't hold Slack and the laptop reboots at 21:00.",
        "It doesn't hold Slack, the laptop reboots at 21:00.",
        "It doesn't hold Slack - the laptop reboots at 21:00.",
        "It doesn't hold Slack, the laptop's builds go elsewhere tonight.",
        "It doesn't hold Slack, the laptop reboots.",
        "It doesn't hold merges, the Mac restarts.",
        "It doesn't hold Slack, laptop reboots tonight.",
        "It doesn't hold Slack, Mac restarts.",
        // A sentence saying when the hold ends still holds what it
        // says to stop.
        "It stays until 22:00: start no builds.",
        "It stays until the Mac is back, and don't merge till then.",
        // A sentence freeing something that holds something as well
        // holds the net by name.
        "Slack is fine, but hold merge-net until 22:00.",
        "Slack is fine, but leave merge-net alone until 22:00.",
        // A hold naming the net, in a sentence freeing nothing, holds it
        // whatever its verb.
        "Hold merge-net till 22:00.",
        "Leave the merge net alone until Slack is back.",
        "Merge-net stays paused till 21:00.",
        "The laptop reboots tonight and Slack is fine.",
        "The laptop switches tonight and Slack is fine.",
        "A rebuild runs, and Slack is fine.",
        "Builds are fine from 21:00.",
        "Rebooting the laptop is fine.",
        // A freeing sentence saying when holds something after all.
        "Switches are fine once the hotfix is live.",
        "Merges carry on after 21:00.",
        "Builds are fine before 21:00, not after.",
        "Builds are fine till 21:00.",
        "Merges carry on unless Joel says so.",
        "Switches are fine while Joel is away.",
    ] {
        let text = subjected(text);
        assert_eq!(
            still_held(&[saying(&text)]),
            Some(text.clone()),
            "not held by: {text}"
        );
    }
    for text in [
        "16V6F4, for every quest: the laptop rebooted; start no rebuild until I end this.",
        "P9R1KI, for every quest: Joel is shutting the laptop down. Start nothing.",
        "P9R1KI, for every quest: we're deploying the core lock, so hold off.",
        // A sentence that frees one thing and holds another holds.
        "P9R1KI, for every quest: documents are fine but start no builds.",
        "P9R1KI, for every quest: don't hold back, stop every build now.",
        "P9R1KI, for every quest: Slack is fine, except that nothing merges until 21:00.",
        "16V6F4: the input update's switch restarts nix-daemon on the laptop. Start no new \
         nix build, eval or nix-offload there run until I end this hold.",
        "Integration (16V6F4): the laptop switches now, and it restarts nix-daemon.",
        "P9R1KI: Joel is rebooting the laptop in about 15 minutes. Park anything that would \
         die: local VM tests, gates.",
        "P9R1KI, for every quest: merge nothing into any repository, on either machine, \
         until I end this hold at 17:00.",
        "P9R1KI, for every quest: Joel is on a call. Start nothing heavy on the laptop.",
    ] {
        for text in [text.to_owned(), subjected(text)] {
            assert_eq!(
                still_held(&[saying(&text)]),
                Some(text.clone()),
                "not held by: {text}"
            );
        }
    }
    for text in [
        // A hold that names neither its subject nor the net's work is
        // read as being about everything, so the net fails closed.
        "P9R1KI: stop now, until I end this hold.",
        // One that names a conclude and the net's work is about both.
        "P9R1KI: conclude no quest, and start no build, until I end this hold.",
        // A sentence freeing one thing frees nothing in the others.
        "P9R1KI: conclude no quest and start no VM tests; documents are fine. \
         Everything else carries on.",
        // "For every quest" names no subject, so a hold worded in no
        // word the net knows still stops it.
        "P9R1KI, for every quest: Joel is flashing new firmware. Hold off for now.",
        "P9R1KI, for every quest: hold everything until 21:00.",
        // What a sentence frees is what its phrase is said of, so a
        // reboot beside it still holds.
        "P9R1KI: retire no quest. Slack is fine, and the laptop reboots at 21:00.",
        "P9R1KI: retire no quest. It doesn't hold Slack; the laptop goes down at 21:00.",
        "P9R1KI: conclude nothing. Documents are fine till the kernel upgrade.",
        "P9R1KI: conclude nothing while the core lock lands on main.",
        "P9R1KI: retire no quest. A rollback of the laptop is coming; boot nothing new.",
    ] {
        assert_eq!(
            still_held(&[saying(text)]),
            Some(text.to_owned()),
            "not held by: {text}"
        );
    }
}

// A short renewal keeps the subject of the hold it replaces, and a hold
// the net passes by doesn't hide one standing after it.
#[test]
fn a_renewal_keeps_what_it_renews_and_a_later_hold_still_counts() {
    let first = json!({ "id": "h1", "from": "A", "to": ["#machine"], "intent": "hold",
        "text": "A: start no VM tests until I end this." });
    let renewed = json!({ "id": "h2", "from": "A", "to": ["#machine"], "intent": "hold",
        "supersedes": "h1", "text": "A, renewing my conclude hold until 21:00." });
    assert_eq!(
        still_held(&[first, renewed.clone()]),
        Some("A, renewing my conclude hold until 21:00.".to_owned()),
        "a renewal naming its subject but not restating it dropped the builds"
    );
    // "No change" beside the hold, not its subject, restates nothing.
    for text in [
        "A, renewing my conclude hold, no change, until 21:00.",
        "A, renewing my conclude hold until 21:00; nothing else changed.",
        "A, renewing my hold on Slack, no change.",
        "A, renewing concluding, nothing else changed.",
    ] {
        let renewed = json!({ "id": "h2", "from": "A", "to": ["#machine"],
            "intent": "hold", "supersedes": "h1", "text": text });
        let first = json!({ "id": "h1", "from": "A", "to": ["#machine"],
            "intent": "hold", "text": "A: start no VM tests until I end this." });
        assert_eq!(still_held(&[first, renewed]), Some(text.to_owned()));
    }
    // One whose hold isn't to be found, as one sent before a reboot,
    // can't say what it renews, so it stops the net.
    assert_eq!(
        still_held(std::slice::from_ref(&renewed)),
        Some("A, renewing my conclude hold until 21:00.".to_owned())
    );
    let fresh = json!({ "id": "s", "from": "A", "to": ["#machine"], "intent": "hold",
        "text": "A: the Slack token rotates until 21:00." });
    assert_eq!(
        still_held(&[fresh]),
        None,
        "a hold renewing none was read as one"
    );
    // One restating its subject outright drops what the first mentioned.
    let conclude = json!({ "id": "c1", "from": "A", "to": ["#machine"], "intent": "hold",
        "text": "A: conclude no quest; at 18:00 both machines switch." });
    let restated = json!({ "id": "c2", "from": "A", "to": ["#machine"], "intent": "hold",
        "supersedes": "c1", "text": "A, renewing: conclude or retire no quest." });
    assert_eq!(still_held(&[conclude, restated]), None);
    let slack = saying("A: send Slack only through the gateway's tools.");
    let builds = json!({ "id": "b", "from": "B", "to": ["#machine"], "intent": "hold",
        "text": "B, for every quest: start no builds." });
    let both = [slack, builds];
    assert_eq!(
        still_held(&both),
        Some("B, for every quest: start no builds.".to_owned())
    );
    assert_eq!(
        passed_by(&both, 0),
        [&both[0]],
        "the Slack hold wasn't the one passed by"
    );
}

#[test]
fn a_hold_on_everyone_stands_until_its_sender_answers_it() {
    let held = hold("h1", "A", "#machine");
    assert_eq!(
        still_held(std::slice::from_ref(&held)),
        Some("h1".to_owned())
    );
    let other = json!({ "id": "x", "from": "B", "re": "h1", "intent": "fyi" });
    assert_eq!(still_held(&[held.clone(), other]), Some("h1".to_owned()));
    let ended = json!({ "id": "e", "from": "A", "re": "h1", "intent": "fyi" });
    assert_eq!(still_held(&[held.clone(), ended]), None);
    let replaced = json!({ "id": "r", "from": "A", "supersedes": "h1", "intent": "fyi" });
    assert_eq!(still_held(&[held, replaced]), None);
}

#[test]
fn a_hold_on_one_session_leaves_the_net_alone() {
    assert_eq!(still_held(&[hold("h", "A", "@B")]), None);
    assert_eq!(still_held(&[hold("h", "A", "#all")]), Some("h".to_owned()));
}

#[test]
fn a_hold_with_no_sender_names_no_one() {
    assert_eq!(
        held_by(&[hold("h", "", "#machine")], 0).map(|held| held.from),
        Some(None),
        "a hold from an empty sender was named as from someone"
    );
}

#[test]
fn only_holds_sent_since_the_boot_are_read() {
    let folder = std::env::temp_dir().join(format!("merge-net-holds-{}", std::process::id()));
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).unwrap();
    let write = |name: &str, message: &Value| {
        fs::write(folder.join(name), message.to_string()).unwrap();
    };
    write("1000-A-1-aa.json", &hold("old", "A", "#machine"));
    assert_eq!(
        standing(&folder, 500, 5000),
        Some(Held {
            why: "old".to_owned(),
            from: Some("A".to_owned())
        }),
        "a hold standing says who sent it"
    );
    // A hold the net doesn't read sent first names no one the net waits on.
    let mut conclude = hold("conclude", "B", "#machine");
    conclude["text"] = json!(
        "P9R1KI hold: conclude or retire no quest, on either machine, until I end this \
         hold. Concluded quests' folders are being deleted. Everything else carries on."
    );
    write("900-B-1-ab.json", &conclude);
    assert_eq!(
        standing(&folder, 500, 5000).and_then(|held| held.from),
        Some("A".to_owned()),
        "a hold the net leaves alone was named as who holds it"
    );
    assert_eq!(standing(&folder, 2000, 5000), None);
    write("3000-A-1-bb.json", &hold("new", "A", "#machine"));
    assert_eq!(
        standing(&folder, 2000, 5000).map(|held| held.why),
        Some("new".to_owned())
    );
    fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn an_answer_filed_in_a_day_folder_still_ends_its_hold() {
    let folder = std::env::temp_dir().join(format!("merge-net-days-{}", std::process::id()));
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(folder.join("2026-11-09")).unwrap();
    let held = hold("1000-A-1-aa", "A", "#all");
    fs::write(folder.join("1000-A-1-aa.json"), held.to_string()).unwrap();
    assert_eq!(
        standing(&folder, 500, 5000).map(|held| held.why),
        Some("1000-A-1-aa".to_owned()),
        "a hold with no answer stands"
    );
    let ended = json!({ "id": "2000-A-1-bb", "from": "A", "re": "1000-A-1-aa", "intent": "fyi" });
    fs::write(
        folder.join("2026-11-09/2000-A-1-bb.json"),
        ended.to_string(),
    )
    .unwrap();
    assert_eq!(
        standing(&folder, 500, 5000),
        None,
        "the answer in the day's folder ends the hold above it"
    );
    fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn an_iso_time_reads_as_the_mesh_writes_it() {
    let cases = [
        ("1970-01-01T00:00:00Z", Some(0)),
        // The hold of 24 November at 12:14, whose file's name begins with it.
        ("2026-11-24T17:14:15.133Z", Some(1_795_540_455_133)),
        ("2026-11-24T12:14:15.133-05:00", Some(1_795_540_455_133)),
        ("2026-11-24T22:44:15.1+05:30", Some(1_795_540_455_100)),
        ("2000-02-29T00:00:00Z", Some(951_782_400_000)),
        ("2000-03-01T00:00:00Z", Some(951_868_800_000)),
        ("2026-11-24T17:14:15", None),
        ("2026-13-01T00:00:00Z", None),
        ("2026-11-24T25:00:00Z", None),
        ("2026-11-24T17:14:15.Z", None),
        ("12:15", None),
        ("", None),
    ];
    for (text, ms) in cases {
        assert_eq!(iso_ms(text), ms, "{text:?} read as another time");
    }
}

/// A hold to everyone, `id`, with the times `fields` give it.
fn timed(id: &str, fields: &[(&str, &str)]) -> Value {
    let mut held = hold(id, "A", "#machine");
    for (field, at) in fields {
        held[*field] = json!(at);
    }
    held
}

#[test]
fn a_hold_sent_ahead_holds_only_in_its_window() {
    // 1OGSE7's 280fdfcb, sent at 10:58 for Joel's call, said in words that
    // it held from 12:15 to 13:10; told in fields, the net runs at 11:35.
    let call = timed(
        "call",
        &[
            ("at", "2026-11-24T15:58:06.559Z"),
            ("starts", "2026-11-24T17:15:00.000Z"),
            ("ends", "2026-11-24T18:10:00.000Z"),
        ],
    );
    let at = |text: &str| iso_ms(text).unwrap();
    let messages = std::slice::from_ref(&call);
    assert_eq!(
        held_by(messages, at("2026-11-24T16:35:00Z")),
        None,
        "held before it starts"
    );
    assert_eq!(
        ahead(messages, at("2026-11-24T16:35:00Z")).count(),
        1,
        "a hold sent ahead wasn't found ahead"
    );
    assert!(
        held_by(messages, at("2026-11-24T17:15:00Z")).is_some(),
        "not held once it starts"
    );
    assert!(
        held_by(messages, at("2026-11-24T18:09:59Z")).is_some(),
        "not held a second before it ends"
    );
    assert_eq!(
        held_by(messages, at("2026-11-24T18:10:00Z")),
        None,
        "held once it ends"
    );
    assert_eq!(
        ahead(messages, at("2026-11-24T17:30:00Z")).count(),
        0,
        "ahead once started"
    );
}

#[test]
fn a_hold_that_says_no_end_stands_two_hours() {
    let open = timed("open", &[("at", "2026-11-24T15:00:00Z")]);
    let messages = std::slice::from_ref(&open);
    let at = |text: &str| iso_ms(text).unwrap();
    assert!(
        held_by(messages, at("2026-11-24T16:59:59Z")).is_some(),
        "an open hold ended before two hours"
    );
    assert_eq!(
        held_by(messages, at("2026-11-24T17:00:00Z")),
        None,
        "stood past two hours"
    );
    // An end it says rules over the two hours, either way.
    let long = timed(
        "long",
        &[
            ("at", "2026-11-24T15:00:00Z"),
            ("ends", "2026-11-24T19:00:00Z"),
        ],
    );
    assert!(
        held_by(std::slice::from_ref(&long), at("2026-11-24T18:00:00Z")).is_some(),
        "a hold's own end past two hours was cut short"
    );
}

#[test]
fn a_window_that_cant_be_read_holds_as_one_with_none() {
    let unread = timed(
        "unread",
        &[("at", "soon"), ("starts", "12:15"), ("ends", "later")],
    );
    assert_eq!(
        still_held(std::slice::from_ref(&unread)),
        Some("unread".to_owned()),
        "a hold whose times can't be read held nothing"
    );
    assert_eq!(
        ahead(std::slice::from_ref(&unread), 0).count(),
        0,
        "unread start read as ahead"
    );
}
