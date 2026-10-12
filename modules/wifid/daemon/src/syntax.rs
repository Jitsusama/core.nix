//! The format systemd's own files are written in, systemd.syntax(7): sections
//! of settings, one `Key=value` to a line. networkd's .network files are in
//! it, and so is wifid.conf.

/// A file in systemd's format, kept line by line so that writing it back
/// changes nothing a person put there, comments included.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Document {
    /// What comes before the first section: a comment saying what the file
    /// is for, usually.
    head: Vec<Line>,
    sections: Vec<Section>,
}

#[derive(Clone, Debug, PartialEq)]
struct Section {
    name: String,
    lines: Vec<Line>,
}

impl Section {
    /// The keys set here, each once, in the order they're first set.
    fn keys(&self) -> Vec<&str> {
        let mut keys: Vec<&str> = Vec::new();
        for line in &self.lines {
            if let Line::Setting { key, .. } = line
                && !keys.contains(&key.as_str())
            {
                keys.push(key);
            }
        }
        keys
    }

    fn settings_of(&self, key: &str) -> Vec<Line> {
        self.lines.iter().filter(|l| l.sets(key)).cloned().collect()
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Line {
    Setting {
        key: String,
        value: String,
    },
    /// A comment or a blank line, kept as it was.
    Other(String),
}

impl Line {
    fn sets(&self, key: &str) -> bool {
        matches!(self, Line::Setting { key: k, .. } if k == key)
    }
}

#[derive(Debug, PartialEq)]
pub struct Malformed {
    pub line: usize,
    pub reason: &'static str,
}

impl std::fmt::Display for Malformed {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.reason)
    }
}

