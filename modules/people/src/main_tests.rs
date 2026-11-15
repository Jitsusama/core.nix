//! The daemon's own order of telling, with no socket between: a watch's
//! whole answer goes before any change gathered after it.

use super::*;

#[test]
fn a_watch_is_answered_whole_before_any_change_told_it() {
    let shared: Shared = Arc::new((
        Mutex::new(Daemon {
            book: Book::of(&[]),
            inputs: HashMap::new(),
            ahead: HashMap::new(),
            put_off: HashMap::new(),
            stepping: HashMap::new(),
            watches: Vec::new(),
            took: VecDeque::new(),
            since: now(),
            sources: Vec::new(),
            named: Named::read(&Value::Null),
            holds: Holds::default(),
            revision: 0,
        }),
        Condvar::new(),
    ));
    let (told, lines) = mpsc::channel::<String>();
    let first = answer(&shared, ask(r#"{"id":"all","ask":"watch"}"#), 1, &told);
    // A change gathered once the answer's lock is let go.
    {
        let mut daemon = lock(&shared);
        let someone = json!({"key": "someone@example.com"});
        for watch in &mut daemon.watches {
            watch
                .pending
                .insert("someone@example.com".to_owned(), someone.clone());
        }
        watching::say_gathered(&mut daemon);
    }
    if let Some(first) = first {
        told.send(first.to_string()).expect("told");
    }
    let said: Vec<Value> = lines
        .try_iter()
        .map(|line| serde_json::from_str(&line).expect("json"))
        .collect();
    assert_eq!(said.len(), 2, "{said:?}");
    assert!(said[0]["records"].is_array(), "{said:?}");
    assert_eq!(
        said[1]["changed"][0]["key"], "someone@example.com",
        "{said:?}"
    );
}
