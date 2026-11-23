//! Reservations as eero answers them in the contract, and each act's walk
//! against an eero that answers as each test says, so what is sent, what
//! is refused before anything is, and how a change's outcome is judged
//! are each seen.

use std::collections::{HashMap, VecDeque};

use super::*;

const NETWORK: &str = "/2.2/networks/1001";
const KEPT_URL: &str = "/2.2/networks/1001/reservations/2c8db13091a2";

fn mac(said: &str) -> Mac {
    Mac::parse(said).expect("a MAC the test means to be good")
}

fn address(said: &str) -> Ipv4Addr {
    said.parse().expect("an address the test means to be good")
}

/// The data of the contract's answer to `networks/1001/<name>`.
fn contract(name: &str) -> Value {
    let path = format!("{}/contract/net.{name}.json", env!("CARGO_MANIFEST_DIR"));
    let body: Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("the contract's file"))
            .expect("the contract's JSON");
    body["data"].clone()
}

fn reservations() -> Vec<Kept> {
    kept(&contract("reservations")).expect("the contract's reservations read")
}

/// An eero that answers each method and path from a queue the test fills,
/// each answer once, and keeps what it was asked, in order: a read or a
/// change the test didn't foresee fails it, rather than being answered as
/// the last one was.
#[derive(Default)]
struct Eero {
    answers: HashMap<String, VecDeque<Result<Value, Failure>>>,
    asked: Vec<String>,
}

impl Eero {
    /// The contract's network, whose reads answer as the contract does.
    fn contract() -> Eero {
        let mut eero = Eero::default();
        for name in ["subnets_config", "devices", "eeros", "forwards"] {
            eero.answers(&format!("GET {NETWORK}/{name}"), Ok(contract(name)));
        }
        eero
    }

    fn answers(&mut self, asked: &str, answer: Result<Value, Failure>) -> &mut Eero {
        self.answers
            .entry(asked.to_owned())
            .or_default()
            .push_back(answer);
        self
    }

    fn answer(&mut self, asked: &str) -> Result<Value, Failure> {
        let answer = self.answers.get_mut(asked).and_then(VecDeque::pop_front);
        self.asked.push(asked.to_owned());
        answer.unwrap_or_else(|| panic!("the test gave eero no answer to {asked}"))
    }

    /// What was asked other than reads.
    fn changes(&self) -> Vec<&str> {
        self.asked
            .iter()
            .filter(|asked| !asked.starts_with("GET "))
            .map(String::as_str)
            .collect()
    }
}

impl Asks for Eero {
    fn get(&mut self, path: &str) -> Result<Value, Failure> {
        self.answer(&format!("GET {path}"))
    }

    fn post(&mut self, path: &str, body: Option<&Value>) -> Result<Value, Failure> {
        self.answer(&format!("POST {path} {}", body.unwrap_or(&Value::Null)))
    }

    fn put(&mut self, path: &str, body: &Value) -> Result<Value, Failure> {
        self.answer(&format!("PUT {path} {body}"))
    }

    fn delete(&mut self, path: &str) -> Result<Value, Failure> {
        self.answer(&format!("DELETE {path}"))
    }
}

/// The contract's reservations, then `after` once a change is sent.
fn kept_then(eero: &mut Eero, after: Result<Value, Failure>) {
    let reservations = format!("GET {NETWORK}/reservations");
    eero.answers(&reservations, Ok(contract("reservations")))
        .answers(&reservations, after);
}

fn with_new(mac: &str, ip: &str) -> Value {
    let mut kept = contract("reservations");
    kept.as_array_mut()
        .expect("the contract's reservations are a list")
        .push(json!({"url": format!("{NETWORK}/reservations/9f00aa"), "mac": mac, "ip": ip}));
    kept
}

fn posted(mac: &str, ip: &str) -> String {
    format!(
        "POST {NETWORK}/reservations {}",
        json!({"mac": mac, "ip": ip, "description": mac})
    )
}

