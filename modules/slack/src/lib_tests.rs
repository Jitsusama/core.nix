//! The look-up's tests, beside it: who Slack knows by an email address.

use super::*;
use serde_json::json;

#[test]
fn slack_saying_how_long_is_waited_that_long() {
    assert_eq!(retry_after(Some("12")), Duration::from_secs(12));
    assert_eq!(retry_after(Some(" 3 ")), Duration::from_secs(3));
}

#[test]
fn slack_saying_nothing_readable_is_waited_half_a_minute() {
    assert_eq!(retry_after(None), WAIT_UNSAID);
    assert_eq!(retry_after(Some("soon")), WAIT_UNSAID);
    assert_eq!(
        retry_after(Some("Wed, 21 Oct 2026 07:28:00 GMT")),
        WAIT_UNSAID
    );
}

#[test]
fn slack_asking_for_hours_is_waited_ten_minutes_at_most() {
    assert_eq!(retry_after(Some("86400")), WAIT_MOST);
}

#[test]
fn someone_found_is_said_by_their_id_and_real_name() {
    let said = json!({ "ok": true, "user": { "id": "U1", "team_id": "T9",
        "real_name": "Ada L", "tz": "Europe/London",
        "profile": { "real_name": "Ada Lovelace", "title": "Analyst" } } });
    assert_eq!(
        found(&said, "ada@example.com", "T1"),
        Ok(Found {
            email: "ada@example.com".into(),
            id: Some("U1".into()),
            team: "T9".into(),
            name: "Ada Lovelace".into(),
            zone: "Europe/London".into(),
            presence: String::new(),
            status: String::new(),
            emoji: String::new(),
            handle: String::new(),
            title: "Analyst".into(),
            conversation: None,
        })
    );
    let handled = json!({ "ok": true, "user": { "id": "U1", "name": "ada" } });
    assert_eq!(
        found(&handled, "ada@example.com", "T1").unwrap().handle,
        "ada"
    );
}

#[test]
fn a_conversation_is_their_direct_messages_with_joel_newest_last() {
    let im = json!({ "id": "D1", "is_im": true, "name": "U1" });
    let said = json!({ "ok": true, "messages": { "matches": [
        { "channel": im, "user": "U1", "text": "See you at <!here|here> lunch",
          "ts": "1730312345.123456", "permalink": "https://slack.example/p3" },
        { "channel": { "id": "C1", "is_im": false, "name": "general" },
          "user": "U1", "text": "In a channel", "ts": "1730312300.000000" },
        { "channel": { "id": "D7", "is_im": true, "name": "U7" },
          "user": "U7", "text": "Someone else's", "ts": "1730312290.000000" },
        { "channel": im, "user": "U0", "text": "", "ts": "1730312280.000000" },
        { "channel": im, "user": "U0", "text": "Lunch, <@U1>?",
          "ts": "1730312200.5", "permalink": "https://slack.example/p1" }] } });
    assert_eq!(
        conversation(&said, "U1", "Ada Lovelace", "U0"),
        Ok(Some(Conversation {
            channel: "D1".into(),
            link: "https://slack.example/p3".into(),
            last: "See you at @here lunch".into(),
            at: 1_730_312_345_123,
            messages: vec![
                Message {
                    mine: true,
                    text: "Lunch, @Ada Lovelace?".into(),
                    at: 1_730_312_200_500,
                },
                Message {
                    mine: false,
                    text: "See you at @here lunch".into(),
                    at: 1_730_312_345_123,
                },
            ],
        }))
    );
    let none = json!({ "ok": true, "messages": { "matches": [] } });
    assert_eq!(conversation(&none, "U1", "Ada", "U0"), Ok(None));
    let refused = json!({ "ok": false, "error": "invalid_auth" });
    assert_eq!(
        conversation(&refused, "U1", "Ada", "U0"),
        Err(Refused::SignedOut)
    );
}

