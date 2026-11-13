//! `eero profiles`, each profile on a row, and `eero profile`, one profile:
//! whether it's held off, what it blocks, its traffic over the last day,
//! and every device in it, on now or not.

use super::clock::Clock;
use super::figures::{bytes, joined, said, unknown, yes};
use super::marks::marks;
use super::nodes::traffic;
use super::text::{Align, Ink, Line, Page, Table, muted, plain, strong};
use super::{counted, device_name, head, location, profile_name, titled};
use crate::model::{Device, Filters, HomeNetwork, Profile};

#[derive(serde::Serialize)]
struct Every<'a> {
    profiles: &'a [Profile],
}

#[derive(serde::Serialize)]
struct One<'a> {
    profile: &'a Profile,
    /// Its devices that are on now.
    devices: Vec<&'a Device>,
}

pub fn all_json(model: &HomeNetwork) -> Result<String, String> {
    super::json::slice(
        model,
        "eero.profiles/1",
        Every {
            profiles: &model.profiles,
        },
    )
}

pub fn one_json(model: &HomeNetwork, profile: &Profile) -> Result<String, String> {
    let devices = super::json::on(model, |device| {
        profile
            .devices
            .iter()
            .any(|mac| mac.eq_ignore_ascii_case(&device.mac))
    });
    super::json::slice(model, "eero.profile/1", One { profile, devices })
}

/// What the profile names the device with hardware address `mac`.
fn member_name<'a>(profile: &'a Profile, mac: &str) -> Option<&'a str> {
    profile
        .members
        .iter()
        .find(|member| member.mac.eq_ignore_ascii_case(mac))
        .and_then(|member| member.name.as_deref())
}

fn on_now(model: &HomeNetwork, profile: &Profile) -> usize {
    profile
        .devices
        .iter()
        .filter(|mac| {
            model
                .devices
                .iter()
                .any(|device| device.mac.eq_ignore_ascii_case(mac))
        })
        .count()
}

/// Its schedules, as a count of them, or none.
fn schedules(profile: &Profile) -> Line {
    match profile.schedules {
        None => unknown(),
        Some(0) => vec![plain("none")],
        Some(count) => vec![plain(counted(count, "schedule"))],
    }
}

