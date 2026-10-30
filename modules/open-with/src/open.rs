//! What a program has open now, for its Recent in the launcher to lead with:
//! each of its processes and the whole path of the file it shows. A zathura
//! says it over the session bus, as `org.pwmt.zathura.PID-<pid>`, whose
//! `filename` is the document it has open, empty while it has none.

use zbus::Connection;
use zbus::fdo::DBusProxy;
use zbus::proxy::{Builder, CacheProperties};

/// The name each zathura takes on the session bus, its process id after it.
pub const ZATHURA: &str = "org.pwmt.zathura.PID-";

/// The process a bus name is for, when it's one `prefix` names.
#[must_use]
pub fn pid_of(name: &str, prefix: &str) -> Option<u32> {
    name.strip_prefix(prefix)?.parse().ok()
}

/// Every zathura on the bus with a document open, by its process and the
/// document's path. One quitting as it's asked is passed over, so what's
/// left is still said.
///
/// # Errors
///
/// When the bus can't list its names.
pub async fn zathura(connection: &Connection) -> zbus::Result<Vec<(u32, String)>> {
    let names = DBusProxy::new(connection).await?.list_names().await?;
    let mut open = Vec::new();
    for name in names {
        let Some(pid) = pid_of(name.as_str(), ZATHURA) else {
            continue;
        };
        let Ok(asked) = Builder::<zbus::Proxy>::new(connection)
            .destination(name.as_str())?
            .path("/org/pwmt/zathura")?
            .interface("org.pwmt.zathura")?
            .cache_properties(CacheProperties::No)
            .build()
            .await
        else {
            continue;
        };
        if let Ok(file) = asked.get_property::<String>("filename").await
            && !file.is_empty()
        {
            open.push((pid, file));
        }
    }
    Ok(open)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_zathura_is_known_by_its_process() {
        assert_eq!(pid_of("org.pwmt.zathura.PID-4242", ZATHURA), Some(4242));
    }

    #[test]
    fn another_name_is_no_zathura() {
        assert_eq!(pid_of("org.freedesktop.Notifications", ZATHURA), None);
        assert_eq!(pid_of("org.pwmt.zathura.PID-", ZATHURA), None);
        assert_eq!(pid_of("org.pwmt.zathura.PID-12x", ZATHURA), None);
    }
}
