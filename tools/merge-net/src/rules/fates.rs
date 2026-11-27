//! Folders that share a fate (`fates.txt`): one check measures what they
//! all hold, as the map's live test holds its first visit, files, kilobytes
//! and elements, to one budget with room. A merge judges a rebase by the
//! files main moved, so two changes in different files of the map, each
//! fitting the room alone, would merge as they stand and pass it together
//! on main. A move anywhere in a fate's folders counts as one the change
//! shares, so changed-checks runs again at its head over what main moved.

/// The fates, as `fates.txt` holds them, built into the program so it
/// answers the same on any branch it was built from.
const FATES: &str = include_str!("../../fates.txt");

/// Folders whose files one check measures together.
#[derive(Debug, PartialEq, Eq)]
pub struct Fate {
    pub name: String,
    pub folders: Vec<String>,
}

impl Fate {
    /// Whether `file` lies in one of the fate's folders.
    fn holds(&self, file: &str) -> bool {
        self.folders
            .iter()
            .any(|folder| file.starts_with(folder.as_str()))
    }
}

/// The fates `fates.txt` names.
pub fn fates() -> Vec<Fate> {
    parse(FATES).expect("fates.txt reads as fates, which its own test holds")
}

/// Read fates from lines of `name: folder/...`, skipping comments and
/// blank lines, or say which line doesn't read. A folder ends in `/`, so
/// `modules/mycelium/site/` never holds `modules/mycelium/site-old/`.
pub fn parse(text: &str) -> Result<Vec<Fate>, String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (name, folders) = line
                .split_once(':')
                .ok_or_else(|| format!("no fate named before a colon: {line}"))?;
            let folders: Vec<String> = folders.split_whitespace().map(str::to_owned).collect();
            if folders.is_empty() {
                return Err(format!("a fate needs its folders: {line}"));
            }
            if let Some(bare) = folders.iter().find(|folder| !folder.ends_with('/')) {
                return Err(format!(
                    "a fate's folder ends in `/`, which {bare} doesn't: {line}"
                ));
            }
            Ok(Fate {
                name: name.trim().to_owned(),
                folders,
            })
        })
        .collect()
}

/// The files of `main`, what main moved, in a fate one of the branch's
/// files is in.
pub fn beside(fates: &[Fate], branch: &[String], main: &[String]) -> Vec<String> {
    let touched: Vec<&Fate> = fates
        .iter()
        .filter(|fate| branch.iter().any(|file| fate.holds(file)))
        .collect();
    main.iter()
        .filter(|file| touched.iter().any(|fate| fate.holds(file)))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Fate, beside, fates, parse};

    fn owned(files: &[&str]) -> Vec<String> {
        files.iter().map(|file| (*file).to_owned()).collect()
    }

    fn the_map() -> Vec<Fate> {
        parse("the map: modules/mycelium/site/ modules/local-sites/kit/")
            .expect("the map's fate reads")
    }

    #[test]
    fn fates_txt_reads_and_names_the_map() {
        let named: Vec<String> = fates().into_iter().map(|fate| fate.name).collect();
        assert!(
            named.iter().any(|name| name == "the map's first visit"),
            "fates.txt lost the map's fate: {named:?}"
        );
    }

    #[test]
    fn every_folder_the_maps_check_is_built_from_shares_its_fate() {
        // The page is built from the theme's web kit and the sites' strip,
        // build and feeds too, so a move in any can join a map change in
        // overrunning its room.
        let moved = owned(&[
            "modules/theme/web/base.css",
            "modules/local-sites/home/strip.css",
            "modules/local-sites/site.nix",
            "modules/local-sites/home.nix",
            "modules/local-sites/kit/head.css",
        ]);
        let shared = beside(
            &fates(),
            &owned(&["modules/mycelium/site/view.js"]),
            &moved,
        );
        assert_eq!(shared, moved, "a move the map's check is built from wasn't shared");
    }

    #[test]
    fn two_changes_in_different_files_of_one_folder_share_it() {
        let shared = beside(
            &the_map(),
            &owned(&["modules/mycelium/site/view.js"]),
            &owned(&["modules/mycelium/site/site.css", "docs/testing.md"]),
        );
        assert_eq!(
            shared,
            owned(&["modules/mycelium/site/site.css"]),
            "a move beside the change in its folder wasn't shared"
        );
    }

    #[test]
    fn a_move_in_another_folder_of_the_fate_is_shared() {
        let shared = beside(
            &the_map(),
            &owned(&["modules/mycelium/site/tests/map-reads-live-test.py"]),
            &owned(&["modules/local-sites/kit/head.css"]),
        );
        assert_eq!(
            shared,
            owned(&["modules/local-sites/kit/head.css"]),
            "a move in the kit wasn't shared with a change to the map"
        );
    }

    #[test]
    fn a_change_outside_every_fate_shares_nothing_by_one() {
        let shared = beside(
            &the_map(),
            &owned(&["modules/net/site/panel.ts"]),
            &owned(&["modules/mycelium/site/site.css"]),
        );
        assert!(
            shared.is_empty(),
            "a change outside the fate shared {shared:?}"
        );
    }

    #[test]
    fn a_folder_holds_only_what_is_under_it() {
        let shared = beside(
            &the_map(),
            &owned(&["modules/mycelium/site-old/view.js"]),
            &owned(&["modules/mycelium/site/site.css"]),
        );
        assert!(
            shared.is_empty(),
            "a folder beside the fate's shared {shared:?}"
        );
    }

    #[test]
    fn a_fate_refuses_a_line_it_cannot_read() {
        for line in ["no colon here", "bare:", "slashless: modules/mycelium/site"] {
            assert!(parse(line).is_err(), "read a fate from {line:?}");
        }
        assert_eq!(
            parse("# a comment\n\n").expect("comments and blanks read as none"),
            Vec::new(),
            "comments and blank lines read as fates"
        );
    }
}
