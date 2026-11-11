//! What kitty's page reads over the socket, of zoxide's database, ssh's
//! files and zsh's history: each read as it's asked, and said again as its
//! file changes while watched.

use serde_json::json;

use super::Running;

#[test]
fn what_kittys_page_reads_of_zoxide_and_ssh_is_read_as_each_is_asked() {
    let running = Running::new("kitty");
    let tide = running.path("tide");
    std::fs::create_dir_all(&tide).unwrap();
    let named = tide.display().to_string();
    let mut database = 3_u32.to_le_bytes().to_vec();
    database.extend(1_u64.to_le_bytes());
    database.extend(u64::try_from(named.len()).unwrap().to_le_bytes());
    database.extend(named.as_bytes());
    database.extend(1.0_f64.to_le_bytes());
    database.extend(1_700_000_000_u64.to_le_bytes());
    std::fs::write(running.path("db.zo"), database).unwrap();
    let ssh = running.path(".ssh");
    std::fs::create_dir_all(&ssh).unwrap();
    std::fs::write(ssh.join("config"), "Host reef\n").unwrap();
    let history = running.path("history");
    std::fs::write(&history, ": 1700000000:0;ls\n: 1700000000:0;ssh reef\n").unwrap();
    let keys = [
        format!("visited:{}", running.path("db.zo").display()),
        format!("machines:{}", ssh.display()),
        format!("reached:{}", history.display()),
    ];
    let mut client = running.client();
    let said = client.ask(&json!({"id": 1, "ask": "get", "keys": keys}));
    let records = &said["records"];
    assert_eq!(
        records[0]["folders"],
        json!([{"path": named, "at": 1_700_000_000_000_u64}]),
        "zoxide's folder wasn't said with when it was seen: {said}"
    );
    assert_eq!(
        (&records[1]["config"], &records[1]["hosts"]),
        (&json!("Host reef\n"), &json!("")),
        "ssh's files weren't said as written: {said}"
    );
    assert_eq!(
        records[2]["lines"],
        json!([": 1700000000:0;ssh reef"]),
        "the history's ssh lines weren't said alone: {said}"
    );
    std::fs::write(ssh.join("known_hosts"), "shoal ssh-ed25519 AAAA\n").unwrap();
    let again = client.ask(&json!({"id": 2, "ask": "get", "keys": [keys[1]]}));
    assert_eq!(
        again["records"][0]["hosts"],
        json!("shoal ssh-ed25519 AAAA\n"),
        "known_hosts written since wasn't read as it was asked again: {again}"
    );
}

#[test]
fn what_kittys_page_reads_is_said_again_as_its_file_changes_while_watched() {
    let running = Running::new("kitty-heard");
    // ssh's folder is made only once its key is watched, as a new account's is.
    let ssh = running.path(".ssh");
    let history = running.path("history");
    std::fs::write(&history, ": 1700000000:0;ssh reef\n").unwrap();
    let reached = format!("reached:{}", history.display());
    let machines = format!("machines:{}", ssh.display());
    let mut hearing = running.client();
    hearing.ask(&json!({"id": 1, "ask": "watch", "kind": "reached", "keys": [reached]}));
    let first = hearing.changed(&reached);
    assert_eq!(
        first["lines"].as_array().map_or(0, Vec::len),
        1,
        "the history wasn't read as the watch began: {first}"
    );
    // Another file in the history's folder, then the history itself.
    std::fs::write(running.path("elsewhere"), "not the history").unwrap();
    std::fs::write(
        &history,
        ": 1700000000:0;ssh reef\n: 1700000001:0;ssh shoal\n",
    )
    .unwrap();
    let again = hearing.changed(&reached);
    assert_eq!(
        again["lines"],
        json!([": 1700000000:0;ssh reef", ": 1700000001:0;ssh shoal"]),
        "a command run since wasn't heard in the history: {again}"
    );
    let mut knowing = running.client();
    knowing.ask(&json!({"id": 1, "ask": "watch", "kind": "machines", "keys": [machines]}));
    knowing.changed(&machines);
    std::fs::create_dir_all(&ssh).unwrap();
    std::fs::write(ssh.join("known_hosts"), "shoal ssh-ed25519 AAAA\n").unwrap();
    let known = knowing.changed(&machines);
    assert_eq!(
        known["hosts"],
        json!("shoal ssh-ed25519 AAAA\n"),
        "a host known since wasn't heard in ssh's folder: {known}"
    );
}
