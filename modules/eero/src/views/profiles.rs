//! `eero profiles`, each profile on a row, and `eero profile`, one profile
//! and every device in it, on now or not.

use super::clock::Clock;
use super::figures::{said, yes};
use super::marks::marks;
use super::text::{Align, Page, Table, muted, plain, strong};
use super::{counted, device_name, head, location, profile_name, titled};
use crate::model::{Device, HomeNetwork, Profile};

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
    super::json::slice(model, One { profile, devices })
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

pub fn all(model: &HomeNetwork, clock: &Clock) -> Page {
    let mut page = Page::default();
    head(
        &mut page,
        model,
        clock,
        titled(model, Some(counted(model.profiles.len(), "profile"))),
    );
    let mut table = Table::new(&[
        Align::Left,
        Align::Left,
        Align::Left,
        Align::Right,
        Align::Right,
    ]);
    table.header(&["profile", "state", "paused", "devices", "on now"]);
    for profile in &model.profiles {
        table.row(vec![
            vec![strong(profile_name(profile))],
            said(profile.state.as_deref()),
            yes(profile.paused),
            vec![plain(profile.devices.len().to_string())],
            vec![plain(on_now(model, profile).to_string())],
        ]);
    }
    page.extend(table.lines(0));
    page
}

pub fn one(model: &HomeNetwork, profile: &Profile, clock: &Clock) -> Page {
    let mut page = Page::default();
    let title = vec![
        strong(profile_name(profile)),
        muted(format!(" {} ", marks().separator)),
        muted("id "),
        plain(&profile.id),
    ];
    head(&mut page, model, clock, title);
    page.facts(
        0,
        vec![
            ("state", said(profile.state.as_deref())),
            ("paused", yes(profile.paused)),
            (
                "devices",
                vec![
                    plain(profile.devices.len().to_string()),
                    muted(format!(" {} ", marks().separator)),
                    plain(format!("{} on now", on_now(model, profile))),
                ],
            ),
        ],
    );
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
            None => vec![
                vec![plain(mac)],
                vec![muted("not on now")],
                Vec::new(),
                Vec::new(),
            ],
        });
    }
    page.extend(table.lines(0));
    page
}