#[test]
fn the_contracts_reservation_is_kept() {
    assert_eq!(
        reservations(),
        vec![Kept {
            url: KEPT_URL.into(),
            mac: mac("02:00:00:00:00:38"),
            address: address("10.0.0.208"),
            device: Some("/2.2/networks/1001/devices/020000000038".into()),
        }],
        "eero's reservation didn't read as it answered it"
    );
}

#[test]
fn a_reservation_list_this_cant_read_whole_is_refused() {
    let good = json!({"url": KEPT_URL, "mac": "02:00:00:00:00:38", "ip": "10.0.0.208"});
    for (odd, list) in [
        ("not a list", json!({"reservations": [good.clone()]})),
        (
            "no url",
            json!([good.clone(), {"mac": "02:00:00:00:00:01", "ip": "10.0.0.31"}]),
        ),
        (
            "an odd MAC",
            json!([{"url": KEPT_URL, "mac": "020000000001", "ip": "10.0.0.31"}]),
        ),
        (
            "an odd address",
            json!([{"url": KEPT_URL, "mac": "02:00:00:00:00:01", "ip": "10.0.0"}]),
        ),
    ] {
        assert!(
            kept(&list).is_err(),
            "a list with {odd} read as one this can trust"
        );
    }
}

#[test]
fn a_reservation_is_added_only_where_it_crosses_none() {
    let kept = reservations();
    let held = mac("02:00:00:00:00:38");
    let other = mac("02:00:00:00:00:01");
    assert_eq!(
        [
            to_reserve(&kept, held, address("10.0.0.208")),
            to_reserve(&kept, other, address("10.0.0.77")),
            to_reserve(&kept, held, address("10.0.0.77")),
            to_reserve(&kept, other, address("10.0.0.208")),
        ],
        [
            Ok(Change::Nothing),
            Ok(Change::Add),
            Err("02:00:00:00:00:38 already keeps 10.0.0.208; unreserve it first".into()),
            Err("10.0.0.208 is already kept for 02:00:00:00:00:38; unreserve that first".into()),
        ],
        "a reservation replaced one, or wasn't added where it crossed none"
    );
}

#[test]
fn an_unreservation_takes_only_the_devices_one_reservation() {
    let kept = reservations();
    let mut twice = kept.clone();
    twice.push(Kept {
        url: format!("{NETWORK}/reservations/9f00aa"),
        ..kept[0].clone()
    });
    assert_eq!(
        [
            to_unreserve(&kept, mac("02:00:00:00:00:38")),
            to_unreserve(&kept, mac("02:00:00:00:00:01")),
        ],
        [Ok(Some(&kept[0])), Ok(None)],
        "an unreservation took the wrong reservation, or one that wasn't there"
    );
    assert!(
        to_unreserve(&twice, mac("02:00:00:00:00:38")).is_err(),
        "a device eero keeps two addresses for had one taken at a guess"
    );
}

#[test]
fn only_an_address_the_home_subnet_gives_a_device_is_kept() {
    let subnets = contract("subnets_config");
    for (ok, said) in [
        (true, "10.0.0.77"),
        (true, "10.0.0.20"),
        (true, "10.0.0.254"),
        (false, "10.0.0.19"),
        (false, "10.0.0.2"),
        (false, "10.0.0.255"),
        (false, "10.0.3.254"),
        (false, "10.0.4.1"),
        (false, "192.168.4.20"),
        (false, "10.0.0.0"),
        (false, "10.0.3.255"),
    ] {
        assert_eq!(
            within(&subnets, address(said)).is_ok(),
            ok,
            "{said} was judged wrongly against the contract's 10.0.0.0/22, given out from \
             10.0.0.20 to 10.0.0.254"
        );
    }
    let mut no_range = subnets.clone();
    no_range[0]["dedicated_subnet"]
        .as_object_mut()
        .expect("the contract's home subnet")
        .remove("end_ip");
    assert!(
        within(&no_range, address("10.0.0.77")).is_err(),
        "an address was kept with no range to check it against"
    );
    assert!(
        within(&json!([]), address("10.0.0.77")).is_err(),
        "an address was kept with no home subnet to check it against"
    );
}

