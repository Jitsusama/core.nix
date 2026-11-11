//! 1Password as a password manager of Passwords (modules/passwords): its
//! items read through its own command, `op`, and said as entries in
//! Passwords' shape, every hidden detail's value taken off before it goes.
//!
//! `op item get --format json` gives every concealed field's value
//! whether or not `--reveal` is asked (1Password's forum, 2026-04-13), and
//! a password's history beside it, so nothing here passes an item on as
//! op gives it: [`entry`] builds each entry afresh from what's safe to
//! show, and only [`value`] reads a secret, for the one detail asked, and
//! [`passwords`] every login's password, for an audit Joel asked for.

use serde_json::{Map, Value, json};

/// An entry's kind, by 1Password's category: Passwords' own word where
/// one is shared by every password manager, else 1Password's, lowercased
/// and in Canadian spelling.
pub fn category(op: &str) -> String {
    match op {
        "LOGIN" => "login".to_owned(),
        "PASSWORD" => "password".to_owned(),
        "SECURE_NOTE" => "note".to_owned(),
        "CREDIT_CARD" => "card".to_owned(),
        "IDENTITY" => "identity".to_owned(),
        "SSH_KEY" => "key".to_owned(),
        "API_CREDENTIAL" => "credential".to_owned(),
        "DRIVER_LICENSE" => "driver's licence".to_owned(),
        "OUTDOOR_LICENSE" => "outdoor licence".to_owned(),
        "SOFTWARE_LICENSE" => "software licence".to_owned(),
        other => other.to_lowercase().replace('_', " "),
    }
}

/// A detail's kind, by 1Password's field type: hidden for any that
/// 1Password conceals, its private keys and card numbers among them, and
/// code for a one-time password, whose seed is as secret as a password.
pub fn kind(op: &str) -> &'static str {
    match op {
        "CONCEALED" | "SSHKEY" | "CREDIT_CARD_NUMBER" => "hidden",
        "OTP" => "code",
        "EMAIL" => "email",
        "URL" => "url",
        "DATE" => "date",
        "MONTH_YEAR" => "month",
        "PHONE" => "phone",
        "REFERENCE" => "reference",
        _ => "text",
    }
}

/// What a detail is for, where 1Password says.
fn purpose(op: &str) -> Option<&'static str> {
    match op {
        "USERNAME" => Some("username"),
        "PASSWORD" => Some("password"),
        "NOTES" => Some("notes"),
        _ => None,
    }
}

fn text<'a>(value: &'a Value, name: &str) -> Option<&'a str> {
    value.get(name).and_then(Value::as_str)
}

/// Milliseconds since the epoch of one of op's times (RFC 3339, in UTC
/// as op writes them), or none.
pub fn millis(time: &str) -> Option<u64> {
    // 2026-11-07T20:21:35Z, or with a fraction or an offset of +00:00.
    let (date, clock) = time.split_once('T')?;
    let mut date = date.splitn(3, '-').map(str::parse::<i64>);
    let (year, month, day) = (date.next()?.ok()?, date.next()?.ok()?, date.next()?.ok()?);
    let clock = clock.trim_end_matches('Z');
    let clock = clock.split(['+', '-']).next()?;
    let mut parts = clock.splitn(3, ':');
    let hour: i64 = parts.next()?.parse().ok()?;
    let minute: i64 = parts.next()?.parse().ok()?;
    // Seconds, and the first three digits of any fraction as milliseconds.
    let seconds = parts.next()?;
    let (whole, fraction) = seconds.split_once('.').unwrap_or((seconds, ""));
    let second: i64 = whole.parse().ok()?;
    let fraction: String = fraction.chars().chain("000".chars()).take(3).collect();
    let thousandths: i64 = fraction.parse().ok()?;
    // Days since the epoch by the civil calendar (Howard Hinnant's).
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let of_era = year - era * 400;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let of_era_days = of_era * 365 + of_era / 4 - of_era / 100 + day_of_year;
    let days = era * 146_097 + of_era_days - 719_468;
    let seconds = days * 86_400 + hour * 3_600 + minute * 60 + second;
    u64::try_from(seconds * 1000 + thousandths).ok()
}

