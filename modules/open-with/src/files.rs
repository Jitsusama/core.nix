//! The portal's chooser of files, for niri. When a program asks to open a
//! file or a folder, or to save one or several, the portal asks its
//! `FileChooser`, which GTK's portal answers with a dialog of its own. This
//! asks the launcher instead, as the chooser of an app is asked (main.rs).
//!
//! The shell's file-chooser function is called with the socket's place and
//! one request, in JSON: whether it opens, saves or saves several, the title,
//! the word for Return, whether it's folders and whether several may be
//! picked, the folder to start in, the name and the file to save over, the
//! filters and the one in use, the choices, and the files to save. The shell
//! answers on the socket with one JSON line: the paths picked, the filter
//! picked by its place in the list, and the option picked for each choice,
//! or an empty line for none. Saving several, the one path is the folder
//! they go in.
//!
//! The shell is held to what was asked: a path must be whole, one must be
//! picked where several may not, and a filter or option must be one offered.
//! Every file is chosen in the launcher, never in another dialog (Joel,
//! 2026-10-26), so a shell that can't be asked, as while it restarts, leaves
//! the program with nothing chosen, and the journal says why.

use std::collections::HashMap;
use std::fmt::Write;
use std::path::Path;
use std::sync::Arc;

use serde_json::{Value as Json, json};
use zbus::interface;
use zbus::object_server::ObjectServer;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

use crate::{Ended, NEITHER, PICKED, REFUSED, Shell, Target, text};

/// The shell's chooser of files, whose answer has room for a great many
/// paths.
const FILE_CHOOSER: Target = Target {
    name: "file-chooser",
    longest: 1 << 20,
};

/// A filter: its name, and its rules, each a glob (0) or a kind (1).
type Filter = (String, Vec<(u32, String)>);
/// A choice: its id, its label, its options as id and label, and the one
/// picked to start with. One with no options is a yes or no.
type Choice = (String, String, Vec<(String, String)>, String);

#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    Open,
    Save,
    SaveFiles,
}

impl Mode {
    fn named(self) -> &'static str {
        match self {
            Mode::Open => "open",
            Mode::Save => "save",
            Mode::SaveFiles => "save-files",
        }
    }
}

/// An option of a type of its own, if it has one and it is that type.
fn option<T: TryFrom<OwnedValue>>(options: &HashMap<String, OwnedValue>, key: &str) -> Option<T> {
    options.get(key)?.try_clone().ok()?.try_into().ok()
}

/// A path the portal gives as bytes ending in a nul, if it gives one.
fn path(options: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    named(&option::<Vec<u8>>(options, key)?)
}

/// Bytes ending in a nul, as text, if there's any.
fn named(bytes: &[u8]) -> Option<String> {
    let bytes = bytes.strip_suffix(&[0]).unwrap_or(bytes);
    (!bytes.is_empty()).then(|| String::from_utf8_lossy(bytes).into_owned())
}

/// A path as a file's URI, each byte but the plain ones escaped.
fn uri(path: &str) -> String {
    let mut uri = String::from("file://");
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~".contains(&byte) {
            uri.push(char::from(byte));
        } else {
            let _ = write!(uri, "%{byte:02X}");
        }
    }
    uri
}

/// What a program asked for, and what the shell's answer is held to.
#[derive(Debug)]
struct Asked {
    mode: Mode,
    multiple: bool,
    filters: Vec<Filter>,
    choices: Vec<Choice>,
    files: Vec<String>,
}

/// What the shell picked, as the portal is told it.
#[derive(Debug, PartialEq)]
struct Picked {
    uris: Vec<String>,
    filter: Option<Filter>,
    choices: Vec<(String, String)>,
}

impl Asked {
    fn new(mode: Mode, options: &HashMap<String, OwnedValue>) -> Asked {
        Asked {
            mode,
            multiple: mode == Mode::Open && option(options, "multiple").unwrap_or(false),
            filters: option(options, "filters").unwrap_or_default(),
            choices: option(options, "choices").unwrap_or_default(),
            files: option::<Vec<Vec<u8>>>(options, "files")
                .unwrap_or_default()
                .iter()
                .filter_map(|file| named(file))
                .collect(),
        }
    }

