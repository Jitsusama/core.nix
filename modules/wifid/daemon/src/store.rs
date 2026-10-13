//! Where saved networks are kept. Nix declares some, in networkd's own
//! directory under /etc, which wifid only reads. The launcher saves the rest
//! in wifid's state directory, each one only what's its own, and wifid
//! writes networkd's file for each in /run, laid over the Wi-Fi defaults, so
//! a change to the defaults reaches every saved network.

use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::network::Network;
use crate::syntax::Document;

pub struct Store {
    /// Where Nix declares networks: /etc/systemd/network.
    pub declared: PathBuf,
    /// Where the launcher's networks are kept: /var/lib/wifid/networks.
    pub saved: PathBuf,
    /// Where networkd reads what wifid writes: /run/systemd/network.
    pub runtime: PathBuf,
    /// The file every Wi-Fi network starts from: 90-wifi.network.
    pub defaults: PathBuf,
}

/// A saved network, and where it's kept.
#[derive(Debug, PartialEq)]
pub struct Saved {
    pub network: Network,
    pub file: PathBuf,
    /// Whether Nix declares it, so that only a rebuild changes it.
    pub declared: bool,
}

#[derive(Debug)]
pub enum Failure {
    /// Nix declares the network in this file, so only a rebuild changes it.
    Declared(PathBuf),
    NotSaved,
    Io(io::Error),
}

impl From<io::Error> for Failure {
    fn from(error: io::Error) -> Failure {
        Failure::Io(error)
    }
}

/// How every file wifid gives networkd starts, which is how it knows its
/// own from anyone else's in /run.
const WRITTEN: &str = "# Written by wifid from";

/// What the store holds.
#[derive(Debug, Default, PartialEq)]
pub struct Listing {
    pub saved: Vec<Saved>,
    /// Files that claim to be a saved network but can't be joined, and why.
    pub unreadable: Vec<(PathBuf, String)>,
}

impl Store {
    /// Every saved network: Nix's, then the launcher's, each in the order
    /// of their files.
    pub fn networks(&self) -> io::Result<Listing> {
        let mut listing = Listing::default();
        for (directory, declared) in [(&self.declared, true), (&self.saved, false)] {
            for file in network_files(directory)? {
                let network = fs::read_to_string(&file)
                    .map_err(|error| error.to_string())
                    .and_then(|text| Document::parse(&text).map_err(|m| m.to_string()))
                    .and_then(|document| Network::read(&document).map_err(|u| u.0));
                match network {
                    Ok(Some(network)) => listing.saved.push(Saved {
                        network,
                        file,
                        declared,
                    }),
                    Ok(None) => {}
                    Err(why) => listing.unreadable.push((file, why)),
                }
            }
        }
        Ok(listing)
    }

    /// Keeps the network, or what's changed about it, and gives networkd
    /// its file.
    pub fn save(&self, network: &Network) -> Result<(), Failure> {
        let (file, mut own) = match self.find(&network.name)? {
            Some(saved) if saved.declared => return Err(Failure::Declared(saved.file)),
            Some(saved) => {
                let own = read(&saved.file)?;
                (saved.file, own)
            }
            None => (self.new_file(&network.name), Document::default()),
        };
        network.write_into(&mut own);
        replace(&file, &own.render())?;
        Ok(self.give_networkd(&file, &own)?)
    }

    /// Gives networkd the file for every launcher network as it stands,
    /// as wifid does when it starts, so the defaults changing or a file
    /// edited by hand reach networkd, and takes away the files it wrote
    /// for networks that have gone since.
    pub fn give_networkd_all(&self) -> io::Result<()> {
        let launchers: Vec<PathBuf> = self
            .networks()?
            .saved
            .into_iter()
            .filter(|saved| !saved.declared)
            .map(|saved| saved.file)
            .collect();
        let given: Vec<PathBuf> = launchers
            .iter()
            .map(|f| self.given_to_networkd(f))
            .collect();
        for file in network_files(&self.runtime)? {
            // A file that isn't text isn't one wifid wrote.
            let ours = fs::read_to_string(&file).is_ok_and(|text| text.starts_with(WRITTEN));
            if ours && !given.contains(&file) {
                fs::remove_file(file)?;
            }
        }
        for file in launchers {
            self.give_networkd(&file, &read(&file)?)?;
        }
        Ok(())
    }

