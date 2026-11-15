//! Enough of XML to keep what a device answers in its own words: `UPnP`'s
//! descriptions and SOAP replies, read into JSON so they sit under a
//! device's `detail` beside eero's and the modem's.
//!
//! An element with only text is its text. One with attributes or elements
//! inside is an object of them, by name without the namespace's prefix; a
//! name said twice is a list; and text beside elements is kept as `text`.
//! Namespace declarations say nothing about the device, so they're left out.

use serde_json::{Map, Value};

/// The outermost element, by its name, or `None` when the text isn't XML
/// this can read.
pub fn read(text: &str) -> Option<(String, Value)> {
    let mut rest = text;
    let mut open: Vec<Element> = Vec::new();
    loop {
        let at = rest.find('<')?;
        if let Some(element) = open.last_mut() {
            element.text.push_str(&rest[..at]);
        } else if !rest[..at].trim().is_empty() {
            return None;
        }
        rest = &rest[at..];
        if let Some(after) = rest.strip_prefix("<!--") {
            rest = &after[after.find("-->")? + 3..];
        } else if let Some(after) = rest.strip_prefix("<![CDATA[") {
            let end = after.find("]]>")?;
            open.last_mut()?.cdata.push_str(&after[..end]);
            rest = &after[end + 3..];
        } else if rest.starts_with("<?") || rest.starts_with("<!") {
            rest = &rest[rest.find('>')? + 1..];
        } else if let Some(after) = rest.strip_prefix("</") {
            let end = after.find('>')?;
            let element = open.pop()?;
            if after[..end].trim() != element.name {
                return None;
            }
            rest = &after[end + 1..];
            let (name, value) = element.close();
            match open.last_mut() {
                Some(parent) => put(&mut parent.fields, name, value),
                None => return Some((name, value)),
            }
        } else {
            let end = rest.find('>')?;
            let tag = &rest[1..end];
            rest = &rest[end + 1..];
            let (tag, empty) = match tag.strip_suffix('/') {
                Some(tag) => (tag, true),
                None => (tag, false),
            };
            let element = Element::open(tag)?;
            if empty {
                let (name, value) = element.close();
                match open.last_mut() {
                    Some(parent) => put(&mut parent.fields, name, value),
                    None => return Some((name, value)),
                }
            } else {
                open.push(element);
            }
        }
    }
}

/// The first element named `name` anywhere inside `value`, depth first.
pub fn find<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
    match value {
        Value::Object(fields) => fields
            .get(name)
            .or_else(|| fields.values().find_map(|inside| find(inside, name))),
        Value::Array(items) => items.iter().find_map(|item| find(item, name)),
        _ => None,
    }
}

/// An element being read: its name as written, so its end can be matched,
/// and what it holds so far.
struct Element {
    name: String,
    fields: Map<String, Value>,
    text: String,
    cdata: String,
}

impl Element {
    /// An element from its start tag's inside: its name, then attributes.
    fn open(tag: &str) -> Option<Self> {
        let tag = tag.trim();
        let name_ends = tag.find(char::is_whitespace).unwrap_or(tag.len());
        let name = &tag[..name_ends];
        if name.is_empty() || name.contains(['<', '"', '\'', '=']) {
            return None;
        }
        let mut fields = Map::new();
        let mut rest = tag[name_ends..].trim_start();
        while !rest.is_empty() {
            let equals = rest.find('=')?;
            let key = rest[..equals].trim();
            let after = rest[equals + 1..].trim_start();
            let quote = after
                .chars()
                .next()
                .filter(|quote| matches!(quote, '"' | '\''))?;
            let end = after[1..].find(quote)? + 1;
            if key != "xmlns" && !key.starts_with("xmlns:") {
                put(
                    &mut fields,
                    local(key).to_owned(),
                    Value::String(unescape(&after[1..end])),
                );
            }
            rest = after[end + 1..].trim_start();
        }
        Some(Self {
            name: name.to_owned(),
            fields,
            text: String::new(),
            cdata: String::new(),
        })
    }

    /// The element as a value, by its name without its prefix.
    fn close(self) -> (String, Value) {
        let text = unescape(self.text.trim()) + &self.cdata;
        let name = local(&self.name).to_owned();
        if self.fields.is_empty() {
            return (name, Value::String(text));
        }
        let mut fields = self.fields;
        if !text.is_empty() {
            put(&mut fields, "text".into(), Value::String(text));
        }
        (name, Value::Object(fields))
    }
}