#[test]
fn an_address_a_device_or_an_eero_holds_is_another_devices() {
    let devices = contract("devices");
    let eeros = contract("eeros");
    let ours = mac("02:00:00:00:00:01");
    assert_eq!(
        [
            holder(&devices, &eeros, ours, address("10.0.0.31")),
            holder(&devices, &eeros, ours, address("10.0.0.1")),
            holder(&devices, &eeros, ours, address("10.0.0.29")),
            holder(&devices, &eeros, ours, address("10.0.0.77")),
        ],
        [
            Ok(Some("held by printer 1".to_owned())),
            Ok(Some("an eero's own".to_owned())),
            Ok(None),
            Ok(None),
        ],
        "a held address was missed, or the device's own counted as another's"
    );
    // A device holding the address only among its others, or one whose
    // MAC eero lists oddly, is still another's: only our own MAC is ours.
    let odd = json!([
        {"mac": "02:00:00:00:00:44", "ip": null, "ips": ["fe80::4", "10.0.0.77"],
            "nickname": "second address"},
        {"mac": "02-00-00-00-00-45", "ip": "10.0.0.78", "nickname": "odd MAC"},
        {"mac": null, "ip": "10.0.0.79", "nickname": "no MAC"},
    ]);
    assert_eq!(
        ["10.0.0.77", "10.0.0.78", "10.0.0.79"].map(|held| holder(
            &odd,
            &eeros,
            ours,
            address(held)
        )),
        [
            Ok(Some("held by second address".to_owned())),
            Ok(Some("held by odd MAC".to_owned())),
            Ok(Some("held by no MAC".to_owned())),
        ],
        "an address held among a device's others, or by one eero lists oddly, was free"
    );
    let wrapped = json!({"data": []});
    assert_eq!(
        [
            holder(&wrapped, &eeros, ours, address("10.0.0.77")),
            holder(&devices, &wrapped, ours, address("10.0.0.77")),
        ],
        [
            Err("eero lists its devices in a way this can't read".to_owned()),
            Err("eero lists its eeros in a way this can't read".to_owned()),
        ],
        "a list of devices or eeros this can't read was taken as no one holding the address"
    );
}

#[test]
fn a_port_forward_naming_the_reservation_any_way_is_found_wherever_it_says_it() {
    let held = &reservations()[0];
    let forward = |target: Value| {
        json!([{"url": "/2.2/networks/1001/forwards/1", "target": {"to": target},
            "ports": [{"from": 25565}]}])
    };
    for target in [
        json!("10.0.0.208"),
        json!("10.0.0.208:25565"),
        json!("10.0.0.208/32"),
        json!("10.0.0.208-10.0.0.210"),
        json!("ip=10.0.0.208"),
        json!("tcp 10.0.0.208:22"),
        json!(" 10.0.0.208."),
        json!(KEPT_URL),
        json!(KEPT_URL.to_ascii_uppercase()),
        json!(format!("https://api-user.e2ro.com{KEPT_URL}/")),
        json!("2c8db13091a2"),
        json!("/2.2/networks/1001/devices/020000000038"),
        json!("02:00:00:00:00:38"),
        json!("02:00:00:00:00:38".to_ascii_uppercase()),
        json!("02-00-00-00-00-38"),
        json!("0200.0000.0038"),
        json!("/devices/02:00:00:00:00:38"),
        json!("020000000038"),
        json!(u32::from(held.address)),
        json!(u32::from(held.address).to_string()),
    ] {
        assert_eq!(
            forwarded(&forward(target.clone()), held),
            Ok(true),
            "a forward naming the reservation by {target} was missed"
        );
    }
    for target in [
        json!("10.0.0.20"),
        json!("10.0.0.2080"),
        json!("110.0.0.208"),
        json!("/2.2/networks/1001/devices/020000000001"),
        json!("/2.2/networks/1001/reservations/2c8db13091a3"),
        json!("02:00:00:00:00:3"),
        json!(25565),
    ] {
        assert_eq!(
            forwarded(&forward(target.clone()), held),
            Ok(false),
            "a forward naming something else, {target}, was taken for the reservation's"
        );
    }
    // eero names the device by its own id, which needn't be the MAC: the
    // url it lists is what's searched for, not one made from the MAC.
    let named_apart = Kept {
        device: Some("/2.2/networks/1001/devices/d7a91c".into()),
        ..held.clone()
    };
    assert_eq!(
        forwarded(
            &forward(json!("/2.2/networks/1001/devices/d7a91c")),
            &named_apart
        ),
        Ok(true),
        "a forward naming the device by the url eero lists for it was missed"
    );
    assert_eq!(
        forwarded(&contract("forwards"), held),
        Ok(false),
        "the contract's empty forwards were found to reach the reservation"
    );
    assert!(
        forwarded(&json!({"data": []}), held).is_err(),
        "forwards this can't read were taken as none"
    );
}

