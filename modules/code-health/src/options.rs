//! What code-health is asked to read: the repositories its flags name,
//! each with its ref, folders, targets and owners, and the folder its
//! files go into, refused whole when a flag names what no repository or
//! measure is.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::hour;

/// The flags it takes, each with a value.
const FLAGS: [&str; 6] = [
    "--repo", "--ref", "--under", "--target", "--owner", "--into",
];

/// Every measure it reads (measure.rs and merges.rs).
pub const MEASURES: [&str; 20] = [
    "crowded-folders",
    "daemons-waiting",
    "duplicated-lines",
    "edit-to-merge",
    "files-per-change",
    "folders-without-notes",
    "lints-off",
    "long-files",
    "quarantined-tests",
    "shared-files",
    "suppressions",
    "tf-invalid",
    "tf-undocumented",
    "tf-unformatted",
    "tf-untested",
    "tf-warnings",
    "tflint-findings",
    "trivy-findings",
    "unreached-files",
    "vm-test-lines",
];

/// A repository to read: where its checkout is, which ref, which folder
/// of it, and what it's judged by.
#[derive(Debug, Default, PartialEq)]
pub struct Repo {
    pub checkout: String,
    pub reference: String,
    pub under: Vec<String>,
    pub judged: hour::Judged,
}

/// The repositories asked for, by name, and the folder to write into.
pub fn parse(arguments: &[String]) -> Result<(BTreeMap<String, Repo>, Option<PathBuf>), String> {
    let mut repos: BTreeMap<String, Repo> = BTreeMap::new();
    let mut refs = BTreeMap::new();
    let mut unders = BTreeMap::new();
    let mut targets = Vec::new();
    let mut owners = Vec::new();
    let mut into = None;
    let mut given = arguments.iter();
    while let Some(flag) = given.next() {
        if !FLAGS.contains(&flag.as_str()) {
            return Err(format!(
                "no flag called {flag}; the flags are {}",
                FLAGS.join(", ")
            ));
        }
        let value = given
            .next()
            .ok_or_else(|| format!("{flag} wants a value"))?;
        if flag == "--into" {
            into = Some(PathBuf::from(value));
            continue;
        }
        let (name, said) = value
            .split_once('=')
            .ok_or_else(|| format!("{flag} wants NAME=VALUE, not {value}"))?;
        let (name, said) = (name.to_owned(), said.to_owned());
        match flag.as_str() {
            "--repo" => {
                let repo = Repo {
                    checkout: said,
                    reference: "origin/main".to_owned(),
                    ..Repo::default()
                };
                if repos.insert(name.clone(), repo).is_some() {
                    return Err(format!("{name} is named twice"));
                }
            }
            "--ref" => {
                refs.insert(name, said);
            }
            "--under" => {
                // An empty folder is the whole repository, as none is.
                let folders: &mut Vec<String> = unders.entry(name).or_default();
                if !said.is_empty() {
                    folders.push(said);
                }
            }
            "--target" => {
                let figure = said
                    .parse::<f64>()
                    .map_err(|_| format!("--target {name} wants a figure, not {said}"))?;
                targets.push((name, figure));
            }
            "--owner" => {
                owners.push((name, said));
            }
            _ => unreachable!("FLAGS names every flag but --into, taken above"),
        }
    }
    for (name, reference) in refs {
        let repo = repos
            .get_mut(&name)
            .ok_or_else(|| format!("--ref names {name}, which no --repo does"))?;
        repo.reference = reference;
    }
    for (name, under) in unders {
        let repo = repos
            .get_mut(&name)
            .ok_or_else(|| format!("--under names {name}, which no --repo does"))?;
        repo.under = under;
    }
    for (named, figure) in targets {
        let (name, measure) = measured(&named, "--target")?;
        let measure = measure.ok_or_else(|| format!("--target wants NAME/MEASURE, not {named}"))?;
        judged(&mut repos, name, "--target")?
            .targets
            .insert(measure.to_owned(), figure);
    }
    for (named, owner) in owners {
        if let Some((name, folder)) = named.split_once(':') {
            let folder = folder.trim_end_matches('/').to_owned();
            judged(&mut repos, name, "--owner")?
                .folders
                .insert(folder, owner);
            continue;
        }
        let (name, measure) = measured(&named, "--owner")?;
        let judged = judged(&mut repos, name, "--owner")?;
        match measure {
            Some(measure) => {
                judged.owners.insert(measure.to_owned(), owner);
            }
            None => judged.owner = Some(owner),
        }
    }
    if repos.is_empty() {
        return Err("no --repo given".to_owned());
    }
    Ok((repos, into))
}