    /// The request the shell is given, in JSON.
    fn request(&self, title: &str, options: &HashMap<String, OwnedValue>) -> Json {
        let current = option::<Filter>(options, "current_filter");
        let rules = |rules: &[(u32, String)], kind: u32| -> Vec<String> {
            rules
                .iter()
                .filter(|(each, _)| *each == kind)
                .map(|(_, rule)| rule.clone())
                .collect()
        };
        json!({
            "mode": self.mode.named(),
            "title": title,
            "accept": text(options, "accept_label"),
            "modal": option(options, "modal").unwrap_or(true),
            "folders": self.mode == Mode::Open && option(options, "directory").unwrap_or(false),
            "multiple": self.multiple,
            "folder": path(options, "current_folder"),
            "name": text(options, "current_name"),
            "current": path(options, "current_file"),
            "filters": self.filters.iter().map(|(name, each)| json!({
                "name": name,
                "globs": rules(each, 0),
                "kinds": rules(each, 1),
            })).collect::<Vec<_>>(),
            "filter": current.and_then(|current| self.filters.iter().position(|each| *each == current)),
            "choices": self.choices.iter().map(|(id, label, offered, picked)| json!({
                "id": id,
                "label": label,
                "options": offered.iter().map(|(id, label)| json!({"id": id, "label": label})).collect::<Vec<_>>(),
                "default": picked,
            })).collect::<Vec<_>>(),
            "files": self.files,
        })
    }

    /// What the portal is told for how the ask ended, and what was picked,
    /// held to what was asked.
    fn response(&self, ended: &Ended) -> (u32, Option<Picked>) {
        match ended {
            Ended::Said(said) if said.is_empty() => (REFUSED, None),
            Ended::Said(said) => self
                .picked(said)
                .map_or((NEITHER, None), |picked| (PICKED, Some(picked))),
            _ => (NEITHER, None),
        }
    }

    fn picked(&self, said: &str) -> Option<Picked> {
        let said: Json = serde_json::from_str(said).ok()?;
        let paths = said
            .get("paths")?
            .as_array()?
            .iter()
            .map(|path| path.as_str().filter(|path| path.starts_with('/')))
            .collect::<Option<Vec<_>>>()?;
        let uris = match (self.mode, paths.as_slice()) {
            (_, []) => return None,
            (Mode::Open, _) if self.multiple => paths.iter().map(|path| uri(path)).collect(),
            (Mode::Open | Mode::Save, [path]) => vec![uri(path)],
            (Mode::SaveFiles, [folder]) if !self.files.is_empty() => self
                .files
                .iter()
                .filter_map(|file| Path::new(file).file_name())
                .map(|file| uri(&Path::new(folder).join(file).to_string_lossy()))
                .collect(),
            _ => return None,
        };
        let filter = said
            .get("filter")
            .and_then(Json::as_u64)
            .and_then(|at| self.filters.get(usize::try_from(at).ok()?))
            .cloned();
        let answered = said.get("choices").and_then(Json::as_object);
        let choices = self
            .choices
            .iter()
            .filter_map(|(id, _, offered, _)| {
                let picked = answered?.get(id)?.as_str()?;
                let fits = if offered.is_empty() {
                    ["true", "false"].contains(&picked)
                } else {
                    offered.iter().any(|(each, _)| each == picked)
                };
                fits.then(|| (id.clone(), picked.to_owned()))
            })
            .collect();
        Some(Picked {
            uris,
            filter,
            choices,
        })
    }
}

/// The portal's results for what was picked.
fn results(picked: Picked) -> HashMap<String, OwnedValue> {
    let mut results = HashMap::new();
    let mut put = |key: &str, value: Value<'_>| {
        if let Ok(value) = OwnedValue::try_from(value) {
            results.insert(key.to_owned(), value);
        }
    };
    put("uris", Value::from(picked.uris));
    if let Some(filter) = picked.filter {
        put("current_filter", Value::from(filter));
    }
    if !picked.choices.is_empty() {
        put("choices", Value::from(picked.choices));
    }
    results
}

pub(crate) struct Files {
    shell: Arc<Shell>,
}

impl Files {
    pub(crate) fn new(shell: Arc<Shell>) -> Files {
        Files { shell }
    }

    async fn choose(
        &self,
        mode: Mode,
        handle: OwnedObjectPath,
        app_id: String,
        title: String,
        options: HashMap<String, OwnedValue>,
        server: &ObjectServer,
    ) -> (u32, HashMap<String, OwnedValue>) {
        let asked = Asked::new(mode, &options);
        let words = vec![asked.request(&title, &options).to_string()];
        let (ended, _) = self
            .shell
            .ask(&FILE_CHOOSER, &handle, server, Vec::new(), words, None)
            .await;
        if ended == Ended::Unasked {
            eprintln!("<3>open-with: the launcher couldn't be asked, so {app_id} chose no file");
        }
        let (code, picked) = asked.response(&ended);
        (code, picked.map(results).unwrap_or_default())
    }
}

