//! Power's rows, ready for the launcher, in Y2YBTS's ready-row form
//! (QEST-20261023-Y2YBTS attachments/speed/ready-rows.md): the Charger,
//! holding each USB-C port and what it gives; the battery's Charge as a
//! bar; what it's Drawing or Charging at and its Voltage, each a spark of
//! the last reads; the Energy it holds; its Health against its design and
//! the Cycles it's been through; its Charge Type and Charge Limit, the
//! limit's choices a step in where Joel's account may set it; its
//! Temperature as a bar; its Model and when it was Made; and Power's
//! headline, its charge and how long it has. They're what the launcher's
//! `Power.qml` and `Power.js` made of the same supply records, moved here
//! so the shell words, sorts and builds nothing (Joel, 2026-11-10: "the
//! daemons need to be the data layer").
//!
//! A choice of limit asks the daemon's own `charge-limit` act, and the
//! row moves as the supply is read again. The last reads are kept only
//! between reads made while Power is watched ([`Power::forget`] drops
//! them), so a shut launcher costs nothing.
//!
//! Long because every row of the one supply, its drawer and its choices
//! change together as Power's one design does, and its cases stay beside
//! the private pieces they prove.

use std::cmp::Ordering;

use serde_json::{Value, json};

use crate::rows::{GRAPHED, Made, Ready, bar, figure, item, spark};
use crate::supply::RESTART_PCT;
use crate::words::Words;

/// The place the launcher shows these rows in.
pub const PLACE: &str = "power";
/// Reads nearer than this to the last a spark took are left out of it.
const SAMPLED_MS: u64 = 1000;
/// Reads further apart than this start a spark afresh.
const STALE_MS: u64 = 5000;
/// Below this share, draining, the battery wants Joel to plug in, as the
/// margin's battery says by default, so its bar warns.
const LOW: i64 = 20;
/// A lithium battery's temperature is drawn against the most it's made
/// to run at, and warns from where it's too warm to charge well.
const HOTTEST: f64 = 60.0;
const WARM: f64 = 45.0;
/// A battery holding under four fifths of its design is worn, as makers
/// count it, so its bar warns.
const WORN: f64 = 0.8;
/// The limits a step into the charge limit offers; the daemon starts
/// charging again a little below each.
const STOPS: [i64; 5] = [60, 70, 80, 90, 100];
/// The parts of a run said in one, as "45.0 W · 15.0 V".
const DOT: &str = " \u{b7} ";

/// The reads kept between one and the next while Power is watched: the
/// watts drawn or charging at, and the volts, with when they were last
/// taken.
#[derive(Clone, Debug, Default)]
pub struct Power {
    at: Option<u64>,
    watts: Vec<f64>,
    volts: Vec<f64>,
}

impl Power {
    /// Drops every kept read, as nothing watches Power any more.
    pub fn forget(&mut self) {
        *self = Power::default();
    }

    /// Power's rows and headline from the daemon's supply records, read
    /// `now` (in ms), its sparks taking this read. A machine with no
    /// battery has none, as the shell answered nothing without one.
    pub fn read(&mut self, supplies: &[Value], now: u64, words: &Words) -> Ready {
        let Some(battery) = battery_of(supplies) else {
            return Ready::default();
        };
        self.push(battery, now);
        let say = Say { words };
        let mut made = vec![say.charger(supplies)];
        let (own, charge) = say.own(battery, self);
        made.extend(own);
        let mut rows = Vec::new();
        crate::rows::placed(PLACE, made, None, 0, &mut rows);
        let mut head =
            json!({ "key": format!("head:{PLACE}"), "place": PLACE, "said": say.doing(battery) });
        if let Some(charge) = charge {
            head["graph"] = charge;
        }
        Ready {
            rows,
            head: Some(head),
        }
    }

    /// Adds this read to both sparks, at most once a second, starting
    /// them afresh after a gap.
    fn push(&mut self, battery: &Value, now: u64) {
        if self
            .at
            .is_some_and(|at| now.saturating_sub(at) < SAMPLED_MS)
        {
            return;
        }
        let fresh = self.at.is_some_and(|at| now.saturating_sub(at) <= STALE_MS);
        let watts = battery["watts"]
            .as_f64()
            .or_else(|| battery["upower"]["rate_w"].as_f64());
        let add = |kept: &mut Vec<f64>, value: Option<f64>| {
            if !fresh {
                kept.clear();
            }
            if let Some(value) = value {
                kept.push(value);
                let from = kept.len().saturating_sub(GRAPHED);
                kept.drain(..from);
            }
        };
        add(&mut self.watts, watts);
        add(&mut self.volts, battery["volts"].as_f64());
        self.at = Some(now);
    }
}

/// The machine's own battery, of the supplies, by key: a battery whose
/// scope is a device's, as a mouse's is, is that device's.
fn battery_of(supplies: &[Value]) -> Option<&Value> {
    let mut batteries: Vec<&Value> = supplies
        .iter()
        .filter(|each| each["type"] == "Battery" && each["scope"] != "Device")
        .collect();
    batteries.sort_by(|a, b| by_name(text(&a["key"]), text(&b["key"])));
    batteries.first().copied()
}

/// Two names A to Z as they're read, numbers by their value, so BAT9
/// comes before BAT10, as the shell's `Order.byName` does.
fn by_name(a: &str, b: &str) -> Ordering {
    let runs = |name: &str| {
        let mut out: Vec<String> = Vec::new();
        for c in name.to_lowercase().chars() {
            match out.last_mut() {
                Some(run)
                    if run.starts_with(|d: char| d.is_ascii_digit()) == c.is_ascii_digit() =>
                {
                    run.push(c);
                }
                _ => out.push(c.to_string()),
            }
        }
        out
    };
    let (a, b) = (runs(a), runs(b));
    for (one, other) in a.iter().zip(&b) {
        let said = match (one.parse::<u64>(), other.parse::<u64>()) {
            (Ok(one), Ok(other)) => one.cmp(&other),
            _ => one.cmp(other),
        };
        if said.is_ne() {
            return said;
        }
    }
    a.len().cmp(&b.len())
}