/// Each kind of content eero filters, by the word the views say it with.
fn kinds(filters: &Filters) -> [(Option<bool>, &'static str); 8] {
    [
        (filters.gaming, "gaming"),
        (filters.illegal, "illegal"),
        (filters.messaging, "messaging"),
        (filters.pornographic, "pornographic"),
        (filters.social, "social"),
        (filters.shopping, "shopping"),
        (filters.streaming, "streaming"),
        (filters.violent, "violent"),
    ]
}

/// What it blocks, as one fact: each kind of content, ads, and every site
/// and app it names, a filter that's off saying nothing; nothing, where
/// none is on, and a dash where eero said none of its filters.
fn blocks(filters: &Filters) -> Line {
    if kinds(filters).iter().all(|(on, _)| on.is_none()) {
        return unknown();
    }
    let mut parts: Vec<String> = kinds(filters)
        .into_iter()
        .filter(|(on, _)| *on == Some(true))
        .map(|(_, kind)| kind.to_owned())
        .collect();
    if filters.ad_block == Some(true) {
        parts.push("ads".into());
    } else if filters.ad_block_network == Some(true) {
        parts.push("ads network-wide".into());
    }
    parts.extend(filters.blocked_sites.iter().cloned());
    parts.extend(filters.blocked_apps.iter().cloned());
    if parts.is_empty() {
        return vec![plain("nothing")];
    }
    joined(parts.into_iter().map(|part| vec![plain(part)]).collect())
}

/// What it holds search and `YouTube` to, where it holds either.
fn enforces(filters: &Filters) -> Option<Line> {
    let parts: Vec<Line> = [
        (filters.safe_search, "safe search"),
        (filters.youtube_restricted, "YouTube restricted mode"),
    ]
    .into_iter()
    .filter(|(on, _)| *on == Some(true))
    .map(|(_, what)| vec![plain(what)])
    .collect();
    (!parts.is_empty()).then(|| joined(parts))
}

/// The sites it lets through, where it names any.
fn allows(filters: &Filters) -> Option<Line> {
    (!filters.allowed_sites.is_empty()).then(|| {
        joined(
            filters
                .allowed_sites
                .iter()
                .map(|site| vec![plain(site)])
                .collect(),
        )
    })
}

pub fn all(model: &HomeNetwork, clock: &Clock) -> Page {
    let mut page = Page::default();
    head(
        &mut page,
        model,
        clock,
        titled(model, Some(counted(model.profiles.len(), "profile"))),
    );
    // What a profile holds search to and the sites it lets through are
    // columns only while one has them, as a fact that's off says nothing.
    let enforcing = model
        .profiles
        .iter()
        .any(|profile| enforces(&profile.filters).is_some());
    let allowing = model
        .profiles
        .iter()
        .any(|profile| allows(&profile.filters).is_some());
    let mut headings = vec!["profile", "state", "paused", "blocks"];
    headings.extend(enforcing.then_some("enforces"));
    headings.extend(allowing.then_some("allows"));
    let mut aligns = vec![Align::Left; headings.len()];
    headings.extend(["schedule", "default", "devices", "on now", "down", "up"]);
    aligns.extend([
        Align::Left,
        Align::Left,
        Align::Right,
        Align::Right,
        Align::Point,
        Align::Point,
    ]);
    let mut table = Table::new(&aligns);
    table.header(&headings);
    for profile in &model.profiles {
        let usage = profile.usage.as_ref();
        let filters = &profile.filters;
        let mut cells = vec![
            vec![strong(profile_name(profile))],
            said(profile.state.as_deref()),
            yes(profile.paused),
            blocks(filters),
        ];
        if enforcing {
            cells.push(enforces(filters).unwrap_or_default());
        }
        if allowing {
            cells.push(allows(filters).unwrap_or_default());
        }
        cells.extend([
            schedules(profile),
            yes(profile.default),
            vec![plain(profile.devices.len().to_string())],
            vec![plain(on_now(model, profile).to_string())],
            usage.map_or_else(unknown, |usage| bytes(usage.download.total_bytes)),
            usage.map_or_else(unknown, |usage| bytes(usage.upload.total_bytes)),
        ]);
        table.row(cells);
    }
    page.extend(table.lines(0));
    page
}

pub fn one(model: &HomeNetwork, profile: &Profile, clock: &Clock, ink: &Ink) -> Page {
    let mut page = Page::default();
    let title = vec![
        strong(profile_name(profile)),
        muted(format!(" {} ", marks().separator)),
        muted("id "),
        plain(&profile.id),
    ];
    head(&mut page, model, clock, title);
    let filters = &profile.filters;
    // In the order its drawer says them, what it blocks first.
    let mut facts = vec![
        ("state", said(profile.state.as_deref())),
        ("paused", yes(profile.paused)),
        ("blocks", blocks(filters)),
    ];
    facts.extend(enforces(filters).map(|line| ("enforces", line)));
    facts.extend(allows(filters).map(|line| ("allows", line)));
    facts.extend([
        ("schedule", schedules(profile)),
        ("default", yes(profile.default)),
        ("traffic", traffic(profile.usage.as_ref(), ink)),
        (
            "devices",
            vec![
                plain(profile.devices.len().to_string()),
                muted(format!(" {} ", marks().separator)),
                plain(format!("{} on now", on_now(model, profile))),
            ],
        ),
    ]);
    page.facts(0, facts);
    page.blank();
    let mut table = Table::new(&[Align::Left, Align::Left, Align::Left, Align::Left]);
    table.header(&["hardware", "device", "node", "address"]);
    for mac in &profile.devices {
        let device = model
            .devices
            .iter()
            .find(|device| device.mac.eq_ignore_ascii_case(mac));
        table.row(match device {
            Some(device) => vec![
                vec![plain(mac)],
                vec![plain(device_name(device))],
                vec![strong(location(model, device.node.as_deref()))],
                said(device.address.as_deref()),
            ],
            // One that isn't on now is named as the profile names it, its
            // node saying it's not on.
            None => vec![
                vec![plain(mac)],
                said(member_name(profile, mac)),
                vec![muted("not on now")],
                Vec::new(),
            ],
        });
    }
    page.extend(table.lines(0));
    page
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(line: &Line) -> String {
        line.iter().map(|run| run.text.as_str()).collect()
    }

    #[test]
    fn a_profile_blocks_nothing_while_no_filter_is_on() {
        let filters = Filters {
            gaming: Some(false),
            safe_search: Some(false),
            ..Filters::default()
        };

        assert_eq!(text(&blocks(&filters)), "nothing");
        assert_eq!(enforces(&filters), None);
        assert_eq!(allows(&filters), None);
    }

    #[test]
    fn what_a_profile_blocks_is_one_fact_each_filter_on_named() {
        let filters = Filters {
            gaming: Some(true),
            social: Some(true),
            violent: Some(false),
            ad_block_network: Some(true),
            blocked_sites: vec!["example.com".into()],
            blocked_apps: vec!["tiktok".into()],
            youtube_restricted: Some(true),
            allowed_sites: vec!["example.org".into(), "example.net".into()],
            ..Filters::default()
        };

        assert_eq!(
            text(&blocks(&filters)),
            "gaming · social · ads network-wide · example.com · tiktok"
        );
        assert_eq!(
            enforces(&filters).map(|line| text(&line)).as_deref(),
            Some("YouTube restricted mode")
        );
        assert_eq!(
            allows(&filters).map(|line| text(&line)).as_deref(),
            Some("example.org · example.net")
        );
    }

    #[test]
    fn a_profile_eero_said_no_filter_of_gives_a_dash() {
        assert_eq!(text(&blocks(&Filters::default())), "–");
    }

    #[test]
    fn a_profile_counts_its_schedules() {
        let profile = |schedules| Profile {
            id: "1".into(),
            name: None,
            state: None,
            paused: None,
            devices: Vec::new(),
            members: Vec::new(),
            default: None,
            schedules,
            filters: Filters::default(),
            usage: None,
        };

        assert_eq!(text(&schedules(&profile(Some(0)))), "none");
        assert_eq!(text(&schedules(&profile(Some(2)))), "2 schedules");
        assert_eq!(text(&schedules(&profile(None))), "–");
    }
}