#[interface(name = "org.freedesktop.impl.portal.FileChooser")]
impl Files {
    #[zbus(out_args("response", "results"))]
    async fn open_file(
        &self,
        handle: OwnedObjectPath,
        app_id: String,
        parent_window: String,
        title: String,
        options: HashMap<String, OwnedValue>,
        #[zbus(object_server)] server: &ObjectServer,
    ) -> (u32, HashMap<String, OwnedValue>) {
        // The shell asks over its own window, whichever program asked.
        let _ = parent_window;
        self.choose(Mode::Open, handle, app_id, title, options, server)
            .await
    }

    #[zbus(out_args("response", "results"))]
    async fn save_file(
        &self,
        handle: OwnedObjectPath,
        app_id: String,
        parent_window: String,
        title: String,
        options: HashMap<String, OwnedValue>,
        #[zbus(object_server)] server: &ObjectServer,
    ) -> (u32, HashMap<String, OwnedValue>) {
        let _ = parent_window;
        self.choose(Mode::Save, handle, app_id, title, options, server)
            .await
    }

    #[zbus(out_args("response", "results"))]
    async fn save_files(
        &self,
        handle: OwnedObjectPath,
        app_id: String,
        parent_window: String,
        title: String,
        options: HashMap<String, OwnedValue>,
        #[zbus(object_server)] server: &ObjectServer,
    ) -> (u32, HashMap<String, OwnedValue>) {
        let _ = parent_window;
        self.choose(Mode::SaveFiles, handle, app_id, title, options, server)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zbus::zvariant::Str;

    fn value(value: Value<'_>) -> OwnedValue {
        OwnedValue::try_from(value).unwrap()
    }

    fn bytes(text: &str) -> Vec<u8> {
        let mut bytes = text.as_bytes().to_vec();
        bytes.push(0);
        bytes
    }

    fn pictures() -> Filter {
        (
            "Pictures".to_owned(),
            vec![(0, "*.png".to_owned()), (1, "image/jpeg".to_owned())],
        )
    }

    fn everything() -> Filter {
        ("Everything".to_owned(), vec![(0, "*".to_owned())])
    }

    fn encoding() -> Choice {
        (
            "encoding".to_owned(),
            "Encoding".to_owned(),
            vec![
                ("utf8".to_owned(), "Unicode".to_owned()),
                ("latin1".to_owned(), "Western".to_owned()),
            ],
            "utf8".to_owned(),
        )
    }

    fn reencode() -> Choice {
        (
            "reencode".to_owned(),
            "Re-encode".to_owned(),
            Vec::new(),
            "false".to_owned(),
        )
    }

    fn options() -> HashMap<String, OwnedValue> {
        HashMap::from([
            (
                "accept_label".to_owned(),
                value(Value::from(Str::from("_Open"))),
            ),
            ("modal".to_owned(), value(Value::from(false))),
            ("multiple".to_owned(), value(Value::from(true))),
            (
                "filters".to_owned(),
                value(Value::from(vec![pictures(), everything()])),
            ),
            (
                "current_filter".to_owned(),
                value(Value::from(everything())),
            ),
            (
                "choices".to_owned(),
                value(Value::from(vec![encoding(), reencode()])),
            ),
            (
                "current_folder".to_owned(),
                value(Value::from(bytes("/home/someone/Pictures"))),
            ),
        ])
    }

    #[test]
    fn the_request_says_what_the_program_asked() {
        let asked = Asked::new(Mode::Open, &options());
        assert_eq!(
            asked.request("Open a picture", &options()),
            json!({
                "mode": "open",
                "title": "Open a picture",
                "accept": "_Open",
                "modal": false,
                "folders": false,
                "multiple": true,
                "folder": "/home/someone/Pictures",
                "name": null,
                "current": null,
                "filters": [
                    {"name": "Pictures", "globs": ["*.png"], "kinds": ["image/jpeg"]},
                    {"name": "Everything", "globs": ["*"], "kinds": []},
                ],
                "filter": 1,
                "choices": [
                    {"id": "encoding", "label": "Encoding", "options": [
                        {"id": "utf8", "label": "Unicode"},
                        {"id": "latin1", "label": "Western"},
                    ], "default": "utf8"},
                    {"id": "reencode", "label": "Re-encode", "options": [], "default": "false"},
                ],
                "files": [],
            })
        );
    }

    #[test]
    fn a_request_with_no_options_is_modal_for_one_file() {
        let request = Asked::new(Mode::Save, &HashMap::new()).request("Save", &HashMap::new());
        assert_eq!(request["mode"], "save");
        assert_eq!(request["modal"], true);
        assert_eq!(request["multiple"], false);
        assert_eq!(request["filter"], Json::Null);
    }

    fn said(text: &str) -> Ended {
        Ended::Said(text.to_owned())
    }

    #[test]
    fn a_file_picked_is_opened_by_its_uri() {
        let asked = Asked::new(Mode::Open, &HashMap::new());
        assert_eq!(
            asked.response(&said(r#"{"paths": ["/home/someone/a note é.md"]}"#)),
            (
                PICKED,
                Some(Picked {
                    uris: vec!["file:///home/someone/a%20note%20%C3%A9.md".to_owned()],
                    filter: None,
                    choices: Vec::new(),
                })
            )
        );
    }

    #[test]
    fn nothing_picked_is_refused() {
        assert_eq!(
            Asked::new(Mode::Open, &HashMap::new()).response(&said("")),
            (REFUSED, None)
        );
    }

    #[test]
    fn an_answer_that_breaks_what_was_asked_is_neither() {
        let one = Asked::new(Mode::Open, &HashMap::new());
        for answer in [
            r#"{"paths": ["/a", "/b"]}"#,
            r#"{"paths": ["relative.md"]}"#,
            r#"{"paths": []}"#,
            r#"{"picked": "/a"}"#,
            "/a",
        ] {
            assert_eq!(one.response(&said(answer)), (NEITHER, None), "{answer}");
        }
        for ended in [Ended::Closed, Ended::Late, Ended::Unasked] {
            assert_eq!(one.response(&ended), (NEITHER, None));
        }
    }

    #[test]
    fn several_are_opened_only_where_several_may_be() {
        let several = Asked::new(Mode::Open, &options());
        let (code, picked) = several.response(&said(r#"{"paths": ["/a", "/b"]}"#));
        assert_eq!(code, PICKED);
        assert_eq!(picked.unwrap().uris, ["file:///a", "file:///b"]);
    }

    #[test]
    fn a_filter_and_options_are_kept_only_as_offered() {
        let asked = Asked::new(Mode::Open, &options());
        let (_, picked) = asked.response(&said(
            r#"{"paths": ["/a"], "filter": 0, "choices": {"encoding": "latin1", "reencode": "true", "stray": "x"}}"#,
        ));
        let picked = picked.unwrap();
        assert_eq!(picked.filter, Some(pictures()));
        assert_eq!(
            picked.choices,
            [
                ("encoding".to_owned(), "latin1".to_owned()),
                ("reencode".to_owned(), "true".to_owned())
            ]
        );
        let (_, picked) = asked.response(&said(
            r#"{"paths": ["/a"], "filter": 7, "choices": {"encoding": "ebcdic", "reencode": "maybe"}}"#,
        ));
        let picked = picked.unwrap();
        assert_eq!(picked.filter, None);
        assert!(picked.choices.is_empty());
    }

    #[test]
    fn several_saved_go_in_the_folder_picked_by_their_own_names() {
        let options = HashMap::from([(
            "files".to_owned(),
            value(Value::from(vec![bytes("one.txt"), bytes("../two.txt")])),
        )]);
        let asked = Asked::new(Mode::SaveFiles, &options);
        assert_eq!(
            asked.request("Save", &options)["files"],
            json!(["one.txt", "../two.txt"])
        );
        let (code, picked) = asked.response(&said(r#"{"paths": ["/home/someone/Saved"]}"#));
        assert_eq!(code, PICKED);
        assert_eq!(
            picked.unwrap().uris,
            [
                "file:///home/someone/Saved/one.txt",
                "file:///home/someone/Saved/two.txt"
            ]
        );
    }

    #[test]
    fn what_was_picked_reaches_the_portal_in_its_types() {
        let results = results(Picked {
            uris: vec!["file:///a".to_owned()],
            filter: Some(pictures()),
            choices: vec![("encoding".to_owned(), "utf8".to_owned())],
        });
        let uris: Vec<String> = results["uris"].try_clone().unwrap().try_into().unwrap();
        assert_eq!(uris, ["file:///a"]);
        let filter: Filter = results["current_filter"]
            .try_clone()
            .unwrap()
            .try_into()
            .unwrap();
        assert_eq!(filter, pictures());
        let choices: Vec<(String, String)> =
            results["choices"].try_clone().unwrap().try_into().unwrap();
        assert_eq!(choices, [("encoding".to_owned(), "utf8".to_owned())]);
    }
}