#[test]
fn only_a_reservation_of_the_network_may_be_deleted() {
    for (ok, url) in [
        (true, KEPT_URL),
        (false, "/2.2/networks/1002/reservations/2c8db13091a2"),
        (false, "/2.2/networks/1001/reservations/"),
        (false, "/2.2/networks/1001/reservations/../devices"),
        (false, "/2.2/networks/1001"),
        (
            false,
            "@elsewhere.example/2.2/networks/1001/reservations/2c8db13091a2",
        ),
    ] {
        assert_eq!(of_network(NETWORK, url), ok, "{url} was judged wrongly");
    }
}

#[test]
fn a_free_address_is_posted_and_read_back() {
    let mut eero = Eero::contract();
    kept_then(&mut eero, Ok(with_new("02:00:00:00:00:01", "10.0.0.77")));
    eero.answers(&posted("02:00:00:00:00:01", "10.0.0.77"), Ok(json!({})));
    assert_eq!(
        reserve(
            &mut eero,
            NETWORK,
            mac("02:00:00:00:00:01"),
            address("10.0.0.77")
        ),
        Ok(Done::Made),
        "a free address eero then kept wasn't said made"
    );
    let read = |name: &str| format!("GET {NETWORK}/{name}");
    assert_eq!(
        eero.asked,
        [
            read("reservations"),
            read("subnets_config"),
            read("devices"),
            read("eeros"),
            posted("02:00:00:00:00:01", "10.0.0.77"),
            read("reservations"),
        ],
        "the reservation wasn't checked, asked once, then read back, in that order"
    );
}

#[test]
fn nothing_is_posted_where_the_reservation_stands_or_crosses_a_device() {
    let mut stands = Eero::contract();
    kept_then(&mut stands, Ok(contract("reservations")));
    let held = mac("02:00:00:00:00:38");
    assert_eq!(
        reserve(&mut stands, NETWORK, held, address("10.0.0.208")),
        Ok(Done::Unchanged),
        "a reservation that stood wasn't said unchanged"
    );
    let mut taken = Eero::contract();
    kept_then(&mut taken, Ok(contract("reservations")));
    let refused = reserve(
        &mut taken,
        NETWORK,
        mac("02:00:00:00:00:01"),
        address("10.0.0.31"),
    );
    assert!(
        matches!(&refused, Err(Failure::Refused(why)) if why.contains("printer 1")),
        "an address the printer holds wasn't refused for it: {refused:?}"
    );
    let mut outside = Eero::contract();
    kept_then(&mut outside, Ok(contract("reservations")));
    let refused = reserve(
        &mut outside,
        NETWORK,
        mac("02:00:00:00:00:01"),
        address("10.0.8.1"),
    );
    assert!(
        matches!(&refused, Err(Failure::Refused(why)) if why.contains("home subnet")),
        "an address outside the home subnet wasn't refused for it: {refused:?}"
    );
    let mut below = Eero::contract();
    kept_then(&mut below, Ok(contract("reservations")));
    let refused = reserve(
        &mut below,
        NETWORK,
        mac("02:00:00:00:00:01"),
        address("10.0.0.5"),
    );
    assert!(
        matches!(&refused, Err(Failure::Refused(why)) if why.contains("range eero gives out")),
        "an address below the range eero gives out wasn't refused for it: {refused:?}"
    );
    for eero in [stands, taken, outside, below] {
        assert_eq!(eero.changes(), Vec::<&str>::new(), "eero was sent a change");
    }
}