/// Text as written, its five named escapes and numbered characters read.
fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let Some(end) = rest.find(';') else {
            break;
        };
        let read = match &rest[1..end] {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            code => code
                .strip_prefix("#x")
                .map(|hex| u32::from_str_radix(hex, 16))
                .or_else(|| code.strip_prefix('#').map(str::parse))
                .and_then(Result::ok)
                .and_then(char::from_u32),
        };
        if let Some(character) = read {
            out.push(character);
            rest = &rest[end + 1..];
        } else {
            out.push('&');
            rest = &rest[1..];
        }
    }
    out.push_str(rest);
    out
}

/// A name without its namespace's prefix: `CurrentMute` of `u:CurrentMute`.
fn local(name: &str) -> &str {
    name.rsplit_once(':').map_or(name, |(_, local)| local)
}

/// Puts a value under its name, a second under the same name making a list.
fn put(fields: &mut Map<String, Value>, name: String, value: Value) {
    match fields.get_mut(&name) {
        Some(Value::Array(items)) => items.push(value),
        Some(first) => {
            let first = first.take();
            fields.insert(name, Value::Array(vec![first, value]));
        }
        None => {
            fields.insert(name, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::json;

    #[test]
    fn an_element_of_text_is_its_text() {
        let read = read("<?xml version=\"1.0\"?><volume>27</volume>");
        assert_eq!(
            read,
            Some(("volume".into(), json!("27"))),
            "the text wasn't read"
        );
    }

    #[test]
    fn elements_inside_are_an_object_by_name_without_their_prefix() {
        let (name, value) = read(
            "<s:Envelope xmlns:s=\"urn:soap\"><s:Body>\
             <u:GetMuteResponse xmlns:u=\"urn:rc\"><CurrentMute>0</CurrentMute>\
             </u:GetMuteResponse></s:Body></s:Envelope>",
        )
        .expect("it reads");
        assert_eq!(name, "Envelope", "the prefix stayed on the name");
        assert_eq!(
            value,
            json!({"Body": {"GetMuteResponse": {"CurrentMute": "0"}}}),
            "the elements inside weren't kept by name"
        );
    }

    #[test]
    fn a_name_said_twice_is_a_list() {
        let (_, value) =
            read("<l><icon><w>48</w></icon><icon><w>120</w></icon></l>").expect("it reads");
        assert_eq!(
            value,
            json!({"icon": [{"w": "48"}, {"w": "120"}]}),
            "the second icon replaced the first"
        );
    }

    #[test]
    fn attributes_are_kept_beside_the_elements_inside() {
        let (_, value) = read(
            "<systemInformation><remoteType bundled=\"true\">RM-AAP102</remoteType>\
             <actionHeader name=\"CERS-DEVICE-ID\" /></systemInformation>",
        )
        .expect("it reads");
        assert_eq!(
            value,
            json!({
                "remoteType": {"bundled": "true", "text": "RM-AAP102"},
                "actionHeader": {"name": "CERS-DEVICE-ID"},
            }),
            "an attribute was lost"
        );
    }

    #[test]
    fn escaped_text_reads_as_written() {
        let (_, value) =
            read("<a>VEN_0106&amp;DEV_0500 &lt;b&gt; &#65;&#x42;</a>").expect("it reads");
        assert_eq!(value, json!("VEN_0106&DEV_0500 <b> AB"), "an escape stayed");
    }

    #[test]
    fn comments_and_empty_elements_say_nothing_wrong() {
        let (_, value) =
            read("<!-- a note --><a><CurrentURI></CurrentURI><b/></a>").expect("it reads");
        assert_eq!(
            value,
            json!({"CurrentURI": "", "b": ""}),
            "an empty element wasn't empty text"
        );
    }

    #[test]
    fn what_isnt_xml_reads_as_nothing() {
        assert_eq!(read("not xml"), None, "plain words read as XML");
        assert_eq!(read("<a><b></a>"), None, "a mismatched end read as XML");
    }

    #[test]
    fn an_element_is_found_anywhere_inside() {
        let value = json!({"Body": {"GetVolumeResponse": {"CurrentVolume": "27"}}});
        assert_eq!(
            find(&value, "CurrentVolume"),
            Some(&json!("27")),
            "the volume wasn't found"
        );
        assert_eq!(find(&value, "Missing"), None, "a missing element was found");
    }
}