impl Document {
    pub fn parse(text: &str) -> Result<Document, Malformed> {
        let mut document = Document::default();
        let mut continued = String::new();
        for (index, line) in text.lines().enumerate() {
            // systemd turns a line's closing backslash into a space and reads
            // the next line on from there.
            let comment = continued.is_empty() && line.trim_start().starts_with(['#', ';']);
            if let Some(start) = line.trim_end().strip_suffix('\\')
                && !comment
            {
                continued.push_str(start.trim_start());
                continued.push(' ');
                continue;
            }
            let joined = std::mem::take(&mut continued) + line.trim_start();
            let line = joined.trim();
            if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                document.sections.push(Section {
                    name: name.to_string(),
                    lines: Vec::new(),
                });
                continue;
            }
            let section = document.sections.last_mut();
            if line.is_empty() || line.starts_with(['#', ';']) {
                let lines = section.map_or(&mut document.head, |s| &mut s.lines);
                lines.push(Line::Other(line.to_string()));
                continue;
            }
            let malformed = |reason| Malformed {
                line: index + 1,
                reason,
            };
            let section = section.ok_or(malformed("a setting outside a section"))?;
            let (key, value) = line.split_once('=').ok_or(malformed("a line with no ="))?;
            section.lines.push(Line::Setting {
                key: key.trim_end().to_string(),
                value: value.trim_start().to_string(),
            });
        }
        Ok(document)
    }

    pub fn has(&self, section: &str) -> bool {
        self.sections.iter().any(|s| s.name == section)
    }

    /// A setting's value, as systemd reads it: the last time the key is set
    /// in any section of that name.
    pub fn value(&self, section: &str, key: &str) -> Option<&str> {
        self.sections
            .iter()
            .filter(|s| s.name == section)
            .flat_map(|s| &s.lines)
            .filter_map(|line| match line {
                Line::Setting { key: k, value } if k == key => Some(value.as_str()),
                _ => None,
            })
            .next_back()
    }

    /// Gives a key this one value. A key already set keeps its place, and
    /// any later settings of it go, so the file reads as it behaves.
    pub fn set(&mut self, section: &str, key: &str, value: &str) {
        let setting = Line::Setting {
            key: key.to_string(),
            value: value.to_string(),
        };
        self.put(section, key, vec![setting]);
    }

    /// Takes every setting of the key out of the section.
    pub fn remove(&mut self, section: &str, key: &str) {
        self.replace(section, key, &[]);
    }

    /// This file laid over the defaults, as wifid writes a saved network's
    /// file for networkd: each key it sets takes the place of the defaults'
    /// settings of that key, every value of it kept, and a section the
    /// defaults don't have is added whole, so `[Route]` can repeat.
    pub fn laid_over(&self, defaults: &Document) -> Document {
        let mut file = defaults.clone();
        for section in &self.sections {
            if !defaults.sections.iter().any(|s| s.name == section.name) {
                file.add_section(section.clone());
                continue;
            }
            for key in section.keys() {
                file.put(&section.name, key, section.settings_of(key));
            }
        }
        file
    }

    /// Gives a key these settings in place of any it had.
    fn put(&mut self, section: &str, key: &str, settings: Vec<Line>) {
        if !self.replace(section, key, &settings) && !self.add_to(section, &settings) {
            self.add_section(Section {
                name: section.to_string(),
                lines: settings,
            });
        }
    }

    /// Puts the settings where the key is first set, and drops the rest.
    fn replace(&mut self, section: &str, key: &str, settings: &[Line]) -> bool {
        let mut placed = false;
        for section in self.sections.iter_mut().filter(|s| s.name == section) {
            for line in std::mem::take(&mut section.lines) {
                if !line.sets(key) {
                    section.lines.push(line);
                } else if !placed {
                    section.lines.extend_from_slice(settings);
                    placed = true;
                }
            }
        }
        placed
    }

    /// Adds the settings after the last one in the section, if there is one.
    fn add_to(&mut self, section: &str, settings: &[Line]) -> bool {
        let Some(section) = self.sections.iter_mut().rfind(|s| s.name == section) else {
            return false;
        };
        let after = section
            .lines
            .iter()
            .rposition(|l| matches!(l, Line::Setting { .. }))
            .map_or(0, |at| at + 1);
        section.lines.splice(after..after, settings.iter().cloned());
        true
    }

    /// Adds the section at the end, a blank line apart from what's above,
    /// and with none after it.
    fn add_section(&mut self, mut section: Section) {
        while section.lines.last() == Some(&Line::Other(String::new())) {
            section.lines.pop();
        }
        let above = self
            .sections
            .last_mut()
            .map_or(&mut self.head, |s| &mut s.lines);
        if above
            .last()
            .is_some_and(|line| *line != Line::Other(String::new()))
        {
            above.push(Line::Other(String::new()));
        }
        self.sections.push(section);
    }

    pub fn render(&self) -> String {
        let mut text = String::new();
        let mut write = |lines: &[Line]| {
            for line in lines {
                match line {
                    Line::Setting { key, value } => text.push_str(&format!("{key}={value}\n")),
                    Line::Other(line) => text.push_str(&format!("{line}\n")),
                }
            }
        };
        write(&self.head);
        for section in &self.sections {
            write(&[Line::Other(format!("[{}]", section.name))]);
            write(&section.lines);
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_reads_back_as_it_was_written() {
        let text = "[Match]\nType=wlan\n\n[Network]\nDHCP=yes\nLLMNR=no\n";

        assert_eq!(Document::parse(text).unwrap().render(), text);
    }

    #[test]
    fn comments_are_kept() {
        let text = "# The hotel's network.\n[Match]\n; by name, not C:\\\nSSID=Hilton\n";

        assert_eq!(Document::parse(text).unwrap().render(), text);
    }

    #[test]
    fn the_last_setting_of_a_key_is_its_value() {
        let text = "[Network]\nDHCP=no\n[X-WiFi]\nDHCP=maybe\n[Network]\nDHCP=yes\n";

        let document = Document::parse(text).unwrap();

        assert_eq!(document.value("Network", "DHCP"), Some("yes"));
        assert_eq!(document.value("Network", "LLMNR"), None);
    }

    #[test]
    fn a_key_set_again_keeps_its_place() {
        let mut document = Document::parse("[Network]\nMulticastDNS=no\nLLMNR=no\n").unwrap();

        document.set("Network", "MulticastDNS", "yes");

        assert_eq!(document.render(), "[Network]\nMulticastDNS=yes\nLLMNR=no\n");
    }

    #[test]
    fn a_key_set_once_overrides_every_earlier_setting() {
        let mut document = Document::parse("[Network]\nDNS=10.0.0.1\nDNS=10.0.0.2\n").unwrap();

        document.set("Network", "DNS", "10.0.0.3");

        assert_eq!(document.render(), "[Network]\nDNS=10.0.0.3\n");
    }

    #[test]
    fn a_new_key_follows_its_section_s_last_setting() {
        let mut document = Document::parse("[Network]\nDHCP=yes\n\n[DHCPv4]\n").unwrap();

        document.set("Network", "LLMNR", "no");

        assert_eq!(
            document.render(),
            "[Network]\nDHCP=yes\nLLMNR=no\n\n[DHCPv4]\n"
        );
    }

    #[test]
    fn a_new_section_goes_at_the_end() {
        let mut document = Document::parse("[Match]\nSSID=Lantern\n").unwrap();

        document.set("X-WiFi", "Security", "wpa-psk");

        assert_eq!(
            document.render(),
            "[Match]\nSSID=Lantern\n\n[X-WiFi]\nSecurity=wpa-psk\n"
        );
    }

    #[test]
    fn a_file_laid_over_the_defaults_takes_their_place_key_by_key() {
        let defaults = Document::parse(concat!(
            "[Match]\nType=wlan\n\n",
            "[Network]\nDHCP=yes\nDNS=10.0.0.1\nMulticastDNS=no\n",
        ))
        .unwrap();
        let own = Document::parse(concat!(
            "# Joined from the launcher.\n",
            "[Match]\nSSID=Lantern\n\n",
            "[Network]\nDNS=10.42.0.1\nDNS=10.42.0.2\nMulticastDNS=yes\n\n",
            "[Route]\nGateway=10.42.0.1\n\n",
            "[Route]\nGateway=10.42.0.2\n",
        ))
        .unwrap();

        let file = own.laid_over(&defaults);

        assert_eq!(
            file.render(),
            concat!(
                "[Match]\nType=wlan\nSSID=Lantern\n\n",
                "[Network]\nDHCP=yes\nDNS=10.42.0.1\nDNS=10.42.0.2\nMulticastDNS=yes\n\n",
                "[Route]\nGateway=10.42.0.1\n\n",
                "[Route]\nGateway=10.42.0.2\n",
            )
        );
    }

    #[test]
    fn a_backslash_carries_a_setting_on_to_the_next_line() {
        let text = "[Network]\nDNS=10.0.0.1 \\\n    10.0.0.2\nLLMNR=no\n";

        let document = Document::parse(text).unwrap();

        assert_eq!(document.value("Network", "DNS"), Some("10.0.0.1  10.0.0.2"));
        assert_eq!(document.value("Network", "LLMNR"), Some("no"));
    }
}