    /// Takes away a network the launcher saved, handing it back so that
    /// whatever else it had, such as its secret, can go too.
    pub fn forget(&self, name: &str) -> Result<Network, Failure> {
        let saved = self.find(name)?.ok_or(Failure::NotSaved)?;
        if saved.declared {
            return Err(Failure::Declared(saved.file));
        }
        fs::remove_file(self.given_to_networkd(&saved.file))?;
        fs::remove_file(&saved.file)?;
        Ok(saved.network)
    }

    /// The network of that name. Nix's comes first, so when both have one,
    /// Nix's is the one joined and the launcher can't change it.
    fn find(&self, name: &str) -> io::Result<Option<Saved>> {
        let listing = self.networks()?;
        Ok(listing
            .saved
            .into_iter()
            .find(|saved| saved.network.name == name))
    }

    /// A file for a network not saved before, named for it, and numbered
    /// when another network's name makes the same file name. Its file for
    /// networkd mustn't share a name with one of Nix's either, since
    /// networkd would read Nix's in its place.
    fn new_file(&self, name: &str) -> PathBuf {
        let slug = slug(name);
        let mut file = self.saved.join(format!("{slug}.network"));
        for n in 2.. {
            let given = self.given_to_networkd(&file);
            let nix = self.declared.join(given.file_name().unwrap_or_default());
            if !file.exists() && !nix.exists() {
                break;
            }
            file = self.saved.join(format!("{slug}-{n}.network"));
        }
        file
    }

    /// The file networkd reads for a network the launcher saved.
    fn given_to_networkd(&self, file: &Path) -> PathBuf {
        let stem = file.file_stem().unwrap_or_default().to_string_lossy();
        self.runtime.join(format!("50-wifi-{stem}.network"))
    }

    fn give_networkd(&self, file: &Path, own: &Document) -> io::Result<()> {
        let header = format!(
            "{WRITTEN} {}, laid over {}.\n\
             # Change that file, not this one: wifid writes this one again.\n",
            file.display(),
            self.defaults.display(),
        );
        let laid = own.laid_over(&read(&self.defaults)?);
        replace(&self.given_to_networkd(file), &(header + &laid.render()))
    }
}

fn network_files(directory: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = fs::read_dir(directory)?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<io::Result<_>>()?;
    files.retain(|file| file.extension().is_some_and(|e| e == "network"));
    files.sort();
    Ok(files)
}

fn read(file: &Path) -> io::Result<Document> {
    Document::parse(&fs::read_to_string(file)?).map_err(|malformed| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{}, {malformed}", file.display()),
        )
    })
}

/// Writes the file whole or not at all, so networkd never reads half of one.
/// networkd doesn't run as root, so the file is made readable by everyone
/// whatever wifid's umask, which keeps everything else it writes to itself.
/// A file that already says this, readable, is left as it is, so networkd
/// doesn't take an unchanged file for a new one.
fn replace(file: &Path, text: &str) -> io::Result<()> {
    let readable = |file| fs::metadata(file).is_ok_and(|m| m.permissions().mode() & 0o777 == 0o644);
    if readable(file) && fs::read(file).is_ok_and(|held| held == text.as_bytes()) {
        return Ok(());
    }
    let name = file.file_name().unwrap_or_default().to_string_lossy();
    let partial = file.with_file_name(format!(".{name}.partial"));
    fs::write(&partial, text)?;
    fs::set_permissions(&partial, fs::Permissions::from_mode(0o644))?;
    fs::rename(&partial, file)
}

