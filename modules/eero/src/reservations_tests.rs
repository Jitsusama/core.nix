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
/// and keeps what it was asked, in order.
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
        let answer = self.answers.get_mut(asked).and_then(|queue| {
            if queue.len() > 1 {
                queue.pop_front()
            } else {
                queue.front().cloned()
            }
        });
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
        (true, "10.0.3.254"),
        (false, "10.0.4.1"),
        (false, "192.168.4.20"),
        (false, "10.0.0.0"),
        (false, "10.0.3.255"),
    ] {
        assert_eq!(
            within(&subnets, address(said)).is_ok(),
            ok,
            "{said} was judged wrongly against the contract's 10.0.0.0/22"
        );
    }
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
            Some("held by printer 1".to_owned()),
            Some("an eero's own".to_owned()),
            None,
            None,
        ],
        "a held address was missed, or the device's own counted as another's"
    );
}

#[test]
fn a_port_forward_naming_an_address_is_found_wherever_it_says_it() {
    let forwards = json!([{"url": "/2.2/networks/1001/forwards/1",
        "reservation": {"ip": "10.0.0.208"}, "ports": [{"from": 25565}]}]);
    assert_eq!(
        [
            forwarded(&forwards, address("10.0.0.208")),
            forwarded(&forwards, address("10.0.0.20")),
            forwarded(&contract("forwards"), address("10.0.0.208")),
        ],
        [true, false, false],
        "a forward was missed, or one found that names another address"
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
    assert_eq!(
        eero.changes(),
        [posted("02:00:00:00:00:01", "10.0.0.77")],
        "the reservation wasn't asked of eero once, as it should be"
    );
    assert_eq!(
        eero.asked.last().map(String::as_str),
        Some(format!("GET {NETWORK}/reservations").as_str()),
        "the reservation wasn't read back after it was asked"
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
    assert!(
        reserve(
            &mut outside,
            NETWORK,
            mac("02:00:00:00:00:01"),
            address("10.0.8.1")
        )
        .is_err(),
        "an address outside the home subnet wasn't refused"
    );
    for eero in [stands, taken, outside] {
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
    let no_data = Failure::Refused("eero answered 201 without data".into());
    let lost = Failure::Unreachable("couldn't reach eero: timed out".into());
    let signed_out = Failure::SignedOut("aged".into());
    assert_eq!(
        walk(Err(no_data.clone()), Ok(with_new(ours, "10.0.0.77"))),
        Ok(Done::Made),
        "a reservation eero keeps read as refused for its answer's shape"
    );
    assert_eq!(
        walk(Err(no_data.clone()), Ok(contract("reservations"))),
        Err(no_data),
        "a refusal eero keeps nothing of wasn't said as eero said it"
    );
    assert!(
        matches!(
            walk(Ok(json!({})), Ok(contract("reservations"))),
            Err(Failure::Refused(_))
        ),
        "a reservation eero answered but doesn't keep wasn't refused"
    );
    assert!(
        matches!(walk(Err(lost.clone()), Err(lost)), Err(Failure::Unknown(_))),
        "a reservation neither eero's answer nor a read settles wasn't said unknown"
    );
    assert_eq!(
        walk(Err(signed_out.clone()), Ok(with_new(ours, "10.0.0.77"))),
        Err(signed_out),
        "a reservation sent under a refused session wasn't said signed out"
    );
}

#[test]
fn an_unreservation_deletes_the_devices_own_and_says_what_it_freed() {
    let mut eero = Eero::contract();
    kept_then(&mut eero, Ok(json!([])));
    eero.answers(&format!("DELETE {KEPT_URL}"), Ok(Value::Null));
    assert_eq!(
        unreserve(&mut eero, NETWORK, mac("02:00:00:00:00:38")),
        Ok(Done::Freed(address("10.0.0.208"))),
        "an unreservation didn't say the address it freed"
    );
    assert_eq!(
        eero.changes(),
        [format!("DELETE {KEPT_URL}")],
        "the wrong thing was deleted"
    );

    let mut gone = Eero::contract();
    kept_then(&mut gone, Ok(json!([])));
    let missing = Failure::Refused("eero answered 404".into());
    gone.answers(&format!("DELETE {KEPT_URL}"), Err(missing));
    assert_eq!(
        unreserve(&mut gone, NETWORK, mac("02:00:00:00:00:38")),
        Ok(Done::Freed(address("10.0.0.208"))),
        "a reservation already gone when deleted wasn't judged by eero's list"
    );

    let mut none = Eero::contract();
    kept_then(&mut none, Ok(contract("reservations")));
    assert_eq!(
        unreserve(&mut none, NETWORK, mac("02:00:00:00:00:01")),
        Ok(Done::Unchanged),
        "a device keeping no address wasn't said unchanged"
    );
    assert_eq!(none.changes(), Vec::<&str>::new(), "eero was sent a change");
}

#[test]
fn an_unreservation_a_forward_reaches_or_eero_names_oddly_is_refused() {
    let mut forwarded = Eero::default();
    kept_then(&mut forwarded, Ok(contract("reservations")));
    forwarded.answers(
        &format!("GET {NETWORK}/forwards"),
        Ok(json!([{"ip": "10.0.0.208", "port": 25565}])),
    );
    assert!(
        matches!(
            unreserve(&mut forwarded, NETWORK, mac("02:00:00:00:00:38")),
            Err(Failure::Refused(why)) if why.contains("port forward")
        ),
        "an address a forward reaches was unreserved"
    );
    let mut odd = Eero::contract();
    let elsewhere = json!([{"url": "/2.2/networks/1001", "mac": "02:00:00:00:00:38",
        "ip": "10.0.0.208"}]);
    odd.answers(&format!("GET {NETWORK}/reservations"), Ok(elsewhere));
    assert!(
        unreserve(&mut odd, NETWORK, mac("02:00:00:00:00:38")).is_err(),
        "a reservation whose url isn't the network's was deleted"
    );
    for eero in [forwarded, odd] {
        assert_eq!(eero.changes(), Vec::<&str>::new(), "eero was sent a change");
    }
}