/// An item as Passwords holds an entry: in summary from a list, or with
/// every detail from a get, every hidden detail without its value, and
/// 1Password's own facts under `own`.
pub fn entry(item: &Value) -> Option<Value> {
    let id = text(item, "id").filter(|id| !id.is_empty())?;
    let mut entry = Map::new();
    entry.insert("id".to_owned(), json!(id));
    entry.insert("title".to_owned(), json!(text(item, "title").unwrap_or("")));
    entry.insert(
        "category".to_owned(),
        json!(category(text(item, "category").unwrap_or(""))),
    );
    if let Some(version) = item.get("version").and_then(Value::as_u64) {
        entry.insert("version".to_owned(), json!(version));
    }
    if let Some(vault) = item.get("vault") {
        entry.insert(
            "collection".to_owned(),
            json!({"id": text(vault, "id"), "name": text(vault, "name")}),
        );
    }
    entry.insert(
        "favourite".to_owned(),
        json!(item.get("favorite").and_then(Value::as_bool) == Some(true)),
    );
    entry.insert(
        "tags".to_owned(),
        item.get("tags")
            .filter(|tags| tags.is_array())
            .cloned()
            .unwrap_or(json!([])),
    );
    let websites: Vec<Value> = item
        .get("urls")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|url| {
            let address = text(url, "href")?;
            Some(json!({
                "address": address,
                "label": text(url, "label").unwrap_or(""),
                "primary": url.get("primary").and_then(Value::as_bool) == Some(true),
            }))
        })
        .collect();
    entry.insert("websites".to_owned(), json!(websites));
    for (from, to) in [("created_at", "created"), ("updated_at", "changed")] {
        if let Some(at) = text(item, from).and_then(millis) {
            entry.insert(to.to_owned(), json!(at));
        }
    }
    let fields = item.get("fields").and_then(Value::as_array);
    // A login's username, which a list gives as its additional information.
    let username = fields
        .into_iter()
        .flatten()
        .find(|field| text(field, "purpose") == Some("USERNAME"))
        .and_then(|field| text(field, "value"))
        .or_else(|| {
            (text(item, "category") == Some("LOGIN"))
                .then(|| text(item, "additional_information"))
                .flatten()
        });
    if let Some(username) = username.filter(|username| !username.is_empty()) {
        entry.insert("username".to_owned(), json!(username));
    }
    if let Some(fields) = fields {
        let sections: Vec<Value> = item
            .get("sections")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let details: Vec<Value> = fields
            .iter()
            .filter_map(|field| detail(field, &sections))
            .collect();
        entry.insert("details".to_owned(), json!(details));
    }
    let mut own = Map::new();
    if let Some(edited) = text(item, "last_edited_by") {
        own.insert("last edited by".to_owned(), json!(edited));
    }
    if let Some(summary) = text(item, "additional_information")
        && !summary.is_empty()
    {
        own.insert("summary".to_owned(), json!(summary));
    }
    if let Some(files) = item.get("files").and_then(Value::as_array) {
        own.insert("files".to_owned(), json!(files.len()));
    }
    entry.insert("own".to_owned(), Value::Object(own));
    Some(Value::Object(entry))
}

/// One field as a detail: its id, label, section, kind and purpose, and
/// its value only when it isn't hidden. A password's strength is said,
/// since it's no secret, but never its history, which is old passwords.
fn detail(field: &Value, sections: &[Value]) -> Option<Value> {
    let id = text(field, "id")?;
    let op_type = text(field, "type").unwrap_or("STRING");
    let kind = kind(op_type);
    let hidden = matches!(kind, "hidden" | "code");
    let label = text(field, "label")
        .filter(|label| !label.is_empty())
        .unwrap_or(id);
    let section = field
        .get("section")
        .and_then(|section| text(section, "id"))
        .and_then(|id| {
            sections
                .iter()
                .find(|section| text(section, "id") == Some(id))
        })
        .and_then(|section| text(section, "label"))
        .filter(|label| !label.is_empty());
    let mut detail = Map::new();
    detail.insert("id".to_owned(), json!(id));
    detail.insert("label".to_owned(), json!(label));
    detail.insert("type".to_owned(), json!(kind));
    detail.insert("hidden".to_owned(), json!(hidden));
    if let Some(section) = section {
        detail.insert("section".to_owned(), json!(section));
    }
    if let Some(purpose) = text(field, "purpose").and_then(purpose) {
        detail.insert("purpose".to_owned(), json!(purpose));
    }
    let value = text(field, "value").unwrap_or("");
    detail.insert("empty".to_owned(), json!(value.is_empty()));
    if !hidden && !value.is_empty() {
        detail.insert("value".to_owned(), json!(value));
    }
    if let Some(strength) = field
        .get("password_details")
        .and_then(|details| text(details, "strength"))
    {
        detail.insert("strength".to_owned(), json!(strength.to_lowercase()));
    }
    Some(Value::Object(detail))
}