#[test]
fn a_message_is_said_in_plain_words() {
    let names = [("U1", "Ada Lovelace")];
    let said = |text| plain(text, &names);
    assert_eq!(
        said("<https://a.example|the plan> &amp; <https://b.example>"),
        "the plan & https://b.example"
    );
    assert_eq!(
        said("write <mailto:ada@example.com|me> or <mailto:g@example.com>"),
        "write me or g@example.com"
    );
    assert_eq!(
        said("<@U1> and <@U2> and <@U3|grace>"),
        "@Ada Lovelace and @someone and @grace"
    );
    assert_eq!(
        said("in <#C1|general> <!channel> <!subteam^S1|@shell>"),
        "in #general @channel @shell"
    );
    assert_eq!(said("1 &lt; 2 &gt; 0"), "1 < 2 > 0");
    assert_eq!(
        said("an open < without its end"),
        "an open < without its end"
    );
    let long = "word ".repeat(100);
    let cut = said(&long);
    assert!(cut.len() <= LONGEST && cut.ends_with("word"), "{cut}");
}

#[test]
fn slacks_times_are_milliseconds() {
    assert_eq!(millis("1730312345.123456"), Some(1_730_312_345_123));
    assert_eq!(millis("1730312345.5"), Some(1_730_312_345_500));
    assert_eq!(millis("1730312345"), Some(1_730_312_345_000));
    assert_eq!(millis("soon"), None);
}

#[test]
fn a_status_is_its_words_and_its_emoji_while_it_lasts() {
    let palm = json!({ "status_text": "Away on the coast ", "status_expiration": 2000,
        "status_emoji_display_info": [{ "emoji_name": "palm_tree", "unicode": "1f334" }] });
    assert_eq!(
        status(&palm, 1000),
        ("Away on the coast".into(), "🌴".into())
    );
    assert_eq!(status(&palm, 2000), (String::new(), String::new()));
    let joined = json!({ "status_text": "Cooking", "status_expiration": 0,
        "status_emoji_display_info": [{ "unicode": "1f469-200d-1f373" }] });
    assert_eq!(
        status(&joined, u64::MAX),
        ("Cooking".into(), "👩\u{200d}🍳".into())
    );
    // A workspace's own picture has no Unicode form, so only the words.
    let own = json!({ "status_text": "Shipping", "status_emoji": ":ship-it:",
        "status_emoji_display_info": [{ "emoji_name": "ship-it",
            "display_url": "https://emoji.example/ship-it.png" }] });
    assert_eq!(status(&own, 0), ("Shipping".into(), String::new()));
    assert_eq!(status(&json!({}), 0), (String::new(), String::new()));
}

#[test]
fn someone_in_a_huddle_is_said_so_in_place_of_active() {
    let said = json!({ "ok": true, "user": { "id": "U1",
        "profile": { "huddle_state": "in_a_huddle" } } });
    assert_eq!(
        found(&said, "a@example.com", "T1").unwrap().presence,
        "in a huddle"
    );
    let not = json!({ "ok": true, "user": { "id": "U1",
        "profile": { "huddle_state": "default_unset" } } });
    assert_eq!(found(&not, "a@example.com", "T1").unwrap().presence, "");
}

#[test]
fn an_address_slack_knows_nobody_by_is_said_so() {
    let said = json!({ "ok": false, "error": "users_not_found" });
    let nobody = found(&said, "x@example.com", "T1").unwrap();
    assert_eq!(nobody.id, None);
    assert_eq!(nobody.team, "T1");
    assert_eq!(nobody.zone, "");
}

#[test]
fn a_sign_in_slack_refuses_is_signed_out() {
    for error in ["invalid_auth", "token_revoked"] {
        let said = json!({ "ok": false, "error": error });
        assert_eq!(found(&said, "x@example.com", "T1"), Err(Refused::SignedOut));
    }
}

#[test]
fn a_search_names_whoever_said_it_only_by_their_own_handle() {
    let said = json!({ "ok": true, "messages": { "matches": [
        { "user": "U7", "username": "someone.else" },
        { "user": "U2", "username": "Grace.Hopper" }] } });
    assert_eq!(speaker(&said, "grace.hopper"), Ok(Some("U2".into())));
    let none = json!({ "ok": true, "messages": { "matches": [] } });
    assert_eq!(speaker(&none, "grace.hopper"), Ok(None));
    let refused = json!({ "ok": false, "error": "invalid_auth" });
    assert_eq!(speaker(&refused, "grace.hopper"), Err(Refused::SignedOut));
}