/// A `NAME/MEASURE` split, or a bare `NAME`, its measure one it reads.
fn measured<'a>(named: &'a str, flag: &str) -> Result<(&'a str, Option<&'a str>), String> {
    match named.split_once('/') {
        Some((name, measure)) if MEASURES.contains(&measure) => Ok((name, Some(measure))),
        Some((_, measure)) => Err(format!(
            "{flag} names {measure}, which isn't a measure; the measures are {}",
            MEASURES.join(", ")
        )),
        None => Ok((named, None)),
    }
}

/// What the repository a flag names is judged by.
fn judged<'a>(
    repos: &'a mut BTreeMap<String, Repo>,
    name: &str,
    flag: &str,
) -> Result<&'a mut hour::Judged, String> {
    repos
        .get_mut(name)
        .map(|repo| &mut repo.judged)
        .ok_or_else(|| format!("{flag} names {name}, which no --repo does"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::measure;

    fn said(words: &str) -> Vec<String> {
        words.split(' ').map(str::to_owned).collect()
    }

    #[test]
    fn a_ref_and_folder_go_to_the_repo_they_name() {
        let (repos, into) = parse(&said(
            "--repo core=/src/core --repo zone=/world --ref zone=main --under zone=home/me \
             --into /out",
        ))
        .expect("good arguments were refused");
        assert_eq!(
            repos["zone"],
            Repo {
                checkout: "/world".to_owned(),
                reference: "main".to_owned(),
                under: vec!["home/me".to_owned()],
                judged: hour::Judged::default(),
            },
            "a repository's ref or folder was lost"
        );
        assert_eq!(
            repos["core"].reference, "origin/main",
            "a repository given no ref wasn't read at origin's main"
        );
        assert_eq!(into, Some(PathBuf::from("/out")), "--into was lost");
    }

    #[test]
    fn a_ref_for_no_repo_is_refused() {
        assert!(
            parse(&said("--repo core=/src/core --ref other=main")).is_err(),
            "a ref naming no repository was taken"
        );
        assert!(
            parse(&said("--into /out")).is_err(),
            "no repository was taken"
        );
    }

    #[test]
    fn a_target_and_owner_go_to_the_measure_they_name() {
        let (repos, _) = parse(&said(
            "--repo core=/src/core --target core/long-files=96 --owner core=someone \
             --owner core/suppressions=another",
        ))
        .expect("good targets were refused");
        let judged = &repos["core"].judged;
        assert_eq!(
            judged.targets,
            BTreeMap::from([("long-files".to_owned(), 96.0)]),
            "a target was lost"
        );
        assert_eq!(
            (
                judged.owner.as_deref(),
                judged.owners.get("suppressions").map(String::as_str)
            ),
            (Some("someone"), Some("another")),
            "a repository's or a measure's owner was lost"
        );
    }

    #[test]
    fn a_folders_owner_goes_to_its_repository() {
        let (repos, _) = parse(&said(
            "--repo core=/src/core --owner core:modules/net/=someone \
             --owner core:modules/net/switch=another",
        ))
        .expect("good folder owners were refused");
        assert_eq!(
            repos["core"].judged.folders,
            BTreeMap::from([
                ("modules/net".to_owned(), "someone".to_owned()),
                ("modules/net/switch".to_owned(), "another".to_owned()),
            ]),
            "a folder's owner was lost"
        );
    }

    #[test]
    fn every_measure_read_can_be_given_a_target() {
        let quarantine = measure::File {
            path: "quarantine.json".to_owned(),
            text: "{}".to_owned(),
        };
        let health = measure::measure(&[], &[quarantine]);
        for measure in health.figures.keys() {
            assert!(MEASURES.contains(measure), "{measure} isn't in MEASURES");
        }
    }

    #[test]
    fn a_target_for_no_measure_or_repo_is_refused() {
        for given in [
            "--repo core=/src/core --target core/long-file=96",
            "--repo core=/src/core --target other/long-files=96",
            "--repo core=/src/core --target core=96",
            "--repo core=/src/core --target core/long-files=many",
            "--repo core=/src/core --owner other=someone",
        ] {
            assert!(parse(&said(given)).is_err(), "{given} was taken");
        }
    }

    #[test]
    fn an_unknown_flag_is_named_before_its_value_is_read() {
        let refused = parse(&said("--rep core")).expect_err("an unknown flag was taken");
        assert!(
            refused.starts_with("no flag called --rep"),
            "the unknown flag wasn't named: {refused}"
        );
    }
}