#[test]
fn a_reservations_outcome_is_what_eero_keeps_after_it() {
    let ours = "02:00:00:00:00:01";
    let walk = |sent: Result<Value, Failure>, after: Result<Value, Failure>| {
        let mut eero = Eero::contract();
        kept_then(&mut eero, after);
        eero.answers(&posted(ours, "10.0.0.77"), sent);
        reserve(&mut eero, NETWORK, mac(ours), address("10.0.0.77"))
    };
    let refused = Failure::Refused("eero answered 409 to the reservations".into());
    let lost = Failure::Unreachable("couldn't reach eero: timed out".into());
    let signed_out = Failure::SignedOut("aged".into());
    assert_eq!(
        walk(Err(refused.clone()), Ok(with_new(ours, "10.0.0.77"))),
        Ok(Done::Made),
        "a reservation eero keeps read as refused for its answer"
    );
    assert_eq!(
        walk(Err(refused.clone()), Ok(contract("reservations"))),
        Err(refused),
        "a refusal eero keeps nothing of wasn't said as eero said it"
    );
    let unknown = |came: Result<Done, Failure>| {
        matches!(&came, Err(Failure::Unknown(why))
            if why.contains("read eero internet's reservations to see"))
    };
    for (sent, after, what) in [
        (
            Ok(json!({})),
            Ok(contract("reservations")),
            "eero answered, but whose list doesn't show it yet",
        ),
        (
            Ok(Value::Null),
            Ok(contract("reservations")),
            "eero took with no data, but whose list doesn't show it yet",
        ),
        (
            Err(lost.clone()),
            Ok(contract("reservations")),
            "a send timed out, which may still land, and the list doesn't show yet",
        ),
        (
            Ok(json!({})),
            Err(lost.clone()),
            "eero answered, then couldn't be read",
        ),
        (
            Ok(json!({})),
            Ok(json!({"oops": 1})),
            "eero answered, then listed in a way this can't read",
        ),
        (
            Err(lost.clone()),
            Err(lost),
            "neither eero's answer nor a read settles",
        ),
    ] {
        assert!(
            unknown(walk(sent, after)),
            "a reservation {what} wasn't said unknown, with how to find out"
        );
    }
    assert_eq!(
        walk(Err(signed_out.clone()), Ok(with_new(ours, "10.0.0.77"))),
        Err(signed_out),
        "a reservation sent under a refused session wasn't said signed out"
    );
}

/// What unreserving the contract's reservation comes to, eero answering
/// its DELETE with `sent` and the read after with `after`, and what it
/// was asked other than reads.
fn unreserved(sent: Result<Value, Failure>, after: Result<Value, Failure>) -> (Came, Vec<String>) {
    let mut eero = Eero::contract();
    kept_then(&mut eero, after);
    eero.answers(&format!("DELETE {KEPT_URL}"), sent);
    let came = unreserve(&mut eero, NETWORK, mac("02:00:00:00:00:38"));
    (
        came,
        eero.changes().into_iter().map(str::to_owned).collect(),
    )
}

#[test]
fn an_unreservations_outcome_is_what_eero_keeps_after_it_and_names_what_it_held() {
    let held = Some(address("10.0.0.208"));
    let lost = Failure::Unreachable("couldn't reach eero: timed out".into());
    let undo = "eero devices 02:00:00:00:00:38 reserve 10.0.0.208";
    for (sent, after, what) in [
        (
            Ok(Value::Null),
            Ok(contract("reservations")),
            "eero answered, but still lists",
        ),
        (
            Err(lost.clone()),
            Err(lost.clone()),
            "neither eero's answer nor a read settles",
        ),
    ] {
        let (came, _) = unreserved(sent, after);
        assert!(
            matches!(&came.done, Err(Failure::Unknown(why)) if why.contains(undo))
                && came.held == held,
            "an unreservation {what} wasn't said unknown with the address it held: {came:?}"
        );
    }
    let signed_out = Failure::SignedOut("aged".into());
    let mut refused = Eero::contract();
    kept_then(&mut refused, Ok(json!([])));
    refused.answers(&format!("DELETE {KEPT_URL}"), Err(signed_out.clone()));
    let came = unreserve(&mut refused, NETWORK, mac("02:00:00:00:00:38"));
    assert_eq!(
        (came.done, refused.asked.last().map(String::as_str)),
        (Err(signed_out), Some(format!("DELETE {KEPT_URL}").as_str())),
        "an unreservation sent under a refused session wasn't said signed out, or was read back"
    );
}