/// Each USB-C port the machine charges through, by its number, online
/// or not: a supply named as `ucsi-source-psy-USBC000:001` is port 1.
fn ports_of(supplies: &[Value]) -> Vec<(u64, &Value)> {
    let mut ports: Vec<(u64, &Value)> = supplies
        .iter()
        .filter_map(|record| Some((usb_c(text(&record["name"]))?, record)))
        .collect();
    ports.sort_by_key(|(n, _)| *n);
    ports
}

fn usb_c(name: &str) -> Option<u64> {
    let (before, n) = name.rsplit_once(':')?;
    let at = before.rfind("USBC")?;
    let controller = &before[at + 4..];
    let digits = |run: &str| !run.is_empty() && run.chars().all(|c| c.is_ascii_digit());
    if digits(controller) && digits(n) {
        n.parse().ok()
    } else {
        None
    }
}

/// Whether a charger is plugged in other than at a USB-C port, as a
/// barrel or a dock is.
fn plugged_elsewhere(supplies: &[Value]) -> bool {
    supplies.iter().any(|each| {
        each["type"] == "Mains" && usb_c(text(&each["name"])).is_none() && each["online"] == true
    })
}

/// What a USB-C port's record says of it: whether it charges the
/// machine, what it gives (now where it says so, else what it's offered),
/// its protocol, whether anything's plugged in and whether the machine
/// is giving the power.
struct Port {
    online: bool,
    volts: Option<f64>,
    amps: Option<f64>,
    protocol: &'static str,
    partner: bool,
    giving: bool,
}

fn port_of(record: &Value) -> Port {
    let typec = &record["typec"];
    Port {
        online: record["online"] == true,
        volts: record["volts"]
            .as_f64()
            .or_else(|| record["max_volts"].as_f64()),
        amps: record["max_amps"]
            .as_f64()
            .or_else(|| record["amps"].as_f64()),
        protocol: protocol(text(&record["usb_type"])),
        partner: !typec["partner"].is_null(),
        giving: typec["power_role"] == "source",
    }
}

impl Port {
    /// The watts it charges the machine at, where it does.
    fn charging(&self) -> Option<f64> {
        match (self.online, self.volts, self.amps) {
            (true, Some(volts), Some(amps)) if volts > 0.0 && amps > 0.0 => Some(volts * amps),
            _ => None,
        }
    }
}

/// A USB-C port's protocol as a person calls it, from the one its supply
/// marks in use: power delivery negotiated, programmable, or only Type-C's
/// own current; anything not named is plain USB, and nothing said is
/// nothing.
fn protocol(usb_type: &str) -> &'static str {
    match usb_type {
        "" => "",
        "PD" | "PD_DRP" => "USB PD",
        "PD_PPS" => "USB PD PPS",
        "C" => "USB-C",
        _ => "USB",
    }
}

/// Every protocol a port's supply names, in use or not, once each.
fn protocols(usb_types: &[Value]) -> String {
    let mut each: Vec<&str> = Vec::new();
    for one in usb_types.iter().map(text) {
        let named = protocol(one);
        if !named.is_empty() && !each.contains(&named) {
            each.push(named);
        }
    }
    each.join(DOT)
}

/// A field's text, or "" where it has none.
fn text(value: &Value) -> &str {
    value.as_str().unwrap_or_default()
}

/// Whether the kernel said something of it: neither nothing nor
/// "Unknown".
fn said(value: &Value) -> bool {
    matches!(value.as_str(), Some(text) if !text.is_empty() && text != "Unknown")
}

/// What the battery is doing, as `UPower` works it out, else as the
/// kernel says it.
fn state_of(battery: &Value) -> String {
    battery["upower"]["state"].as_str().map_or_else(
        || {
            battery["status"]
                .as_str()
                .unwrap_or("unknown")
                .to_lowercase()
        },
        str::to_owned,
    )
}

/// How long something's lasted, the largest two of days, hours and
/// minutes, leaving out the smaller when it's none, as the shell's
/// `Interfaces.lasted` says it. Machine's rows say how long it slept by
/// it too.
pub fn lasted(seconds: f64) -> String {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a battery's time is a few days of seconds, and never below none"
    )]
    let whole = seconds.max(0.0).floor() as u64;
    let (days, hours, minutes) = (whole / 86_400, whole % 86_400 / 3600, whole % 3600 / 60);
    match (days, hours, minutes) {
        (0, 0, 0) => "under a minute".to_owned(),
        (0, 0, m) => format!("{m} min"),
        (0, h, 0) => format!("{h} h"),
        (0, h, m) => format!("{h} h {m} min"),
        (d, 0, _) => format!("{d} d"),
        (d, h, _) => format!("{d} d {h} h"),
    }
}

/// When a battery was made, as a date in full, or its month alone when it
/// says no day, or nothing when what it says isn't a date.
fn made_of(said: &str) -> String {
    let mut parts = said.split('-').map(|part| part.parse::<u32>().ok());
    let (Some(Some(year)), Some(Some(month))) = (parts.next(), parts.next()) else {
        return String::new();
    };
    if year < 1990 || !(1..=12).contains(&month) {
        return String::new();
    }
    match parts.next().flatten() {
        Some(day) if (1..=31).contains(&day) => format!("{year}-{month:02}-{day:02}"),
        _ => format!("{year}-{month:02}"),
    }
}

/// A spark of the last reads, each a share of the most read, or of
/// `least` where every read is under it, as the shell's `Machine.spark`.
pub(crate) fn shares(values: &[f64], least: f64) -> Value {
    let most = values.iter().copied().fold(least, f64::max);
    let drawn: Vec<f64> = values
        .iter()
        .map(|value| if most > 0.0 { value / most } else { 0.0 })
        .collect();
    spark(&drawn)
}

/// What the rows are said with.
struct Say<'a> {
    words: &'a Words,
}

