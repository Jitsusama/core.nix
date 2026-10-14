//! networkd and resolved, on the system bus: what they say as the network
//! changes, and what they say it is.

use std::collections::BTreeSet;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use futures_util::{Stream, StreamExt, stream};
use zbus::fdo::DBusProxy;
use zbus::proxy::CacheProperties;
use zbus::{Connection, MatchRule, MessageStream, proxy};

use crate::reach::Reach;

const NETWORKD: &str = "org.freedesktop.network1";
const RESOLVED: &str = "org.freedesktop.resolve1";

#[proxy(
    interface = "org.freedesktop.resolve1.Manager",
    default_service = "org.freedesktop.resolve1",
    default_path = "/org/freedesktop/resolve1"
)]
trait Resolved {
    /// Every server it asks: the link's index, 0 for none, the address's
    /// family and the address.
    #[zbus(property, name = "DNS")]
    fn dns(&self) -> zbus::Result<Vec<(i32, i32, Vec<u8>)>>;
}

/// Something changed, or networkd or resolved came or went. Nothing is read
/// from it: what changed is looked at once it settles.
pub async fn changes(bus: &Connection) -> zbus::Result<impl Stream<Item = ()> + Unpin> {
    // networkd's manager and every link say each change of their state.
    // resolved says its servers changed whenever networkd writes anything
    // of a link, and whenever a program such as a VPN's sets a link's
    // servers itself.
    let said = [
        (NETWORKD, "/org/freedesktop/network1"),
        (RESOLVED, "/org/freedesktop/resolve1"),
    ];
    let mut heard = Vec::new();
    for (sender, namespace) in said {
        let rule = MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .sender(sender)?
            .interface("org.freedesktop.DBus.Properties")?
            .member("PropertiesChanged")?
            .path_namespace(namespace)?
            .build();
        heard.push(
            MessageStream::for_match_rule(rule, bus, None)
                .await?
                .map(drop)
                .boxed(),
        );
        let owners = DBusProxy::new(bus).await?;
        let owners = owners
            .receive_name_owner_changed_with_args(&[(0, sender)])
            .await?;
        heard.push(owners.map(drop).boxed());
    }
    Ok(stream::select_all(heard))
}

/// What the machine reaches, as they say it is now; nothing while networkd
/// isn't answering, since then nothing is known.
pub async fn reach(bus: &Connection) -> Option<Reach> {
    let described = bus
        .call_method(
            Some(NETWORKD),
            "/org/freedesktop/network1",
            Some("org.freedesktop.network1.Manager"),
            "Describe",
            &(),
        )
        .await
        .ok()?;
    let text: String = described.body().deserialize().ok()?;
    let description = serde_json::from_str(&text).ok()?;
    Some(Reach::read(&description, servers(bus).await))
}

/// The servers resolved asks, each as its address and link; none when
/// resolved isn't answering, as then nothing resolves.
async fn servers(bus: &Connection) -> BTreeSet<String> {
    let servers = async {
        let resolved = ResolvedProxy::builder(bus).cache_properties(CacheProperties::No);
        resolved.build().await?.dns().await
    };
    let Ok(servers) = servers.await else {
        return BTreeSet::new();
    };
    servers
        .into_iter()
        .filter_map(|(link, _, address)| Some(format!("{}%{link}", ip(&address)?)))
        .collect()
}

fn ip(bytes: &[u8]) -> Option<IpAddr> {
    if let Ok(octets) = <[u8; 4]>::try_from(bytes) {
        return Some(Ipv4Addr::from(octets).into());
    }
    let octets = <[u8; 16]>::try_from(bytes).ok()?;
    Some(Ipv6Addr::from(octets).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_address_is_read_by_its_length() {
        assert_eq!(ip(&[10, 0, 0, 53]), Some("10.0.0.53".parse().unwrap()));
        let mut six = [0; 16];
        six[15] = 1;
        assert_eq!(ip(&six), Some("::1".parse().unwrap()));
        assert_eq!(ip(&[1, 2, 3]), None);
    }
}
