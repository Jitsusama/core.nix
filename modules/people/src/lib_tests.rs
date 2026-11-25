//! The daemon's logic, tested apart from its sockets: what a client's ask
//! and a source's line read as, and what the book makes of them.

use super::*;

#[test]
fn one_person_is_one_key_whatever_case_their_address_comes_in() {
    assert_eq!(key(" Ada@Example.com"), "person:ada@example.com");
    assert_eq!(address("person:ada@example.com"), Some("ada@example.com"));
    assert_eq!(address("file:/x"), None);
    assert_eq!(
        ask(r#"{"id": 2, "ask": "get", "keys": ["person:Ada@Example.com", "file:x"]}"#),
        Ask::Get {
            id: json!(2),
            keys: vec!["person:ada@example.com".to_owned()],
            patience: 0
        }
    );
}

#[test]
fn each_ask_is_read_with_its_id_and_a_kind_not_kept_refused() {
    assert_eq!(
        ask(r#"{"id": 1, "ask": "hello", "version": 1}"#),
        Ask::Hello { id: json!(1) }
    );
    assert_eq!(
        ask(r#"{"id": 3, "ask": "list", "kind": "person", "limit": 200}"#),
        Ask::List {
            id: json!(3),
            limit: Some(200),
            listed: false
        }
    );
    assert_eq!(
        ask(r#"{"id": 4, "ask": "watch", "kind": "person"}"#),
        Ask::Watch {
            id: json!(4),
            keys: None,
            listed: false
        }
    );
    assert_eq!(
        ask(r#"{"id": 7, "ask": "watch", "listed": true}"#),
        Ask::Watch {
            id: json!(7),
            keys: None,
            listed: true
        }
    );
    assert_eq!(
        ask(r#"{"id": 8, "ask": "search", "words": "ada b"}"#),
        Ask::Search {
            id: json!(8),
            words: "ada b".to_owned(),
            limit: SEARCHED
        }
    );
    assert_eq!(
        ask(r#"{"id": 5, "ask": "list", "kind": "file"}"#),
        Ask::UnknownKind {
            id: json!(5),
            kind: "file".to_owned()
        }
    );
    assert_eq!(
        ask(r#"{"id": 6, "ask": "shout"}"#),
        Ask::Unknown {
            id: json!(6),
            ask: "shout".to_owned()
        }
    );
    assert_eq!(
        ask("not json"),
        Ask::Unknown {
            id: Value::Null,
            ask: String::new()
        }
    );
}

#[test]
fn a_sources_lines_are_someone_a_later_or_its_sign_in() {
    assert_eq!(
        heard(r#"{"email": "Ada@example.com", "id": "U1", "name": "Ada"}"#),
        Heard::Person {
            address: "ada@example.com".to_owned(),
            entry: json!({"id": "U1", "name": "Ada"})
        }
    );
    assert_eq!(
        heard(r#"{"email": "ada@example.com", "later": true}"#),
        Heard::Later {
            address: "ada@example.com".to_owned()
        }
    );
    assert_eq!(
        heard(r#"{"email": "", "id": 7, "gone": true}"#),
        Heard::Gone {
            address: String::new(),
            id: Some("7".to_owned())
        }
    );
    assert_eq!(heard(r#"{"email": "", "name": "Nobody"}"#), Heard::Other);
    assert_eq!(heard(r#"{"roster": true}"#), Heard::Roster);
    assert_eq!(heard(r#"{"signedOut": true}"#), Heard::SignedOut);
    assert_eq!(heard(r#"{"signedIn": false}"#), Heard::SignedOut);
    assert_eq!(heard("hello"), Heard::Other);
}

#[test]
fn what_a_source_says_is_held_whole_under_its_name_changing_only_when_it_does() {
    let mut book = Book::of(&["slack".to_owned(), "vault".to_owned()]);
    let ada = |name: &str| Heard::Person {
        address: "ada@example.com".to_owned(),
        entry: json!({"name": name}),
    };
    assert_eq!(
        book.take("slack", ada("Ada")),
        Some("person:ada@example.com".to_owned())
    );
    assert_eq!(book.take("slack", ada("Ada")), None);
    assert_eq!(
        book.waiting("person:ada@example.com").collect::<Vec<_>>(),
        ["vault"]
    );
    assert_eq!(
        book.take("vault", ada("Ada L")),
        Some("person:ada@example.com".to_owned())
    );
    assert!(
        book.ready("person:ada@example.com"),
        "Ada waited on {:?} once Slack and Vault had both answered",
        book.waiting("person:ada@example.com").collect::<Vec<_>>()
    );
    let record = book.record("person:ada@example.com").unwrap();
    let served = record.json();
    assert_eq!(
        served,
        json!({"key": "person:ada@example.com", "kind": "person", "stamp": served["stamp"],
               "sources": {"slack": {"name": "Ada"}, "vault": {"name": "Ada L"}}})
    );
    assert_eq!(book.everyone().count(), 1);
}

// A daemon started again, as a switch does, hears everyone anew. What
// it serves of someone whose sources say the same reads as it did, so
// the shell keeps their row rather than making it again: stamped with
// the moment each was heard, Joel's first letter after such a switch
// made 136 and 137 people's rows (his journal, 2026-11-14 09:38 and
// 17:07), every one alike but for that moment.
#[test]
fn someone_said_the_same_reads_the_same_whenever_they_were_heard() {
    let ada = |name: &str| Heard::Person {
        address: "ada@example.com".to_owned(),
        entry: json!({"name": name}),
    };
    let served = |names: &[&str]| {
        let mut book = Book::of(&["slack".to_owned()]);
        for name in names {
            book.take("slack", ada(name));
        }
        book.record("person:ada@example.com").unwrap().json()
    };
    assert_eq!(
        served(&["Ada L", "Ada"]),
        served(&["Ada"]),
        "heard again, whatever came between, as before"
    );
    assert_ne!(
        served(&["Ada L"])["stamp"],
        served(&["Ada"])["stamp"],
        "what's said moved, so its stamp does"
    );
}

#[test]
fn a_calendars_address_is_no_one_and_answers_at_once() {
    let mut book = Book::of(&["google".to_owned()]);
    let dates = "c_1f@group.calendar.google.com";
    let heard = Heard::Person {
        address: dates.to_owned(),
        entry: json!({"name": "Important Dates"}),
    };
    assert_eq!(book.take("google", heard), None);
    assert_eq!(book.everyone().count(), 0, "a calendar is held as no one");
    assert!(
        book.ready(&key(dates)),
        "a step-in on a calendar was left waiting on {:?}",
        book.waiting(&key(dates)).collect::<Vec<_>>()
    );
}

#[test]
fn a_source_signed_out_or_refused_is_never_waited_on_and_one_started_again_is() {
    let mut book = Book::of(&["slack".to_owned(), "vault".to_owned()]);
    book.take(
        "slack",
        Heard::Person {
            address: "ada@example.com".to_owned(),
            entry: json!({}),
        },
    );
    book.take("vault", Heard::SignedOut);
    assert_eq!(
        book.waiting("person:ada@example.com").collect::<Vec<_>>(),
        Vec::<&str>::new(),
        "Ada still waited, though Slack had answered and Vault had signed out"
    );
    assert_eq!(book.standings()["vault"].word(), "signed-out");
    assert_eq!(
        Book::of(&["tuple".to_owned()]).standings()["tuple"].word(),
        "starting"
    );
    book.restarted("slack");
    assert_eq!(
        book.waiting("person:ada@example.com").collect::<Vec<_>>(),
        ["slack"],
        "Ada wasn't waiting on Slack alone, though it had started again and not yet answered"
    );
    assert_eq!(book.standings()["slack"].word(), "answering");
    // One refused, its program stopped, is waited on again once restarted.
    book.stand("vault", Standing::Refused("it stopped".to_owned()));
    assert_eq!(
        book.waiting("person:ada@example.com").collect::<Vec<_>>(),
        ["slack"],
        "Ada wasn't waiting on Slack alone while Vault's program was stopped"
    );
    book.restarted("vault");
    assert_eq!(
        book.waiting("person:ada@example.com").collect::<Vec<_>>(),
        ["slack", "vault"],
        "Ada wasn't waiting on just Slack and Vault once Vault's program started again"
    );
    assert_eq!(book.standings()["vault"].word(), "answering");
}

fn tuple(address: &str, id: u32, name: &str, favourite: bool) -> Heard {
    Heard::Person {
        address: address.to_owned(),
        entry: json!({"id": id, "name": name, "status": "unknown",
                      "favourite": favourite, "recent": false}),
    }
}

#[test]
fn a_source_that_says_its_a_roster_isnt_waited_on_and_knows_some_by_its_own_id() {
    let mut book = Book::of(&["slack".to_owned(), "tuple".to_owned()]);
    book.take("tuple", Heard::Roster);
    book.take("tuple", tuple("", 7, "Gus", false));
    assert!(
        book.record("person:tuple:7").is_some(),
        "Gus, whom the roster knows only by Tuple's id, had no record"
    );
    // Only Slack, which is asked, is waited on.
    assert_eq!(
        book.waiting("person:ada@example.com").collect::<Vec<_>>(),
        ["slack"]
    );
    book.take(
        "slack",
        Heard::Later {
            address: "ada@example.com".to_owned(),
        },
    );
    // Slack is answering, and has nothing to say of Ada yet, so she still
    // waits on it until it says something of her.
    assert_eq!(book.standings()["slack"].word(), "answering");
    assert_eq!(
        book.waiting("person:ada@example.com").collect::<Vec<_>>(),
        ["slack"],
        "Ada wasn't waiting on Slack alone once it had nothing to say of her yet"
    );
    book.take(
        "slack",
        Heard::Person {
            address: "ada@example.com".to_owned(),
            entry: json!({}),
        },
    );
    assert!(
        book.ready("person:ada@example.com"),
        "Ada waited on {:?} once Slack had answered",
        book.waiting("person:ada@example.com").collect::<Vec<_>>()
    );
    // Started again, Tuple says anew whether it's a roster, and is waited
    // on until it does.
    book.restarted("tuple");
    assert_eq!(
        book.waiting("person:ada@example.com").collect::<Vec<_>>(),
        ["tuple"],
        "Ada wasn't waiting on Tuple alone once it started again"
    );
    book.take("tuple", Heard::Roster);
    assert_eq!(
        book.waiting("person:ada@example.com").collect::<Vec<_>>(),
        Vec::<&str>::new(),
        "Ada still waited once Tuple said again it's a roster"
    );
}

#[test]
fn someone_a_source_forgets_keeps_what_the_others_say_or_goes() {
    let mut book = Book::of(&["slack".to_owned(), "tuple".to_owned()]);
    book.take("tuple", tuple("ada@example.com", 1, "Ada", false));
    book.take("tuple", tuple("eli@example.com", 2, "Eli", false));
    book.take(
        "slack",
        Heard::Person {
            address: "ada@example.com".to_owned(),
            entry: json!({}),
        },
    );
    let gone = |address: &str| Heard::Gone {
        address: address.to_owned(),
        id: None,
    };
    assert_eq!(
        book.take("tuple", gone("ada@example.com")),
        Some("person:ada@example.com".to_owned())
    );
    assert_eq!(
        book.record("person:ada@example.com")
            .unwrap()
            .sources
            .keys()
            .collect::<Vec<_>>(),
        ["slack"]
    );
    assert_eq!(
        book.take("tuple", gone("eli@example.com")),
        Some("person:eli@example.com".to_owned())
    );
    assert!(
        book.record("person:eli@example.com").is_none(),
        "Eli kept a record once the only source that knew him forgot him"
    );
    assert_eq!(book.everyone().count(), 1);
    assert_eq!(book.take("tuple", gone("eli@example.com")), None);
}

#[test]
fn words_start_words_of_a_name_or_an_address_as_the_shells_fuzzy_says() {
    let find = |address: &str, name: &str, typed: &str| {
        let mut book = Book::of(&["tuple".to_owned()]);
        book.take("tuple", tuple(address, 1, name, false));
        !book.search(typed, 1, 0.0).is_empty()
    };
    for (address, name, typed, found) in [
        ("a@b.c", "Ada Brightwater", "bri ad", true),
        ("a@b.c", "Ada Brightwater", "ADA", true),
        ("a@b.c", "Ada Brightwater", "water", false),
        ("ada.brightwater@example.com", "Q", "bright", true),
        ("ada@example.com", "Q", "exam", true),
        ("a@b.c", "McKenzie", "ken", true),
        ("a@b.c", "agent007", "007", true),
        ("a@b.c", "Zoë Ångström", "ång", true),
        ("a@b.c", "Ada", "adam", false),
        ("a@b.c", "Ada", "  ", false),
    ] {
        assert_eq!(
            find(address, name, typed),
            found,
            "{typed:?} in {name} <{address}>"
        );
    }
}

#[test]
fn a_search_finds_them_by_the_better_of_their_name_and_their_address() {
    let mut book = Book::of(&["tuple".to_owned()]);
    // "ada" scores 88 in "Adams Ray", 84 in "McAda Ray" and 96 in
    // "ada@example.com", so the later one's address puts them first.
    book.take("tuple", tuple("x1@example.com", 1, "Adams Ray", false));
    book.take("tuple", tuple("ada@example.com", 2, "McAda Ray", false));
    let keys: Vec<_> = book
        .search("ada", 10, 0.0)
        .iter()
        .map(|record| record.key.clone())
        .collect();
    assert_eq!(
        keys,
        ["person:ada@example.com", "person:x1@example.com"],
        "the better of each one's two"
    );
}

#[test]
fn a_search_finds_them_by_the_name_their_row_shows_alone() {
    let mut book = Book::of(&["tuple".to_owned(), "slack".to_owned()]);
    let person = |address: &str, name: &str| Heard::Person {
        address: address.to_owned(),
        entry: json!({"name": name}),
    };
    book.take("slack", person("rob@example.com", "Bob Smith"));
    book.take("tuple", person("rob@example.com", "Robert Smith"));
    book.take("slack", person("chao.duan@example.com", ""));
    let found = |typed| book.search(typed, 10, 0.0).len();
    // Tuple is consulted first, so their row shows Robert, and the
    // shell, matching the row, never finds them as Bob.
    assert_eq!(found("robert"), 1, "by the name shown");
    assert_eq!(found("bob"), 0, "not by a name another source gives");
    // One no source names is shown, and found, as Chao Duan: their
    // address finds them too, so the name searched is read itself.
    assert_eq!(found("chao duan"), 1, "by the name made of their address");
    assert_eq!(
        book.record("person:chao.duan@example.com")
            .map(|chao| chao.named.text.as_str()),
        Some("Chao Duan"),
        "the name searched is the one made of their address"
    );
}

#[test]
fn one_kept_by_a_sources_id_is_found_by_their_name_alone() {
    let mut book = Book::of(&["tuple".to_owned()]);
    book.take("tuple", Heard::Roster);
    book.take("tuple", tuple("", 7, "Gus", false));
    let found = |typed| book.search(typed, 10, 0.0).len();
    assert_eq!(found("gus"), 1, "by their name");
    // Their row says no address, so the shell never finds them by
    // the key they're kept by.
    assert_eq!(found("tuple"), 0, "not by the key they're kept by");
}

#[test]
fn one_a_source_forgets_is_found_by_what_the_rest_say() {
    let mut book = Book::of(&["tuple".to_owned(), "slack".to_owned()]);
    book.take("tuple", tuple("rob@example.com", 1, "Robert Smith", true));
    book.take(
        "slack",
        Heard::Person {
            address: "rob@example.com".to_owned(),
            entry: json!({"name": "Bob Smith"}),
        },
    );
    book.take(
        "tuple",
        Heard::Gone {
            address: "rob@example.com".to_owned(),
            id: Some("1".to_owned()),
        },
    );
    let found = |typed| book.search(typed, 10, 0.0).len();
    assert_eq!(
        found("robert"),
        0,
        "not by the name only the forgetting one gave"
    );
    assert_eq!(found("bob"), 1, "by the name the rest give");
    assert_eq!(
        book.record("person:rob@example.com").map(|rob| rob.listed),
        Some(false),
        "no longer a favourite, as only the forgetting one said"
    );
}

#[test]
fn a_search_lists_first_one_listed_once_something_with_them_began() {
    let mut book = Book::of(&["tuple".to_owned(), "google".to_owned()]);
    book.take("tuple", tuple("ada@example.com", 1, "Ada", false));
    book.take(
        "google",
        Heard::Person {
            address: "lane@example.com".to_owned(),
            entry: json!({"name": "Ada Lane", "spans": [{"from": 400.0, "to": 500.0},
                {"from": 100.0, "to": 200.0}]}),
        },
    );
    let keys = |now| {
        book.search("ada", 10, now)
            .iter()
            .map(|record| record.key.clone())
            .collect::<Vec<_>>()
    };
    // Both score 96, so they're in the order heard until their first
    // meeting begins, as those listed list Ada Lane from then on.
    assert_eq!(
        keys(50.0),
        ["person:ada@example.com", "person:lane@example.com"],
        "before it began"
    );
    assert_eq!(
        keys(100.0),
        ["person:lane@example.com", "person:ada@example.com"],
        "as it begins"
    );
    assert_eq!(
        keys(300.0),
        ["person:lane@example.com", "person:ada@example.com"],
        "once it ended, before the next"
    );
}

#[test]
fn a_search_ranks_the_listed_first_then_the_better_found() {
    let mut book = Book::of(&["tuple".to_owned()]);
    // Fuzzy.js scores "ada" 84 in "McAda Ray", at a capital inside a
    // word, 88 in "Adams", which runs on past it, and 96 in "Ada".
    book.take("tuple", tuple("mc@example.com", 1, "McAda Ray", false));
    book.take("tuple", tuple("adams@example.com", 2, "Adams", true));
    book.take("tuple", tuple("ada@example.com", 3, "Ada", false));
    let keys: Vec<_> = book
        .search("ada", 10, 0.0)
        .iter()
        .map(|record| record.key.clone())
        .collect();
    assert_eq!(
        keys,
        [
            "person:adams@example.com",
            "person:ada@example.com",
            "person:mc@example.com"
        ],
        "the listed, then the better found"
    );
}

#[test]
fn a_search_finds_by_any_name_or_address_and_cuts_once_ranked() {
    let mut book = Book::of(&["tuple".to_owned()]);
    book.take("tuple", tuple("dale@example.com", 1, "Ada's Friend", false));
    book.take("tuple", tuple("ada.k@example.com", 2, "Kim", false));
    book.take("tuple", tuple("eli@example.com", 3, "Ada Eli", false));
    book.take("tuple", tuple("x@example.com", 4, "Adams Ray", true));
    book.take("tuple", tuple("gus@example.com", 5, "Gus", false));
    let keys = |limit| {
        book.search("ada", limit, 0.0)
            .iter()
            .map(|record| record.key.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        keys(10),
        // Kim's address finds them as well as the others' names do
        // (96 each), so they're in the order heard.
        [
            "person:x@example.com",
            "person:dale@example.com",
            "person:ada.k@example.com",
            "person:eli@example.com"
        ]
    );
    assert_eq!(
        keys(2),
        ["person:x@example.com", "person:dale@example.com"],
        "cut to the limit once ranked"
    );
    assert!(
        book.record("person:x@example.com").unwrap().listed(),
        "Adams Ray, a favourite, wasn't listed"
    );
    assert!(
        !book.record("person:gus@example.com").unwrap().listed(),
        "Gus, no favourite, was listed"
    );
}