/// The account's own id, from what `op account get --format json` says of
/// it, or none. 1Password's own link to an item names it beside the vault
/// and the item (`onepassword://view-item?a=ACCOUNT&v=VAULT&i=ITEM`), and
/// nothing op says of an item does, so it's asked once and laid on each.
pub fn account(said: &Value) -> Option<String> {
    text(said, "id")
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
}

/// An entry with its account's id under `own`, as `account`, for its link.
pub fn in_account(mut entry: Value, account: Option<&str>) -> Value {
    if let Some(account) = account
        && let Some(own) = entry.get_mut("own").and_then(Value::as_object_mut)
    {
        own.insert("account".to_owned(), json!(account));
    }
    entry
}

/// The value of one field of an item, by its id: the one place a secret
/// is read, for the detail Joel asked to copy or show.
pub fn value(item: &Value, wanted: &str) -> Option<String> {
    item.get("fields")?
        .as_array()?
        .iter()
        .find(|field| text(field, "id") == Some(wanted))
        .and_then(|field| text(field, "value"))
        .map(str::to_owned)
}

/// Each login's password from what `op item get - --reveal --format json`
/// says of many items, given the list on its input: op writes one JSON
/// object an item, one after another, or an array of them. What's built
/// is only what an audit needs, each item's id and its password, the one
/// whose purpose is a password, else the field so named; an item without
/// one is left out. Every other part of what op said goes as this returns.
pub fn passwords(said: &str) -> Vec<(String, String)> {
    let mut items = Vec::new();
    for value in serde_json::Deserializer::from_str(said)
        .into_iter::<Value>()
        .map_while(Result::ok)
    {
        match value {
            Value::Array(many) => items.extend(many),
            one => items.push(one),
        }
    }
    items
        .iter()
        .filter_map(|item| {
            let id = text(item, "id").filter(|id| !id.is_empty())?;
            let fields = item.get("fields")?.as_array()?;
            let password = fields
                .iter()
                .find(|field| text(field, "purpose") == Some("PASSWORD"))
                .or_else(|| {
                    fields
                        .iter()
                        .find(|field| text(field, "id") == Some("password"))
                })
                .and_then(|field| text(field, "value"))
                .filter(|value| !value.is_empty())?;
            Some((id.to_owned(), password.to_owned()))
        })
        .collect()
}

/// The period of an item's one-time code in seconds, from its otpauth
/// address's `period`, else the 30 every authenticator assumes.
pub fn period(item: &Value) -> u64 {
    item.get("fields")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|field| text(field, "type") == Some("OTP"))
        .and_then(|field| text(field, "value"))
        .and_then(|address| address.split_once('?'))
        .and_then(|(_, query)| {
            query
                .split('&')
                .find_map(|pair| pair.strip_prefix("period="))
                .and_then(|period| period.parse().ok())
        })
        .filter(|period: &u64| *period > 0)
        .unwrap_or(30)
}

/// Whether 1Password's app lets its command ask it, by the app's own
/// settings file: Settings › Developer › Integrate with 1Password CLI,
/// which it writes as `developers.cliSharedLockState.enabled` once it's
/// turned on and leaves out while it's off, as it is on a new machine.
/// While it's off op can't ask the app to unlock, so it says it isn't
/// signed in at once, and no prompt ever comes. `None` when the file
/// isn't read as 1Password writes it, so nothing is said of a setting
/// not seen.
pub fn lets_its_command_in(settings: &str) -> Option<bool> {
    let said: Value = serde_json::from_str(settings).ok()?;
    let held = said.as_object()?;
    Some(
        held.get("developers.cliSharedLockState.enabled")
            .and_then(Value::as_bool)
            == Some(true),
    )
}

/// What Joel reads while 1Password hasn't let the launcher in.
pub const LET_IN: &str = "1Password asks you to let the launcher in";