#[test]
fn only_a_grids_refusal_of_the_sign_in_turns_to_searching() {
    for error in ["not_allowed_token_type", "enterprise_is_restricted"] {
        assert!(
            refused_to_the_sign_in(&json!({ "ok": false, "error": error })),
            "a grid's {error} wasn't taken as refusing the look-up to the sign-in"
        );
    }
    for error in ["users_not_found", "invalid_auth", "ratelimited"] {
        assert!(
            !refused_to_the_sign_in(&json!({ "ok": false, "error": error })),
            "{error} was taken as a grid refusing the look-up, so it would search"
        );
    }
    assert_eq!(
        addressed(&json!({ "ok": true, "user": { "profile": { "email": "Grace@Example.com" } } })),
        Some("grace@example.com".into())
    );
}

#[test]
fn who_was_found_is_kept_without_how_they_were_and_nobody_only_a_day() {
    let path = std::env::temp_dir()
        .join(format!("slack-people-kept-{}", std::process::id()))
        .join("known.json");
    let ada = Found {
        email: "ada@example.com".into(),
        id: Some("U1".into()),
        team: "T1".into(),
        name: "Ada Lovelace".into(),
        zone: "Europe/London".into(),
        presence: "active".into(),
        status: "Writing".into(),
        emoji: "📝".into(),
        handle: "ada".into(),
        title: "Analyst".into(),
        conversation: Some(Conversation {
            channel: "D1".into(),
            link: String::new(),
            last: "Hello".into(),
            at: 1,
            messages: Vec::new(),
        }),
    };
    let nobody = found(&nobody(), "x@example.com", "T1").unwrap();
    let mut kept = Kept::at(Some(path.clone()));
    kept.keep(&ada, 1000);
    kept.keep(&nobody, 1000);
    let again = Kept::at(Some(path.clone()));
    assert_eq!(again.get("ada@example.com", u64::MAX), Some(&ada.who()));
    assert_eq!(
        again.get("x@example.com", 1000 + NOBODY_HOLDS - 1),
        Some(&nobody)
    );
    assert_eq!(again.get("x@example.com", 1000 + NOBODY_HOLDS), None);
    assert_eq!(again.get("grace@example.com", 1000), None);
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
    // With nowhere to keep it, it's only held while the program runs.
    let mut held = Kept::at(None);
    held.keep(&ada, 1000);
    assert_eq!(held.get("ada@example.com", 1000), Some(&ada.who()));
}

#[test]
fn nobody_known_while_running_lapses_after_a_day_as_kept() {
    let nobody = found(&nobody(), "x@example.com", "T1").unwrap();
    let ada = Found {
        email: "ada@example.com".into(),
        id: Some("U1".into()),
        team: "T1".into(),
        name: "Ada Lovelace".into(),
        ..nobody.clone()
    };
    let mut kept = Kept::at(None);
    kept.keep(&nobody, 1000);
    kept.keep(&ada, 1000);
    let day = 24 * 60 * 60;
    assert_eq!(
        kept.over(Some(&nobody), "x@example.com", 1000 + day - 1),
        Some(nobody.clone()),
        "nobody, within the day"
    );
    assert_eq!(
        kept.over(Some(&nobody), "x@example.com", 1000 + day),
        None,
        "asked again a day on, though it's held while running"
    );
    assert_eq!(
        kept.over(Some(&ada), "ada@example.com", u64::MAX),
        Some(ada.clone()),
        "someone found holds"
    );
    // Found to be nobody again, it holds a day from then.
    kept.keep(&nobody, 1000 + day);
    assert_eq!(
        kept.over(Some(&nobody), "x@example.com", 1000 + 2 * day - 1),
        Some(nobody),
        "nobody again, held from when it was found"
    );
}

#[test]
fn a_rate_limit_is_unreachable_for_now() {
    let said = json!({ "ok": false, "error": "ratelimited" });
    let answer = found(&said, "x@example.com", "T1");
    assert!(
        matches!(answer, Err(Refused::Unreachable(_))),
        "a rate limit is unreachable for now, not {answer:?}"
    );
}

#[test]
fn only_a_plain_address_is_asked_about() {
    assert!(
        asked("ada@example.com"),
        "a plain address wasn't asked about"
    );
    for bad in [
        "",
        "ada",
        "@example.com",
        "ada@host",
        "a@b@c.com",
        "a b@c.com",
        "a@b.com,c@d.com",
    ] {
        assert!(!asked(bad), "{bad}");
    }
}