#[test]
fn an_unreservation_deletes_the_devices_own_and_says_what_it_held() {
    let (came, changes) = unreserved(Ok(Value::Null), Ok(json!([])));
    assert_eq!(
        came,
        Came {
            done: Ok(Done::Made),
            held: Some(address("10.0.0.208")),
        },
        "an unreservation didn't say the address it held"
    );
    assert_eq!(
        changes,
        [format!("DELETE {KEPT_URL}")],
        "the wrong thing was deleted"
    );

    let missing = Failure::Refused("eero answered 404".into());
    let (came, _) = unreserved(Err(missing), Ok(json!([])));
    assert_eq!(
        came.done,
        Ok(Done::Made),
        "a reservation already gone when deleted wasn't judged by eero's list"
    );

    let mut none = Eero::contract();
    kept_then(&mut none, Ok(contract("reservations")));
    assert_eq!(
        unreserve(&mut none, NETWORK, mac("02:00:00:00:00:01")),
        Came::of(Ok(Done::Unchanged)),
        "a device keeping no address wasn't said unchanged, with nothing held"
    );
    assert_eq!(none.changes(), Vec::<&str>::new(), "eero was sent a change");
}

#[test]
fn an_unreservation_a_forward_reaches_or_eero_names_oddly_is_refused() {
    let mut forwarded = Eero::default();
    kept_then(&mut forwarded, Ok(contract("reservations")));
    forwarded
        .answers(
            &format!("GET {NETWORK}/subnets_config"),
            Ok(contract("subnets_config")),
        )
        .answers(
            &format!("GET {NETWORK}/forwards"),
            Ok(json!([{"ip": "10.0.0.208", "port": 25565}])),
        );
    assert!(
        matches!(
            unreserve(&mut forwarded, NETWORK, mac("02:00:00:00:00:38")).done,
            Err(Failure::Refused(why)) if why.contains("port forward")
        ),
        "an address a forward reaches was unreserved"
    );
    let mut odd = Eero::contract();
    let elsewhere = json!([{"url": "/2.2/networks/1001", "mac": "02:00:00:00:00:38",
        "ip": "10.0.0.208"}]);
    odd.answers(&format!("GET {NETWORK}/reservations"), Ok(elsewhere));
    assert!(
        matches!(
            unreserve(&mut odd, NETWORK, mac("02:00:00:00:00:38")).done,
            Err(Failure::Refused(why)) if why.contains("oddly")
        ),
        "a reservation whose url isn't the network's wasn't refused for it"
    );
    // Below the range eero gives out, keeping it again would be refused,
    // so the undo this would give couldn't be done.
    let mut unkeepable = Eero::contract();
    let low = json!([{"url": KEPT_URL, "mac": "02:00:00:00:00:38", "ip": "10.0.0.5"}]);
    unkeepable.answers(&format!("GET {NETWORK}/reservations"), Ok(low));
    assert!(
        matches!(
            unreserve(&mut unkeepable, NETWORK, mac("02:00:00:00:00:38")).done,
            Err(Failure::Refused(why))
                if why.contains("outside the range") && why.contains("eero app")
        ),
        "a reservation whose undo couldn't be done here was taken away"
    );
    for eero in [forwarded, odd, unkeepable] {
        assert_eq!(eero.changes(), Vec::<&str>::new(), "eero was sent a change");
    }
}