/// What Joel reads while op hasn't been asked since the manager started
/// or the app opened, so nothing is asking him yet: the manager never runs
/// op on its own, and whether 1Password lets the launcher in is known only
/// once he asks (2026-11-10, a restart read as 1Password asking him).
pub const UNASKED: &str = "1Password is asked as you next open Passwords";

/// What Joel reads while 1Password keeps its command out, by the setting's
/// own name and where the app has it, so he can find it there.
pub const KEPT_OUT: &str =
    "1Password needs Integrate with 1Password CLI on, in Settings \u{203a} Developer";

/// What Joel reads while 1Password's app isn't running, so op has nothing
/// to ask him through, and what Unlock does about it.
pub const CLOSED: &str = "1Password is closed, and Unlock opens it";

/// What Joel reads while the app is being opened for his ask.
pub const OPENING: &str = "1Password is opening, to let the launcher in";

/// What op last said of its leave since the manager started or the app
/// last opened: nothing yet, since nothing asked it; that 1Password keeps
/// it out until Joel lets it in; or that it answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Leave {
    Unasked,
    Out,
    In,
}

impl Leave {
    /// As an atomic holds it.
    #[must_use]
    pub const fn byte(self) -> u8 {
        match self {
            Self::Unasked => 0,
            Self::Out => 1,
            Self::In => 2,
        }
    }

    /// From what an atomic holds; anything else is nothing asked yet.
    #[must_use]
    pub const fn of(byte: u8) -> Self {
        match byte {
            1 => Self::Out,
            2 => Self::In,
            _ => Self::Unasked,
        }
    }
}

/// How the manager stands, from what it knows of 1Password: whether the
/// app keeps its command out, whether the app is open (`None` where
/// nothing says), and what op said of its leave since.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stands {
    KeptOut,
    Closed,
    Unasked,
    LetIn,
    Answering,
}

impl Stands {
    /// The setting comes first, since nothing else helps while it's off;
    /// then the app, since op can't prompt without it; then op's leave.
    #[must_use]
    pub fn of(kept_out: bool, open: Option<bool>, leave: Leave) -> Self {
        if kept_out {
            Self::KeptOut
        } else if open == Some(false) {
            Self::Closed
        } else {
            match leave {
                Leave::Unasked => Self::Unasked,
                Leave::Out => Self::LetIn,
                Leave::In => Self::Answering,
            }
        }
    }

    /// As Passwords' line protocol says a standing.
    pub fn said(self) -> Value {
        match self {
            Self::KeptOut => json!({"state": "refused", "reason": KEPT_OUT}),
            Self::Closed => json!({"state": "signed-out", "reason": CLOSED}),
            Self::Unasked => json!({"state": "signed-out", "reason": UNASKED}),
            Self::LetIn => json!({"state": "signed-out", "reason": LET_IN}),
            Self::Answering => json!({"state": "answering"}),
        }
    }
}

/// Where 1Password keeps its settings: under `XDG_CONFIG_HOME`, else
/// the home's `.config`, as the app finds its own folder.
pub fn settings_file(config_home: Option<&str>, home: Option<&str>) -> Option<String> {
    let config = match (config_home, home) {
        (Some(config), _) if !config.is_empty() => config.to_owned(),
        (_, Some(home)) if !home.is_empty() => format!("{home}/.config"),
        _ => return None,
    };
    Some(format!("{config}/1Password/settings/settings.json"))
}