impl Say<'_> {
    /// A part's name, as the vocabulary says it.
    fn name(&self, id: &str, fill: &[(&str, &str)]) -> String {
        self.words.name(&format!("power:{id}"), fill)
    }

    /// A part's words, by the phrase's name: Power's own where `part` is
    /// "".
    fn says(&self, part: &str, phrase: &str, fill: &[(&str, &str)]) -> String {
        let id = if part.is_empty() {
            PLACE.to_owned()
        } else {
            format!("power:{part}")
        };
        self.words.said(&id, phrase, fill)
    }

    fn measured(&self, value: f64) -> String {
        self.words.significant(value)
    }

    /// A part saying what it is, Return copying it.
    fn leaf(&self, id: &str, side: String) -> Made {
        figure(id.to_owned(), self.name(id, &[]), side, None, String::new())
    }

    /// The charge and what the battery is doing, a dot between: "36% ·
    /// 3 h 10 min to empty". A charge the battery doesn't say is left out.
    fn doing(&self, battery: &Value) -> String {
        let upower = &battery["upower"];
        let time = |field: &str| upower[field].as_f64().filter(|s| *s > 0.0);
        let how = match state_of(battery).as_str() {
            "charging" => time("time_to_full_s").map_or_else(
                || self.says("", "charging", &[]),
                |s| self.says("", "full", &[("time", &lasted(s))]),
            ),
            "discharging" => time("time_to_empty_s").map_or_else(
                || self.says("", "battery", &[]),
                |s| self.says("", "empty", &[("time", &lasted(s))]),
            ),
            "full" => self.says("", "whole", &[]),
            _ => self.says("", "holding", &[]),
        };
        match battery["capacity_pct"].as_i64() {
            Some(charge) => format!("{charge}%{DOT}{how}"),
            None => how,
        }
    }

    /// The Charger: what's plugged in, briefly, holding each USB-C port
    /// and what it gives, or only saying so where the machine has none.
    fn charger(&self, supplies: &[Value]) -> Made {
        let ports: Vec<(u64, &Value, Port)> = ports_of(supplies)
            .into_iter()
            .map(|(n, record)| (n, record, port_of(record)))
            .collect();
        let charging = ports
            .iter()
            .find_map(|(n, _, port)| Some((n, port.charging()?)));
        let online = ports.iter().find(|(_, _, port)| port.online);
        let side = if let Some((n, watts)) = charging {
            self.says(
                "charger",
                "charging",
                &[("watts", &self.measured(watts)), ("n", &n.to_string())],
            )
        } else if let Some((n, _, _)) = online {
            self.says("charger", "on", &[("n", &n.to_string())])
        } else if plugged_elsewhere(supplies) {
            self.says("charger", "plugged", &[])
        } else {
            self.says("charger", "none", &[])
        };
        let mut charger = self.leaf("charger", side);
        charger.kind = "charger";
        if ports.is_empty() {
            return charger;
        }
        "charging".clone_into(&mut charger.glyph);
        charger.note = if ports.len() == 1 {
            self.says("charger", "port", &[])
        } else {
            self.says("charger", "ports", &[("count", &ports.len().to_string())])
        };
        charger.inside = ports
            .iter()
            .map(|(n, record, port)| self.port_row(*n, record, port))
            .collect();
        charger
    }

    /// A USB-C port: what it gives in a few words, and its drawer.
    fn port_row(&self, n: u64, record: &Value, port: &Port) -> Made {
        let words = |phrase: &str| self.says("port", phrase, &[]);
        let with_protocol = |first: String| {
            [first, port.protocol.to_owned()]
                .into_iter()
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join(DOT)
        };
        let side = match (port.charging(), port.volts, port.amps) {
            (Some(watts), Some(volts), Some(amps)) => with_protocol(format!(
                "{} W{DOT}{} V{DOT}{} A",
                self.measured(watts),
                self.measured(volts),
                self.measured(amps)
            )),
            _ if port.online => with_protocol(words("charging")),
            _ if port.giving => words("giving"),
            _ if port.partner => words("plugged"),
            _ => words("empty"),
        };
        let n = n.to_string();
        let name = self.name("port", &[("n", &n)]);
        let drawer = self.port_drawer(record, &name);
        let mut row = figure(format!("port:{n}"), name, side, None, String::new());
        row.kind = "port";
        "charging".clone_into(&mut row.glyph);
        row.drawer = Some(drawer);
        row
    }

    /// A USB-C port's drawer: which way its power and data go, how it
    /// negotiates, what's plugged in and what that offers, the power
    /// delivery revisions, and its supply's name last.
    fn port_drawer(&self, record: &Value, title: &str) -> Value {
        let typec = &record["typec"];
        let partner = &typec["partner"];
        let drawer = |phrase: &str, fill: &[(&str, &str)]| {
            self.words.said("power:port:drawer", phrase, fill)
        };
        let mode = text(&typec["operation_mode"]);
        let mode = if mode.is_empty() {
            String::new()
        } else {
            Some(drawer(&format!("mode:{mode}"), &[]))
                .filter(|said| !said.is_empty())
                .unwrap_or_else(|| mode.to_owned())
        };
        let accessory = text(&partner["accessory"]);
        let partner_revision = text(&partner["power_delivery_revision"]);
        let facts = [
            ("power-role", text(&typec["power_role"]).to_owned(), false),
            ("data-role", text(&typec["data_role"]).to_owned(), false),
            ("mode", mode, false),
            (
                "offers",
                self.offers(&partner["source_capabilities"]),
                false,
            ),
            (
                "protocols",
                protocols(
                    record["usb_types"]
                        .as_array()
                        .map(Vec::as_slice)
                        .unwrap_or_default(),
                ),
                false,
            ),
            (
                "partner",
                if accessory.is_empty() || accessory == "none" {
                    String::new()
                } else {
                    drawer("accessory", &[("mode", accessory)])
                },
                false,
            ),
            (
                "revision",
                text(&typec["power_delivery_revision"]).to_owned(),
                false,
            ),
            (
                "partner-revision",
                if partner_revision == "0.0" {
                    String::new()
                } else {
                    partner_revision.to_owned()
                },
                false,
            ),
            ("supply", text(&record["name"]).to_owned(), true),
        ];
        let facts: Vec<Value> = facts
            .into_iter()
            .filter(|(_, said, _)| !said.is_empty())
            .map(|(id, said, identifies)| {
                let name = self.words.name(&format!("power:port:{id}"), &[]);
                let mut fact = json!({ "name": name, "said": said });
                if identifies {
                    fact["identifies"] = json!(true);
                }
                fact
            })
            .collect();
        // Its eyebrow names where it is, so it's drawn as a name is, as
        // Power.qml drew it (Drawer.qml's eyebrowNames).
        json!({ "eyebrow": drawer("eyebrow", &[]), "eyebrowNames": true, "title": title,
            "facts": facts })
    }

    /// What a charger offers: "5 V · 9 V · 20 V at 3 A" when every one
    /// gives the same current, or each with its own; a figure offered by
    /// name, never measured, with as many digits as it has.
    fn offers(&self, capabilities: &Value) -> String {
        let Some(offers) = capabilities.as_array().filter(|all| !all.is_empty()) else {
            return String::new();
        };
        let drawer = |phrase: &str, fill: &[(&str, &str)]| {
            self.words.said("power:port:drawer", phrase, fill)
        };
        let offered = |value: &Value| self.words.number(value.as_f64().unwrap_or_default(), 2);
        let each = |offer: &Value| {
            if offer["to_volts"].is_null() {
                format!("{} V", offered(&offer["volts"]))
            } else {
                drawer(
                    "range",
                    &[
                        ("from", &offered(&offer["volts"])),
                        ("to", &offered(&offer["to_volts"])),
                    ],
                )
            }
        };
        let at = |what: String, offer: &Value| {
            drawer(
                "at",
                &[("offers", &what), ("amps", &offered(&offer["amps"]))],
            )
        };
        if offers
            .iter()
            .all(|offer| offer["amps"] == offers[0]["amps"])
        {
            let all: Vec<String> = offers.iter().map(each).collect();
            return at(all.join(DOT), &offers[0]);
        }
        offers
            .iter()
            .map(|offer| at(each(offer), offer))
            .collect::<Vec<_>>()
            .join(DOT)
    }

    /// The battery's own rows, all but the Charger's, and its charge's
    /// bar, which the headline draws too.
    fn own(&self, battery: &Value, kept: &Power) -> (Vec<Made>, Option<Value>) {
        let mut rows = Vec::new();
        let state = state_of(battery);
        let charge = battery["capacity_pct"].as_i64();
        #[expect(clippy::cast_precision_loss, reason = "a share is at most 100")]
        let barred = charge.map(|c| bar(c as f64 / 100.0, state == "discharging" && c <= LOW));
        if let Some(charge) = charge {
            let mut row = self.leaf("charge", format!("{charge}%"));
            row.graph.clone_from(&barred);
            if said(&battery["capacity_level"]) {
                let level = text(&battery["capacity_level"]).to_lowercase();
                row.note = self.says("charge", "level", &[("level", &level)]);
            }
            rows.push(row);
        }
        rows.extend(self.rate(&state, kept));
        rows.extend(self.energy(battery));
        rows.extend(self.voltage(battery, kept));
        rows.extend(self.health(battery));
        if let Some(cycles) = battery["cycles"].as_f64() {
            rows.push(self.leaf("cycles", self.words.number(cycles, 0)));
        }
        if said(&battery["charge_type"]) {
            let mut row = self.leaf("type", text(&battery["charge_type"]).to_owned());
            let types: Vec<&str> = battery["charge_types"]
                .as_array()
                .map(|all| all.iter().map(text).collect())
                .unwrap_or_default();
            if let [some @ .., last] = types.as_slice()
                && !some.is_empty()
            {
                row.note = self.says("type", "of", &[("some", &some.join(", ")), ("last", *last)]);
            }
            rows.push(row);
        }
        rows.extend(self.limit(battery));
        if let Some(degrees) = battery["temperature_c"].as_f64() {
            let mut row = self.leaf("temperature", format!("{} \u{b0}C", self.measured(degrees)));
            row.graph = Some(bar(degrees / HOTTEST, degrees >= WARM));
            row.note = self.says("temperature", "of", &[("most", &HOTTEST.to_string())]);
            rows.push(row);
        }
        rows.extend(self.model(battery));
        let made = made_of(text(&battery["made"]));
        if !made.is_empty() {
            rows.push(self.leaf("made", made));
        }
        (rows, barred)
    }

    /// What the machine is Drawing or Charging at, as a spark of the last
    /// reads, while it's a twentieth of a watt or more.
    fn rate(&self, state: &str, kept: &Power) -> Option<Made> {
        let watts = kept.watts.last().copied().unwrap_or_default();
        if watts < 0.05 {
            return None;
        }
        let charging = state == "charging";
        let mut row = self.leaf("rate", format!("{} W", self.measured(watts)));
        row.name = self
            .words
            .name_in("power:rate", if charging { "charging" } else { "" }, &[]);
        row.graph = Some(shares(&kept.watts, 1.0));
        row.note = self.says("rate", if charging { "into" } else { "from" }, &[]);
        Some(row)
    }

    /// The energy it holds, now and full, in watt hours: as it counts
    /// them, or from its charge at its design voltage, as `UPower` works it
    /// out. Its drawer holds the charge behind it. A stand-in's can be too
    /// small to say anything true, so it's said from a tenth of a unit.
    fn energy(&self, battery: &Value) -> Option<Made> {
        let designed = battery["min_design_volts"].as_f64();
        let of = |wh: &str, ah: &str| {
            battery[wh]
                .as_f64()
                .or_else(|| Some(battery[ah].as_f64()? * designed?))
        };
        let full = of("energy_full_wh", "charge_full_ah").filter(|full| *full >= 0.1)?;
        let now = of("energy_wh", "charge_ah");
        let design = of("energy_design_wh", "charge_design_ah");
        let mut facts: Vec<Value> = [
            ("energy:charge", "charge_ah"),
            ("energy:full", "charge_full_ah"),
            ("energy:design", "charge_design_ah"),
        ]
        .into_iter()
        .filter_map(|(id, field)| {
            let ah = battery[field].as_f64()?;
            Some(json!({ "name": self.name(id, &[]), "said": format!("{} Ah", self.measured(ah)) }))
        })
        .collect();
        if let Some(design) = design {
            facts.push(json!({ "name": self.name("energy:designed", &[]),
                "said": format!("{} Wh", self.measured(design)) }));
        }
        if !facts.is_empty() {
            facts.push(
                json!({ "name": self.name("device", &[]), "said": text(&battery["name"]),
                "identifies": true }),
            );
        }
        let side = match now {
            Some(now) => self.says(
                "energy",
                "of",
                &[("now", &self.measured(now)), ("full", &self.measured(full))],
            ),
            None => format!("{} Wh", self.measured(full)),
        };
        let mut row = self.leaf("energy", side);
        row.note = self.says("energy", if now.is_some() { "note" } else { "full" }, &[]);
        if !facts.is_empty() {
            row.drawer = Some(json!({ "eyebrow": self.says("energy", "eyebrow", &[]),
                "eyebrowNames": true, "title": self.name("energy", &[]), "facts": facts }));
        }
        Some(row)
    }

    /// Its voltage, as a spark drawn above the lowest read, by a fifth of
    /// a volt at least, since it moves by tenths around its own and would
    /// otherwise be a flat line at the top.
    fn voltage(&self, battery: &Value, kept: &Power) -> Option<Made> {
        let volts = kept.volts.last().copied().filter(|volts| *volts >= 0.1)?;
        let lowest = kept.volts.iter().copied().fold(f64::INFINITY, f64::min);
        let above: Vec<f64> = kept.volts.iter().map(|each| each - lowest).collect();
        let mut row = self.leaf("voltage", format!("{} V", self.measured(volts)));
        row.graph = Some(shares(&above, 0.2));
        if let Some(designed) = battery["min_design_volts"].as_f64().filter(|d| *d != 0.0) {
            row.note = self.says(
                "voltage",
                "designed",
                &[("volts", &self.measured(designed))],
            );
        }
        Some(row)
    }

    /// Its health against its design and the cycles it's been through,
    /// each a part of its side, the kernel's own word for it the note.
    fn health(&self, battery: &Value) -> Option<Made> {
        let ratio = battery["health_ratio"].as_f64();
        let cycles = battery["cycles"].as_f64();
        let mut items = Vec::new();
        if let Some(ratio) = ratio {
            let pct = (ratio * 100.0).round().to_string();
            items.push(self.says("health", "design", &[("pct", &pct)]));
        }
        if let Some(cycles) = cycles {
            items.push(if cycles.total_cmp(&1.0).is_eq() {
                self.says("health", "cycle", &[])
            } else {
                self.says(
                    "health",
                    "cycles",
                    &[("count", &self.words.number(cycles, 0))],
                )
            });
        }
        let worded = said(&battery["health"]);
        if items.is_empty() && !worded {
            return None;
        }
        let health = text(&battery["health"]);
        let side = if items.is_empty() {
            health.to_owned()
        } else {
            items.join(DOT)
        };
        let mut row = self.leaf("health", side);
        if !items.is_empty() {
            row.side = items
                .iter()
                .zip([2, 1])
                .map(|(said, keep)| json!({ "said": said, "keep": keep }))
                .collect();
        }
        row.graph = ratio.map(|ratio| bar(ratio, ratio < WORN));
        if worded && !items.is_empty() {
            let health = health.to_lowercase();
            row.note = self.says("health", "says", &[("health", &health)]);
        }
        Some(row)
    }

    /// The Charge Limit: where charging stops and starts again, its
    /// choices a step in when Joel's account may set it, and otherwise a
    /// figure saying it's the firmware's to set.
    fn limit(&self, battery: &Value) -> Option<Made> {
        let mut row = self.limit_at(battery)?;
        for (choice, stop) in row.inside.iter_mut().zip(STOPS) {
            if let (Some(act), Some(becomes)) =
                (choice.acts.first_mut(), self.limit_becomes(battery, stop))
            {
                act["becomes"] = becomes;
            }
        }
        Some(row)
    }

    /// The Charge Limit's row as it will be served once its limit stops
    /// at `stop`, and starts again `RESTART_PCT` below it where the
    /// battery has a start, as `supply::set_limit` writes them, so the
    /// shell draws it as its ask goes and keeps no copy of how far below
    /// (Y2YBTS, 2026-11-11).
    fn limit_becomes(&self, battery: &Value, stop: i64) -> Option<Value> {
        let mut set = battery.clone();
        set["limit_stop_pct"] = json!(stop);
        if battery["limit_start_pct"].is_i64() {
            set["limit_start_pct"] = json!((stop - i64::from(RESTART_PCT)).max(0));
        }
        let mut rows = Vec::new();
        crate::rows::placed(PLACE, vec![self.limit_at(&set)?], None, 0, &mut rows);
        rows.into_iter().next()
    }

    /// The Charge Limit's row as the battery's limit stands, with a
    /// choice inside for each stop where the limit can be set.
    fn limit_at(&self, battery: &Value) -> Option<Made> {
        let stop = battery["limit_stop_pct"].as_i64()?;
        let start = battery["limit_start_pct"].as_i64();
        let side = match start {
            _ if stop == 100 && start.is_none_or(|start| start == 0) => {
                self.says("limit", "full", &[])
            }
            None => self.says("limit", "stop", &[("stop", &stop.to_string())]),
            Some(start) => self.says(
                "limit",
                "both",
                &[("stop", &stop.to_string()), ("start", &start.to_string())],
            ),
        };
        let mut row = self.leaf("limit", side);
        if battery["limit_settable"] != true {
            if battery["limit_settable"] == false {
                row.note = self.says("limit", "firmware", &[]);
            }
            return Some(row);
        }
        row.kind = "holder";
        row.copies = String::new();
        "charge".clone_into(&mut row.glyph);
        let supply = text(&battery["name"]);
        row.inside = STOPS
            .iter()
            .map(|each| {
                let n = each.to_string();
                let chosen = if stop == *each {
                    self.words.said("power:limit:stop", "chosen", &[])
                } else {
                    String::new()
                };
                Made {
                    id: format!("limit:stop:{n}"),
                    kind: "choice",
                    name: self.words.name("power:limit:stop", &[("n", &n)]),
                    glyph: "charge".to_owned(),
                    hue: "action",
                    side: if chosen.is_empty() {
                        vec![]
                    } else {
                        vec![item(&chosen)]
                    },
                    acts: vec![json!({ "key": "verb:set", "stays": true, "ask":
                        { "ask": "act", "act": "charge-limit", "supply": supply,
                            "stop_pct": each } })],
                    ..Made::default()
                }
            })
            .collect();
        Some(row)
    }

    /// Its Model, with its chemistry, its drawer its serial where the
    /// battery says one.
    fn model(&self, battery: &Value) -> Option<Made> {
        let what: Vec<&str> = [&battery["manufacturer"], &battery["model"]]
            .into_iter()
            .filter(|part| said(part))
            .map(text)
            .collect();
        if what.is_empty() {
            return None;
        }
        let what = what.join(" ");
        let side = if said(&battery["technology"]) {
            format!("{what}, {}", text(&battery["technology"]))
        } else {
            what
        };
        let mut row = self.leaf("model", side);
        if said(&battery["serial"]) {
            let eyebrow = if said(&battery["manufacturer"]) {
                self.says(
                    "model",
                    "eyebrow",
                    &[("maker", text(&battery["manufacturer"]))],
                )
            } else {
                String::new()
            };
            row.drawer = Some(
                json!({ "eyebrow": eyebrow, "eyebrowNames": true, "title": self.name("model", &[]),
                "facts": [{ "name": self.name("serial", &[]), "said": text(&battery["serial"]),
                    "identifies": true }] }),
            );
        }
        Some(row)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The words these cases say: the launcher's own (`Words::launchers`).
    fn words() -> Words {
        Words::launchers("", "")
    }

    fn battery() -> Value {
        json!({ "key": "supply:BAT0", "name": "BAT0", "type": "Battery", "status": "Charging",
            "capacity_pct": 36, "capacity_level": "Normal", "watts": 30.0, "volts": 16.2,
            "health_ratio": 0.92, "cycles": 312, "health": "Good", "limit_stop_pct": 80,
            "limit_start_pct": 75, "limit_settable": true,
            "upower": { "state": "charging", "time_to_full_s": 3000.0 } })
    }

    fn port(online: bool) -> Value {
        json!({ "key": "supply:ucsi-source-psy-USBC000:001",
            "name": "ucsi-source-psy-USBC000:001", "type": "USB", "online": online,
            "volts": 15.0, "max_amps": 3.0, "usb_type": "PD", "usb_types": ["C", "PD", "PD_PPS"],
            "typec": { "power_role": "sink", "operation_mode": "usb_power_delivery",
                "partner": { "source_capabilities": [
                    { "volts": 5.0, "to_volts": null, "amps": 3.0 },
                    { "volts": 20.0, "to_volts": null, "amps": 3.0 }] } } })
    }

    fn row<'a>(ready: &'a Ready, key: &str) -> &'a Value {
        ready
            .rows
            .iter()
            .find(|row| row["key"] == key)
            .unwrap_or_else(|| panic!("no {key} among {:?}", ready.rows))
    }

    #[test]
    fn the_headline_says_the_charge_and_how_long_it_has() {
        let ready = Power::default().read(&[battery()], 0, &words());
        let head = ready.head.unwrap();
        assert_eq!(head["said"], "36% \u{b7} 50 min to full");
        assert_eq!(head["graph"]["values"], json!([0.35]));
        assert_eq!(lasted(11_400.0), "3 h 10 min");
        assert_eq!(lasted(90_000.0), "1 d 1 h");
        assert_eq!(lasted(30.0), "under a minute");
    }

    #[test]
    fn no_battery_makes_no_rows() {
        assert_eq!(
            Power::default().read(&[port(true)], 0, &words()),
            Ready::default()
        );
    }

    #[test]
    fn the_charger_holds_each_port_and_what_it_gives() {
        let ready = Power::default().read(&[battery(), port(true)], 0, &words());
        let charger = row(&ready, "row:power:charger");
        assert_eq!(charger["side"][0]["said"], "45.0 W on port 1");
        assert_eq!(charger["note"], "one USB-C port");
        let first = row(&ready, "row:power:port:1");
        assert_eq!(first["thing"], "power:port:1");
        assert_eq!(first["parent"], "row:power:charger");
        assert_eq!(
            first["side"][0]["said"],
            "45.0 W \u{b7} 15.0 V \u{b7} 3.00 A \u{b7} USB PD"
        );
        let facts = first["drawer"]["facts"].as_array().unwrap();
        let offers = facts.iter().find(|fact| fact["name"] == "Offers");
        assert_eq!(
            offers,
            Some(&json!({ "name": "Offers", "said": "5 V \u{b7} 20 V at 3 A" }))
        );
        let supply = facts.last().unwrap();
        assert_eq!(
            (supply["name"].clone(), supply["identifies"].clone()),
            (json!("Supply"), json!(true))
        );
        let empty = Power::default().read(&[battery(), port(false)], 0, &words());
        assert_eq!(
            row(&empty, "row:power:port:1")["side"][0]["said"],
            "plugged in, giving nothing"
        );
        let none = Power::default().read(&[battery()], 0, &words());
        assert_eq!(
            row(&none, "row:power:charger")["side"][0]["said"],
            "none plugged in"
        );
    }

    #[test]
    fn the_battery_says_its_rate_health_and_limit() {
        let ready = Power::default().read(&[battery()], 0, &words());
        let rate = row(&ready, "row:power:rate");
        assert_eq!(
            (rate["name"].clone(), rate["side"][0]["said"].clone()),
            (json!("Charging"), json!("30.0 W"))
        );
        let health = row(&ready, "row:power:health");
        assert_eq!(
            health["side"],
            json!([{ "said": "92% of design", "keep": 2 }, { "said": "312 cycles", "keep": 1 }])
        );
        assert_eq!(health["note"], "the battery says it's good");
        let limit = row(&ready, "row:power:limit");
        assert_eq!(limit["side"][0]["said"], "stops at 80%, starts below 75%");
        let eighty = row(&ready, "row:power:limit:stop:80");
        assert_eq!(eighty["side"][0]["said"], "chosen");
        assert_eq!(
            eighty["acts"][0]["ask"],
            json!({ "ask": "act", "act": "charge-limit", "supply": "BAT0", "stop_pct": 80 })
        );
        assert_eq!(row(&ready, "row:power:limit:stop:60")["side"], json!([]));
        // Each choice's ask carries the Charge Limit's row as it will be
        // served once done, starting again 5 below.
        let becomes = &row(&ready, "row:power:limit:stop:60")["acts"][0]["becomes"];
        assert_eq!(becomes["key"], "row:power:limit");
        assert_eq!(becomes["side"][0]["said"], "stops at 60%, starts below 55%");
        assert_eq!(
            becomes["acts"], limit["acts"],
            "the Charge Limit keeps the acts it's served with now"
        );
    }

    #[test]
    fn the_sparks_take_a_read_a_second_while_watched() {
        let mut power = Power::default();
        let mut charged = battery();
        power.read(&[charged.clone()], 0, &words());
        charged["watts"] = json!(60.0);
        power.read(&[charged.clone()], 400, &words());
        let ready = power.read(&[charged.clone()], 1000, &words());
        assert_eq!(
            row(&ready, "row:power:rate")["graph"]["values"],
            json!([0.5, 1.0])
        );
        power.forget();
        let again = power.read(&[charged], 9000, &words());
        assert_eq!(
            row(&again, "row:power:rate")["graph"]["values"],
            json!([1.0])
        );
    }

    /// What a made row says at its right, its items joined.
    fn side_of(made: &Made) -> String {
        made.side
            .iter()
            .map(|item| text(&item["said"]))
            .collect::<Vec<_>>()
            .join(DOT)
    }

    #[test]
    fn a_port_says_its_protocol_and_what_it_gives() {
        assert_eq!(
            ["PD", "PD_PPS", "C", "SDP", ""].map(protocol),
            ["USB PD", "USB PD PPS", "USB-C", "USB", ""]
        );
        let words = words();
        let say = Say { words: &words };
        let side = |record: Value| side_of(&say.port_row(1, &record, &port_of(&record)));
        assert_eq!(
            side(json!({ "online": true, "volts": 5.0, "amps": 0.5 })),
            "2.50 W \u{b7} 5.00 V \u{b7} 0.500 A",
            "half an amp, to three figures"
        );
        assert_eq!(side(json!({ "online": true })), "charging");
        assert_eq!(
            side(json!({ "online": true, "usb_type": "C" })),
            "charging \u{b7} USB-C"
        );
        assert_eq!(
            side(json!({ "online": false, "typec": { "power_role": "source" } })),
            "powering what's plugged in"
        );
        assert_eq!(side(json!({ "online": false })), "empty");
    }

    #[test]
    fn a_ports_drawer_says_its_roles_its_offers_and_its_revisions() {
        let words = words();
        let say = Say { words: &words };
        assert_eq!(
            say.offers(&json!([
                { "volts": 5.0, "to_volts": null, "amps": 3.0 },
                { "volts": 20.0, "to_volts": null, "amps": 4.5 },
                { "volts": 3.3, "to_volts": 21.0, "amps": 2.25 }])),
            "5 V at 3 A \u{b7} 20 V at 4.5 A \u{b7} 3.3 to 21 V at 2.25 A",
            "each offer at its own current, as it names them"
        );
        let record = json!({ "name": "ucsi-source-psy-USBC000:001",
            "usb_types": ["C", "PD", "PD_PPS", "PD_DRP"],
            "typec": { "power_role": "sink", "data_role": "host",
                "operation_mode": "usb_power_delivery", "power_delivery_revision": "3.0",
                "partner": { "accessory": "none", "power_delivery_revision": "0.0" } } });
        let drawer = say.port_drawer(&record, "Port 1");
        let said: Vec<&str> = drawer["facts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|fact| text(&fact["said"]))
            .collect();
        assert_eq!(
            said,
            [
                "sink",
                "host",
                "USB Power Delivery",
                "USB-C \u{b7} USB PD \u{b7} USB PD PPS",
                "3.0",
                "ucsi-source-psy-USBC000:001"
            ],
            "its roles, its mode in words, its protocols once each, a partner saying none left out"
        );
        assert_eq!(
            drawer["eyebrowNames"], true,
            "its eyebrow is drawn as a name"
        );
        let audio = say.port_drawer(
            &json!({ "typec": { "partner": { "accessory": "audio",
                "power_delivery_revision": "2.0" } } }),
            "Port 2",
        );
        assert_eq!(
            audio["facts"],
            json!([{ "name": "power:port:partner", "said": "audio accessory" },
                { "name": "power:port:partner-revision", "said": "2.0" }])
        );
    }

    #[test]
    fn the_battery_is_the_machines_and_its_ports_go_by_number() {
        let supplies = [
            json!({ "key": "supply:hidpp_battery_0", "type": "Battery", "scope": "Device" }),
            json!({ "key": "supply:BAT0", "name": "BAT0", "type": "Battery", "scope": null }),
            json!({ "key": "supply:AC", "name": "AC", "type": "Mains", "online": false }),
            json!({ "name": "ucsi-source-psy-USBC000:002", "type": "USB" }),
            json!({ "name": "ucsi-source-psy-USBC000:001", "type": "USB" }),
        ];
        assert_eq!(
            battery_of(&supplies).map(|battery| text(&battery["name"])),
            Some("BAT0"),
            "the machine's battery, not a mouse's"
        );
        assert!(battery_of(&supplies[2..]).is_none(), "no battery");
        let ports: Vec<u64> = ports_of(&supplies).into_iter().map(|(n, _)| n).collect();
        assert_eq!(ports, [1, 2]);
        assert!(!plugged_elsewhere(&supplies), "the charger's off");
        assert!(
            plugged_elsewhere(&[json!({ "name": "AC", "type": "Mains", "online": true })]),
            "a charger on a barrel"
        );
    }

    #[test]
    fn the_headline_says_what_the_battery_is_doing() {
        let words = words();
        let say = Say { words: &words };
        assert_eq!(
            say.doing(&json!({ "capacity_pct": 36, "status": "Discharging",
                "upower": { "state": "discharging", "time_to_empty_s": 11_400 } })),
            "36% \u{b7} 3 h 10 min to empty"
        );
        assert_eq!(
            say.doing(&json!({ "capacity_pct": 80, "status": "Charging", "upower": null })),
            "80% \u{b7} charging"
        );
        assert_eq!(
            say.doing(&json!({ "capacity_pct": 100, "status": "Full" })),
            "100% \u{b7} full"
        );
        assert_eq!(
            say.doing(&json!({ "status": "Discharging" })),
            "on battery",
            "a charge nobody knows is left out, never said as none"
        );
    }

    #[test]
    fn the_energy_is_as_the_battery_counts_it_or_from_its_charge() {
        let words = words();
        let say = Say { words: &words };
        let counted = say
            .energy(&json!({ "name": "BAT0", "energy_wh": 30.0, "energy_full_wh": 50.0 }))
            .unwrap();
        assert_eq!(
            side_of(&counted),
            "30.0 of 50.0 Wh",
            "as the battery counts it"
        );
        let charged = say
            .energy(
                &json!({ "name": "BAT0", "charge_ah": 1.5, "charge_full_ah": 3.6,
                "charge_design_ah": 4.0, "min_design_volts": 15.0 }),
            )
            .unwrap();
        let drawer = charged.drawer.unwrap();
        let facts: Vec<&str> = drawer["facts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|fact| text(&fact["said"]))
            .collect();
        assert_eq!(
            facts,
            ["1.50 Ah", "3.60 Ah", "4.00 Ah", "60.0 Wh", "BAT0"],
            "the charge behind it, then what it was made to hold, then the battery by name"
        );
        assert_eq!(
            drawer["eyebrowNames"], true,
            "its eyebrow is drawn as a name"
        );
    }

    #[test]
    fn names_and_dates_read_as_the_shell_reads_them() {
        assert!(
            by_name("supply:BAT9", "supply:BAT10").is_lt(),
            "by the number's value"
        );
        assert_eq!(usb_c("ucsi-source-psy-USBC000:002"), Some(2));
        assert_eq!(usb_c("AC"), None);
        assert_eq!(made_of("2026-07-20"), "2026-07-20");
        assert_eq!(made_of("2026-07-00"), "2026-07");
        assert_eq!(made_of("1970-01-01"), "");
    }

    /// Every row Power serves but a choice has a drawer giving a little
    /// more than its line (Joel, 2026-11-11 22:00): what the battery's
    /// folder keeps behind it, in its own units. The Charge's says the
    /// energy now, full and designed, and the rate (Y2YBTS, 2026-11-11).
    #[test]
    fn every_power_row_has_a_drawer_saying_more() {
        let mut b = battery();
        for (field, value) in [
            ("energy_wh", json!(20.0)),
            ("energy_full_wh", json!(55.0)),
            ("energy_design_wh", json!(60.0)),
            ("min_design_volts", json!(15.4)),
            ("technology", json!("Li-poly")),
            ("manufacturer", json!("SMP")),
            ("model", json!("5B11")),
            ("charge_type", json!("Standard")),
            ("charge_types", json!(["Fast", "Standard", "Long_Life"])),
            ("temperature_c", json!(31.5)),
            ("made", json!("2024-03-05")),
        ] {
            b[field] = value;
        }
        let ready = Power::default().read(&[b, port(true)], 0, &words());
        let bare: Vec<&str> = ready
            .rows
            .iter()
            .filter(|each| each["kind"] != "choice")
            .filter(|each| {
                let drawer = &each["drawer"];
                let facts = drawer["facts"].as_array().is_some_and(|f| !f.is_empty());
                !(drawer["title"].is_string() && (facts || drawer["body"].is_string()))
            })
            .filter_map(|each| each["key"].as_str())
            .collect();
        assert!(
            bare.is_empty(),
            "rows whose drawer says nothing more: {bare:?}"
        );
        let facts = |key: &str| -> Vec<String> {
            row(&ready, key)["drawer"]["facts"]
                .as_array()
                .unwrap()
                .iter()
                .map(|fact| format!("{}={}", text(&fact["name"]), text(&fact["said"])))
                .collect()
        };
        assert_eq!(
            facts("row:power:charge"),
            [
                "capacity=36",
                "capacity_level=Normal",
                "status=Charging",
                "energy_now=20000000 \u{b5}Wh",
                "energy_full=55000000 \u{b5}Wh",
                "energy_full_design=60000000 \u{b5}Wh",
                "power_now=30000000 \u{b5}W",
                "Read From=/sys/class/power_supply/BAT0"
            ],
            "the charge says the energy now, full and designed, and the rate"
        );
        assert_eq!(
            facts("row:power:type"),
            [
                "charge_types=Fast [Standard] Long_Life",
                "Read From=/sys/class/power_supply/BAT0"
            ],
            "a choice of the kernel's says its line, the one in use in brackets"
        );
        assert_eq!(
            facts("row:power:temperature"),
            ["temp=315", "Read From=/sys/class/power_supply/BAT0"],
            "its temperature in tenths of a degree, as the kernel counts it"
        );
    }
}