/// A file name for a network: its name in lower case, with a dash for
/// anything that isn't a letter or a digit.
fn slug(name: &str) -> String {
    let mut slug = String::new();
    for c in name.chars().flat_map(char::to_lowercase) {
        if c.is_alphanumeric() {
            slug.push(c);
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    slug.truncate(slug.trim_end_matches('-').len());
    if slug.is_empty() {
        slug.push_str("network");
    }
    slug
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::MetadataExt;

    use tempfile::TempDir;

    use super::*;
    use crate::network::{Address, Security};

    const DEFAULTS: &str = "[Match]\nType=wlan\n\n[Network]\nDHCP=yes\nMulticastDNS=no\n";

    /// A store in a directory of its own, with the defaults in place.
    fn store() -> (TempDir, Store) {
        let root = TempDir::new().unwrap();
        let store = Store {
            declared: root.path().join("etc"),
            saved: root.path().join("var"),
            runtime: root.path().join("run"),
            defaults: root.path().join("etc/90-wifi.network"),
        };
        for directory in [&store.declared, &store.saved, &store.runtime] {
            fs::create_dir(directory).unwrap();
        }
        fs::write(&store.defaults, DEFAULTS).unwrap();
        (root, store)
    }

    fn lantern() -> Network {
        Network {
            name: "Lantern".to_string(),
            security: Security::Personal,
            secret: Some(PathBuf::from("/var/lib/wifid/secrets/lantern")),
            priority: 0,
            hidden: false,
            address: Address::Network,
            trusted: true,
        }
    }

    #[test]
    fn a_network_saved_from_the_launcher_is_kept_and_listed() {
        let (_root, store) = store();

        store.save(&lantern()).unwrap();

        assert_eq!(
            store.networks().unwrap().saved,
            vec![Saved {
                network: lantern(),
                file: store.saved.join("lantern.network"),
                declared: false,
            }]
        );
    }

    #[test]
    fn a_network_nix_declares_is_listed_as_declared() {
        let (_root, store) = store();
        let inn = store.declared.join("50-wifi-roadside-inn.network");
        fs::write(
            &inn,
            "[Match]\nSSID=\"Roadside Inn\"\n\n[X-WiFi]\nSecurity=open\n",
        )
        .unwrap();
        fs::write(
            store.declared.join("40-eth0.network"),
            "[Match]\nName=eth0\n",
        )
        .unwrap();

        let networks = store.networks().unwrap().saved;

        assert_eq!(networks.len(), 1);
        assert_eq!(networks[0].network.name, "Roadside Inn");
        assert_eq!(networks[0].file, inn);
        assert!(networks[0].declared);
    }

    #[test]
    fn saving_a_network_again_keeps_what_a_person_added_to_its_file() {
        let (_root, store) = store();
        let home = store.saved.join("home.network");
        fs::write(
            &home,
            "# Mine.\n[Match]\nSSID=Lantern\n\n[Network]\nDNS=10.42.0.53\n\n[X-WiFi]\n",
        )
        .unwrap();

        store.save(&lantern()).unwrap();

        let kept = fs::read_to_string(&home).unwrap();
        assert!(kept.starts_with("# Mine.\n"), "{kept}");
        assert!(
            kept.contains("DNS=10.42.0.53\nMulticastDNS=yes\n"),
            "{kept}"
        );
        assert!(!store.saved.join("lantern.network").exists());
    }

    #[test]
    fn networks_whose_names_make_the_same_file_name_each_get_their_own() {
        let (_root, store) = store();
        let mut cafe = lantern();
        cafe.name = "Café Wi-Fi".to_string();
        let mut other = lantern();
        other.name = "café wi fi".to_string();

        store.save(&cafe).unwrap();
        store.save(&other).unwrap();

        let files: Vec<PathBuf> = store
            .networks()
            .unwrap()
            .saved
            .into_iter()
            .map(|s| s.file)
            .collect();
        assert_eq!(
            files,
            vec![
                store.saved.join("café-wi-fi-2.network"),
                store.saved.join("café-wi-fi.network")
            ]
        );
        assert!(store.runtime.join("50-wifi-café-wi-fi-2.network").exists());
    }

    #[test]
    fn a_network_nix_declares_cant_be_saved_over() {
        let (_root, store) = store();
        let declared = store.declared.join("50-wifi-lantern.network");
        fs::write(&declared, "[Match]\nSSID=Lantern\n\n[X-WiFi]\n").unwrap();

        let refused = store.save(&lantern());

        assert!(matches!(refused, Err(Failure::Declared(file)) if file == declared));
        assert!(!store.saved.join("lantern.network").exists());
    }

    #[test]
    fn starting_up_gives_networkd_every_saved_network_as_its_file_stands() {
        let (_root, store) = store();
        fs::write(
            store.saved.join("home.network"),
            "[Match]\nSSID=Home\n\n[X-WiFi]\n",
        )
        .unwrap();
        store.save(&lantern()).unwrap();
        fs::remove_file(store.saved.join("lantern.network")).unwrap();
        fs::write(
            store.runtime.join("50-wifi-vpn.network"),
            "[Match]\nName=tun0\n",
        )
        .unwrap();

        store.give_networkd_all().unwrap();

        let mut given: Vec<String> = fs::read_dir(&store.runtime)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        given.sort();
        assert_eq!(given, vec!["50-wifi-home.network", "50-wifi-vpn.network"]);
    }

    #[test]
    fn a_file_networkd_already_has_as_written_is_left_as_it_is() {
        let (_root, store) = store();
        store.save(&lantern()).unwrap();
        let given = store.runtime.join("50-wifi-lantern.network");
        let before = fs::metadata(&given).unwrap().ino();

        store.give_networkd_all().unwrap();

        assert_eq!(fs::metadata(&given).unwrap().ino(), before);
    }

    #[test]
    fn a_file_in_networkd_s_directory_that_isnt_text_is_left_alone() {
        let (_root, store) = store();
        store.save(&lantern()).unwrap();
        let foreign = store.runtime.join("60-other.network");
        fs::write(&foreign, [0xff, 0xfe]).unwrap();

        store.give_networkd_all().unwrap();

        assert!(foreign.exists());
        assert!(store.runtime.join("50-wifi-lantern.network").exists());
    }

    #[test]
    fn a_network_forgotten_is_gone_from_both_places() {
        let (_root, store) = store();
        store.save(&lantern()).unwrap();

        let forgotten = store.forget("Lantern").unwrap();

        assert_eq!(forgotten, lantern());
        assert_eq!(store.networks().unwrap().saved, vec![]);
        assert!(!store.runtime.join("50-wifi-lantern.network").exists());
    }

    #[test]
    fn a_network_nix_declares_cant_be_forgotten() {
        let (_root, store) = store();
        let inn = store.declared.join("50-wifi-roadside-inn.network");
        fs::write(&inn, "[Match]\nSSID=\"Roadside Inn\"\n\n[X-WiFi]\n").unwrap();

        let refused = store.forget("Roadside Inn");

        assert!(matches!(refused, Err(Failure::Declared(file)) if file == inn));
        assert!(inn.exists());
    }

    #[test]
    fn a_network_never_saved_cant_be_forgotten() {
        let (_root, store) = store();

        assert!(matches!(store.forget("Lantern"), Err(Failure::NotSaved)));
    }

    #[test]
    fn a_file_that_cant_be_read_is_named_with_the_reason() {
        let (_root, store) = store();
        let hilton = store.saved.join("hilton.network");
        fs::write(&hilton, "[Match]\nSSID=Hilton*\n\n[X-WiFi]\n").unwrap();
        let broken = store.saved.join("broken.network");
        fs::write(&broken, "[Match]\nSSID\n").unwrap();

        let listing = store.networks().unwrap();

        assert_eq!(listing.saved, vec![]);
        assert_eq!(
            listing.unreadable,
            vec![
                (broken, "line 2: a line with no =".to_string()),
                (
                    hilton,
                    "SSID=Hilton* is a pattern, not one network's name".to_string()
                ),
            ]
        );
    }

    #[test]
    fn a_file_that_isnt_text_is_named_and_the_rest_still_listed() {
        let (_root, store) = store();
        let garbled = store.saved.join("garbled.network");
        fs::write(&garbled, [b'[', 0xff, b']']).unwrap();
        store.save(&lantern()).unwrap();

        let listing = store.networks().unwrap();

        let names: Vec<&str> = listing
            .saved
            .iter()
            .map(|s| s.network.name.as_str())
            .collect();
        assert_eq!(names, vec!["Lantern"]);
        assert_eq!(
            listing.unreadable,
            vec![(garbled, "stream did not contain valid UTF-8".to_string())]
        );
    }

    #[test]
    fn networkd_is_given_a_saved_network_laid_over_the_defaults() {
        let (_root, store) = store();

        store.save(&lantern()).unwrap();

        let written = fs::read_to_string(store.runtime.join("50-wifi-lantern.network")).unwrap();
        assert_eq!(
            written,
            format!(
                concat!(
                    "# Written by wifid from {saved}, laid over {defaults}.\n",
                    "# Change that file, not this one: wifid writes this one again.\n",
                    "[Match]\nType=wlan\nSSID=\"Lantern\"\n\n",
                    "[Network]\nDHCP=yes\nMulticastDNS=yes\n\n",
                    "[X-WiFi]\nSecurity=personal\nSecret=/var/lib/wifid/secrets/lantern\n",
                    "Priority=0\nHidden=no\nAddress=network\n",
                ),
                saved = store.saved.join("lantern.network").display(),
                defaults = store.defaults.display(),
            )
        );
    }
}
