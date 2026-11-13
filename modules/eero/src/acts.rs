//! The two things done to the home network rather than read of it: start
//! eero's own speed test, and turn "Optimize for conferencing and gaming"
//! on or off. The CLI does them (`eero speedtest`, `eero sqm on|off`), and
//! the daemon does them as the shell asks one by a row's act (listeners.rs),
//! so the shell runs no program to change the home network.

use serde_json::Value;

use crate::api;
use crate::cloud::{Cloud, Failure};

/// What's asked of the home network.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Act {
    /// Start eero's own speed test, which it takes most of a minute over.
    SpeedTest,
    /// Turn conferencing and gaming's queueing on, or off.
    Conferencing(bool),
}

impl Act {
    /// The act a wire ask names, as a row's act serves it: `{"ask": "act",
    /// "act": "speedtest"}` or `{"ask": "act", "act": "sqm", "on": true}`,
    /// or why it isn't one.
    pub fn asked(said: &Value) -> Result<Act, String> {
        match said["act"].as_str() {
            Some("speedtest") => Ok(Act::SpeedTest),
            Some("sqm") => said["on"]
                .as_bool()
                .map(Act::Conferencing)
                .ok_or_else(|| "conferencing and gaming is asked on or off".to_owned()),
            Some(other) => Err(format!("eero does nothing called {other}")),
            None => Err("an act names what it does".to_owned()),
        }
    }
}

/// Does `act` to the network at `network`, such as `/2.2/networks/1001`.
/// Conferencing is read back after, since eero answers a change it ignores
/// as gladly as one it takes.
pub fn done(cloud: &mut Cloud, network: &str, act: Act) -> Result<(), Failure> {
    match act {
        Act::SpeedTest => {
            cloud.post(&format!("{network}/speedtest"), None)?;
        }
        Act::Conferencing(on) => {
            let named = api::network(network, &cloud.get(network)?);
            let settings = named.settings_url.ok_or_else(|| {
                Failure::Refused(format!("eero names no settings for {}", named.name))
            })?;
            cloud.put(&settings, &api::conferencing(on))?;
            let now = api::network(network, &cloud.get(network)?);
            api::taken(on, now.sqm).map_err(Failure::Refused)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_rows_ask_names_the_act() {
        assert_eq!(
            Act::asked(&json!({"ask": "act", "act": "speedtest"})),
            Ok(Act::SpeedTest)
        );
        assert_eq!(
            Act::asked(&json!({"ask": "act", "act": "sqm", "on": false})),
            Ok(Act::Conferencing(false))
        );
    }

    #[test]
    fn an_ask_eero_has_no_act_for_is_refused() {
        assert_eq!(
            Act::asked(&json!({"ask": "act", "act": "sqm"})),
            Err("conferencing and gaming is asked on or off".to_owned())
        );
        assert_eq!(
            Act::asked(&json!({"ask": "act", "act": "reboot"})),
            Err("eero does nothing called reboot".to_owned())
        );
    }
}
