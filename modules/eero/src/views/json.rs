//! Each view's slice of the record as JSON, under the record's own names,
//! so a program reads `eero <view> --json` with the types `model.rs`
//! defines. Every slice opens with when eero last answered and whether
//! it's answering, since a figure means little without its age; each view
//! names what follows. A slice names its schema first, `eero.<view>/1`, as
//! every answer of ours does (docs/commands.md), so a program can tell one
//! view's from another's and a change in shape from a change in figures.

use serde::Serialize;

use crate::model::{Cloud, Device, HomeNetwork};

/// A view's figures after the record's head.
#[derive(Serialize)]
pub struct Slice<'a, T: Serialize> {
    schema: &'a str,
    read: &'a str,
    cloud: &'a Cloud,
    name: &'a str,
    #[serde(flatten)]
    figures: T,
}

/// A view's figures as JSON, after the record's head.
pub fn slice<T: Serialize>(
    model: &HomeNetwork,
    schema: &str,
    figures: T,
) -> Result<String, String> {
    let slice = Slice {
        schema,
        read: &model.read,
        cloud: &model.cloud,
        name: &model.name,
        figures,
    };
    let mut text = serde_json::to_string_pretty(&slice).map_err(|e| e.to_string())?;
    text.push('\n');
    Ok(text)
}

/// The devices on now that something wants.
pub fn on(model: &HomeNetwork, wanted: impl Fn(&Device) -> bool) -> Vec<&Device> {
    model
        .devices
        .iter()
        .filter(|device| wanted(device))
        .collect()
}
