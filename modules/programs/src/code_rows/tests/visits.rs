//! Code's rows in the order Joel's own visits give: his newest visit to a
//! checkout, a tree or a folder below either counts for its repository, a
//! nested checkout keeps its own, a tree at the root or gone holds none,
//! and a tree written through a link holds a visit to its real folder.

use super::*;

/// A repository of `name` with one checkout at `path` and its one tree at
/// `tree`, which git wrote in at `written`.
fn named(name: &str, path: &str, tree: &str, written: u64) -> Repository {
    let mut repository = core();
    repository.key = format!("github:Jitsusama/{name}");
    repository.name = name.to_owned();
    repository.written_at = Some(written);
    repository.checkouts = vec![checkout(path, false, vec![super::tree(tree, true)])];
    repository
}

/// The rows of `record` ordered by `visited`, each folder counted where
/// its place resolves, as the daemon counts them.
fn visited_rows(record: &Record, visited: &[(String, u64)]) -> Vec<Value> {
    let last = last_visits(&places(record), visited);
    crate::code_rows::rows(record, None, &words(), &BTreeMap::new(), &last)
}

#[test]
fn joels_own_last_visit_orders_them_and_the_rest_go_a_to_z_whatever_git_wrote() {
    let record = record(vec![
        named("Delta", "/s/delta", "/t/delta", 900),
        named("charlie", "/s/charlie", "/t/charlie", 800),
        named("beta", "/s/beta", "/t/beta", 100),
        named("gamma", "/s/gamma", "/t/gamma", 200),
        named("alpha", "/s/alpha", "/t/alpha", 50),
    ]);
    // alpha's older visit comes first and gamma's last, so only the newest
    // of each repository's visits, wherever it's listed, puts alpha between
    // gamma and beta.
    let visited = [
        ("/s/alpha".to_owned(), 10_000),
        ("/s/beta-old".to_owned(), 900_000),
        ("/t/gamma".to_owned(), 300_000),
        ("/s/beta/src".to_owned(), 200_000),
        ("/t/alpha".to_owned(), 250_000),
        ("/s/delta-two".to_owned(), 950_000),
        ("/s/gamma".to_owned(), 100_000),
    ];
    let rows = visited_rows(&record, &visited);
    let names: Vec<&str> = rows.iter().filter_map(|row| row["name"].as_str()).collect();
    assert_eq!(
        names,
        ["gamma", "alpha", "beta", "charlie", "Delta"],
        "a visit in a tree or below a checkout is his act on it, his newest counts, a \
         folder merely named alike is not, the rest go A to Z with case alike, and git's \
         writes order nothing"
    );
}

#[test]
fn a_visit_in_a_checkout_nested_in_another_counts_for_the_nested_one() {
    let record = record(vec![
        named("outer", "/w/git", "/w/trees/one", 1),
        named("inner", "/w/git/vendor/inner", "/t/inner", 2),
        named("aside", "/s/aside/", "/t/aside", 3),
    ]);
    let visited = [
        ("/w/git/vendor/inner/src".to_owned(), 500),
        ("/s/aside".to_owned(), 400),
        ("/w/git/".to_owned(), 450),
    ];
    let rows = visited_rows(&record, &visited);
    let names: Vec<&str> = rows.iter().filter_map(|row| row["name"].as_str()).collect();
    assert_eq!(
        names,
        ["inner", "outer", "aside"],
        "the visit below both is the nested one's alone, and a trailing slash on either \
         side still counts"
    );
}

#[test]
fn a_tree_at_the_root_or_gone_holds_no_visit() {
    let mut gone = named("gone", "/s/gone", "/w/gone", 1);
    gone.checkouts[0].trees[0].present = false;
    let record = record(vec![
        named("rooted", "/s/rooted", "/", 2),
        gone,
        named("visited", "/s/visited", "/t/visited", 3),
    ]);
    let visited = [
        ("/elsewhere".to_owned(), 900),
        ("/w/gone".to_owned(), 800),
        ("/s/visited".to_owned(), 100),
    ];
    let rows = visited_rows(&record, &visited);
    let names: Vec<&str> = rows.iter().filter_map(|row| row["name"].as_str()).collect();
    assert_eq!(
        names,
        ["visited", "gone", "rooted"],
        "a tree at / held every folder, or a tree gone from the disk still counted"
    );
}

#[test]
fn places_as_written_are_trimmed_and_none_is_the_root() {
    let record = record(vec![
        named("alpha", "/s/alpha/", "/t/alpha", 1),
        named("rooted", "/s/rooted", "/", 2),
    ]);
    assert_eq!(
        written_places(&record),
        [
            ("/s/alpha".to_owned(), "github:Jitsusama/alpha"),
            ("/t/alpha".to_owned(), "github:Jitsusama/alpha"),
            ("/s/rooted".to_owned(), "github:Jitsusama/rooted"),
        ],
        "a place as written kept its trailing slash, or a tree at / held every visit"
    );
}

#[test]
fn a_tree_written_through_a_link_holds_a_visit_to_its_real_folder() {
    let folder = std::env::temp_dir().join(format!("code-rows-link-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(folder.join("real/tree/src")).unwrap();
    std::os::unix::fs::symlink(folder.join("real"), folder.join("linked")).unwrap();
    let written = folder.join("linked/tree").to_string_lossy().into_owned();
    let real = std::fs::canonicalize(folder.join("real/tree/src")).unwrap();
    let record = record(vec![
        named("alpha", "/s/alpha", "/t/alpha", 1),
        named("linked", "/s/linked", &written, 2),
    ]);
    let visited = [(real.to_string_lossy().into_owned(), 100)];
    let rows = visited_rows(&record, &visited);
    std::fs::remove_dir_all(&folder).unwrap();
    let names: Vec<&str> = rows.iter().filter_map(|row| row["name"].as_str()).collect();
    assert_eq!(
        names,
        ["linked", "alpha"],
        "a visit to a tree's real folder didn't count where repos wrote it through a link"
    );
}
