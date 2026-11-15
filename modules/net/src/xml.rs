//! Enough of XML to keep what a device answers in its own words: UPnP's
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
    let _ = text;
    None
}

/// The first element named `name` anywhere inside `value`, depth first.
pub fn find<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
    let _ = (value, name);
    None
}

#[expect(dead_code, reason = "filled in once its cases fail")]
fn unescape(text: &str) -> String {
    text.to_owned()
}

#[expect(dead_code, reason = "filled in once its cases fail")]
fn local(name: &str) -> &str {
    name
}

#[expect(dead_code, reason = "filled in once its cases fail")]
fn put(fields: &mut Map<String, Value>, name: String, value: Value) {
    let _ = (fields, name, value);
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
        let (_, value) = read("<l><icon><w>48</w></icon><icon><w>120</w></icon></l>")
            .expect("it reads");
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
        let (_, value) = read("<a>VEN_0106&amp;DEV_0500 &lt;b&gt; &#65;&#x42;</a>").expect("it reads");
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