/// The reason op gives for not answering, said for Joel: what op printed
/// last, less its "[ERROR] date time" prefix.
pub fn reason(said: &str) -> String {
    let last = said
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("");
    let plain = last.strip_prefix("[ERROR] ").unwrap_or(last);
    // "2026/11/07 20:21:35 account is not signed in"
    let mut words = plain.splitn(3, ' ');
    match (words.next(), words.next(), words.next()) {
        (Some(date), Some(time), Some(rest)) if date.contains('/') && time.contains(':') => {
            rest.to_owned()
        }
        _ => plain.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_setting_stands_before_the_app_and_the_app_before_the_leave() {
        assert_eq!(Stands::of(true, Some(false), Leave::In), Stands::KeptOut);
        assert_eq!(Stands::of(false, Some(false), Leave::In), Stands::Closed);
        assert_eq!(Stands::of(false, Some(true), Leave::In), Stands::Answering);
        assert_eq!(Stands::of(false, None, Leave::Out), Stands::LetIn);
        assert_eq!(Stands::of(false, None, Leave::Unasked), Stands::Unasked);
        for leave in [Leave::Unasked, Leave::Out, Leave::In] {
            assert_eq!(
                Leave::of(leave.byte()),
                leave,
                "{leave:?} is held as itself"
            );
        }
        assert_eq!(
            Stands::Closed.said(),
            json!({"state": "signed-out", "reason": CLOSED})
        );
    }

    /// A login as op item get gives it, in op 2's JSON.
    fn login() -> Value {
        json!({
            "id": "abc123", "title": "GitHub", "version": 4,
            "vault": {"id": "v1", "name": "Private"},
            "category": "LOGIN", "last_edited_by": "U1",
            "created_at": "2024-02-29T12:00:00Z", "updated_at": "2026-11-07T20:21:35.5Z",
            "additional_information": "ada", "favorite": true, "tags": ["work"],
            "urls": [{"label": "website", "primary": true, "href": "https://github.com/login"}],
            "sections": [{"id": "add more", "label": "Security"}],
            "fields": [
                {"id": "username", "type": "STRING", "purpose": "USERNAME",
                 "label": "username", "value": "ada", "reference": "op://Private/GitHub/username"},
                {"id": "password", "type": "CONCEALED", "purpose": "PASSWORD",
                 "label": "password", "value": "hunter2",
                 "password_details": {"strength": "FANTASTIC", "history": ["old-one"]}},
                {"id": "TOTP_1", "section": {"id": "add more"}, "type": "OTP",
                 "label": "one-time password", "value": "otpauth://totp/GitHub?secret=JBSW&period=60",
                 "totp": "123456"},
                {"id": "key", "type": "SSHKEY", "label": "private key", "value": "-----BEGIN"},
                {"id": "notesPlain", "type": "STRING", "purpose": "NOTES", "label": "notesPlain",
                 "value": ""},
            ],
            "files": [{"id": "f1", "name": "codes.txt"}],
        })
    }

    #[test]
    fn an_entry_carries_no_secret_op_gives() {
        let said = entry(&login()).unwrap().to_string();
        for secret in ["hunter2", "old-one", "JBSW", "123456", "BEGIN"] {
            assert!(!said.contains(secret), "{secret} went through: {said}");
        }
    }

    #[test]
    fn an_entry_says_what_1password_shows_of_an_item() {
        let entry = entry(&login()).unwrap();
        assert_eq!(entry["title"], "GitHub");
        assert_eq!(entry["category"], "login");
        assert_eq!(entry["collection"]["name"], "Private");
        assert_eq!(entry["favourite"], true);
        assert_eq!(entry["username"], "ada");
        assert_eq!(entry["websites"][0]["address"], "https://github.com/login");
        assert_eq!(entry["created"], 1_709_208_000_000_u64);
        assert_eq!(entry["changed"], 1_794_082_895_500_u64);
        assert_eq!(entry["own"]["files"], 1);
        let details = entry["details"].as_array().unwrap();
        assert_eq!(details[0]["value"], "ada");
        assert_eq!(details[1]["type"], "hidden");
        assert_eq!(details[1]["purpose"], "password");
        assert_eq!(details[1]["strength"], "fantastic");
        assert_eq!(details[2]["type"], "code");
        assert_eq!(details[2]["section"], "Security");
        assert_eq!(details[3]["hidden"], true);
        assert_eq!(details[4]["empty"], true);
    }

    #[test]
    fn a_listed_login_takes_its_username_from_its_summary() {
        let listed = json!({"id": "x", "title": "Bank", "category": "LOGIN",
                            "additional_information": "ada@example.com"});
        let entry = entry(&listed).unwrap();
        assert_eq!(entry["username"], "ada@example.com");
        assert!(
            entry.get("details").is_none(),
            "a list gave details: {entry}"
        );
        let note = json!({"id": "n", "category": "SECURE_NOTE", "additional_information": "Hi"});
        assert!(super::entry(&note).unwrap().get("username").is_none());
    }

    #[test]
    fn an_entry_names_its_account_for_its_link() {
        // As op account get gives it, in op 2's JSON.
        let said = json!({"id": "ACCT1", "name": "Family", "domain": "my.1password.com",
                          "type": "FAMILY", "state": "ACTIVE"});
        assert_eq!(account(&said).as_deref(), Some("ACCT1"));
        assert_eq!(account(&json!({"id": ""})), None);
        let named = in_account(entry(&login()).unwrap(), Some("ACCT1"));
        assert_eq!(named["own"]["account"], "ACCT1");
        assert_eq!(named["collection"]["id"], "v1");
        let unnamed = in_account(entry(&login()).unwrap(), None);
        assert!(unnamed["own"].get("account").is_none(), "{unnamed}");
    }

    #[test]
    fn a_secret_is_read_only_for_the_detail_asked() {
        assert_eq!(value(&login(), "password").as_deref(), Some("hunter2"));
        assert_eq!(value(&login(), "nothing"), None);
        assert_eq!(period(&login()), 60);
        assert_eq!(period(&json!({"fields": []})), 30);
    }

    #[test]
    fn an_audit_reads_each_items_password_alone() {
        // As op item get - --reveal writes many items: one after another.
        let mut note = login();
        note["id"] = json!("note");
        note["fields"] = json!([{"id": "notesPlain", "type": "STRING", "value": "hi"}]);
        let mut plain = login();
        plain["id"] = json!("pw");
        plain["fields"] = json!([{"id": "password", "type": "CONCEALED", "value": "abc"}]);
        let said = format!("{:#}\n{}\n{}", login(), note, plain);
        assert_eq!(
            passwords(&said),
            [
                ("abc123".to_owned(), "hunter2".to_owned()),
                ("pw".to_owned(), "abc".to_owned())
            ]
        );
        let array = format!("[{}]", login());
        assert_eq!(passwords(&array).len(), 1);
        assert_eq!(passwords(""), Vec::<(String, String)>::new());
    }

    #[test]
    fn categories_and_kinds_are_passwords_own_words() {
        assert_eq!(category("SECURE_NOTE"), "note");
        assert_eq!(category("DRIVER_LICENSE"), "driver's licence");
        assert_eq!(category("BANK_ACCOUNT"), "bank account");
        assert_eq!(kind("CREDIT_CARD_NUMBER"), "hidden");
        assert_eq!(kind("MENU"), "text");
    }

    #[test]
    fn ops_reason_is_said_without_its_stamp() {
        assert_eq!(
            reason("[ERROR] 2026/11/07 20:21:35 account is not signed in\n"),
            "account is not signed in"
        );
        assert_eq!(reason("plain words"), "plain words");
    }

    #[test]
    fn the_command_is_let_in_only_once_its_setting_is_on() {
        let on = r#"{"version": 1, "developers.cliSharedLockState.enabled": true}"#;
        assert_eq!(
            lets_its_command_in(on),
            Some(true),
            "turned on, it's let in"
        );
        // As a new machine's app writes it, with the setting never turned on.
        let new = r#"{"version": 1, "security.authenticatedUnlock.enabled": true}"#;
        assert_eq!(lets_its_command_in(new), Some(false), "left out, it's off");
        let off = r#"{"developers.cliSharedLockState.enabled": false}"#;
        assert_eq!(
            lets_its_command_in(off),
            Some(false),
            "turned off, it's off"
        );
        assert_eq!(
            lets_its_command_in("not json"),
            None,
            "unread, nothing is said"
        );
        assert_eq!(
            lets_its_command_in("[]"),
            None,
            "not its shape, nothing is said"
        );
    }

    #[test]
    fn its_settings_are_found_where_the_app_keeps_them() {
        let file = "1Password/settings/settings.json";
        assert_eq!(
            settings_file(Some("/c"), Some("/h")),
            Some(format!("/c/{file}")),
            "XDG_CONFIG_HOME leads"
        );
        assert_eq!(
            settings_file(Some(""), Some("/h")),
            Some(format!("/h/.config/{file}")),
            "else the home's .config"
        );
        assert_eq!(settings_file(None, None), None, "no home, no file");
    }

    #[test]
    fn ops_times_are_read_as_the_epoch_counts_them() {
        assert_eq!(millis("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(millis("2000-03-01T00:00:01+00:00"), Some(951_868_801_000));
        assert_eq!(millis("not a time"), None);
    }
}
