use super::*;
use repos::model::{Checkout, Repository};

/// zoxide's database at version 3 holding one folder, visited at `seconds`.
fn zoxide(folder: &str, seconds: u64) -> Vec<u8> {
    let mut bytes = 3_u32.to_le_bytes().to_vec();
    bytes.extend(1_u64.to_le_bytes());
    bytes.extend((folder.len() as u64).to_le_bytes());
    bytes.extend(folder.as_bytes());
    bytes.extend(1.0_f64.to_le_bytes());
    bytes.extend(seconds.to_le_bytes());
    bytes
}

/// One repository, alpha, checked out at `path`.
fn alpha_at(path: &str) -> Record {
    let checkout = Checkout {
        path: path.to_owned(),
        bare: false,
        head: None,
        written_at: None,
        trees: Vec::new(),
        weight: None,
        branches: None,
        config_bytes: None,
    };
    Record {
        schema: repos::model::SCHEMA.to_owned(),
        read_at: 0,
        machine: "here".to_owned(),
        roots: Vec::new(),
        sources: Vec::new(),
        repositories: vec![Repository {
            key: "github:someone/alpha".to_owned(),
            name: "alpha".to_owned(),
            owner: None,
            remotes: Vec::new(),
            trunk: None,
            written_at: None,
            checkouts: vec![checkout],
            changes: Vec::new(),
            read_through: None,
        }],
    }
}

/// A resolution held past its deadline leaves zoxide's database read and
/// its visits counted as written, so a folder on a stuck mount never
/// empties Code's order as a stuck database does, which orders nothing.
#[test]
fn reads_and_resolutions_held_late_are_counted_apart() {
    let folder = std::env::temp_dir().join(format!("code-late-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let database = folder.join("db.zo");
    std::fs::write(&database, zoxide("/s/alpha/src", 1_700_000_000)).unwrap();
    let mut code = Code::new("", &Value::Null);
    code.visited = Some(database);
    let record = Arc::new(alpha_at("/s/alpha"));
    // As a resolution stuck on a mount leaves it.
    code.late.fetch_add(1, Ordering::SeqCst);
    let resolving_late = code.last_visits(&record);
    // As a read stuck on a mount leaves it.
    code.late_reads.fetch_add(1, Ordering::SeqCst);
    let reading_late = code.last_visits(&record);
    std::fs::remove_dir_all(&folder).unwrap();
    assert_eq!(
        resolving_late,
        BTreeMap::from([("github:someone/alpha".to_owned(), 1_700_000_000_000)]),
        "a resolution held late left the visits uncounted as written"
    );
    assert!(
        reading_late.is_empty(),
        "a read held late still ordered Code: {reading_late:?}"
    );
}

#[test]
fn a_pass_begun_before_the_one_last_taken_takes_nothing() {
    let taken = AtomicU64::new(0);
    let took = [
        newest(&taken, 2),
        newest(&taken, 1),
        newest(&taken, 2),
        newest(&taken, 3),
    ];
    assert_eq!(
        took,
        [true, false, true, true],
        "a pass that began earlier took its rows over a later one's"
    );
}

/// Each pass on the repositories held is numbered after the one that
/// held them, and a pass on ones held later after both, so `newest`
/// never takes an older set's rows over a newer's.
#[test]
fn a_pass_on_repositories_held_later_is_numbered_later() {
    let code = Code::new("", &Value::Null);
    let held = |machine: &str| {
        Arc::new(Record {
            schema: repos::model::SCHEMA.to_owned(),
            read_at: 0,
            machine: machine.to_owned(),
            roots: Vec::new(),
            sources: Vec::new(),
            repositories: Vec::new(),
        })
    };
    assert!(
        code.latest().is_none(),
        "a pass was numbered before anything was held"
    );
    let first = code.hold(&held("first"));
    let (record, on_first) = code.latest().expect("what was held wasn't found");
    assert_eq!(record.machine, "first");
    let second = code.hold(&held("second"));
    let (record, on_second) = code.latest().expect("what was held wasn't found");
    assert_eq!(
        record.machine, "second",
        "the repositories held last weren't found"
    );
    assert!(
        first < on_first && on_first < second && second < on_second,
        "passes weren't numbered in the order their repositories were held: \
         {first} {on_first} {second} {on_second}"
    );
}

#[test]
fn a_count_git_couldnt_read_keeps_the_last_and_one_no_longer_counted_goes() {
    let held = BTreeMap::from([
        ("/src/a".to_owned(), 2),
        ("/src/b".to_owned(), 3),
        ("/src/gone".to_owned(), 4),
    ]);
    let counted = vec![("/src/a", Some(0)), ("/src/b", None), ("/src/new", None)];
    assert_eq!(
        kept(&held, counted),
        BTreeMap::from([("/src/a".to_owned(), 0), ("/src/b".to_owned(), 3)]),
        "a count git couldn't read wasn't kept at its last, or one no longer counted stayed"
    );
}

/// A checkout already held is refused, the first hold kept, and let
/// go once it ends. Asked on a thread of its own, so a hold that
/// locks itself fails the case rather than hanging it.
#[test]
fn a_checkout_held_refuses_another_and_keeps_its_hold() {
    let (told, heard) = mpsc::channel();
    std::thread::spawn(move || {
        let code = Code::new("", &Value::Null);
        let first = Holding::take(&code, "/src/alpha");
        let second = Holding::take(&code, "/src/alpha").is_some();
        let kept = lock(&code.busy).contains("/src/alpha");
        drop(first);
        let freed = Holding::take(&code, "/src/alpha").is_some();
        told.send((second, kept, freed)).unwrap();
    });
    let said = heard
        .recv_timeout(Duration::from_secs(5))
        .expect("a second hold on a held checkout never came back");
    assert_eq!(said, (false, true, true));
}
