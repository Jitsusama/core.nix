//! What a stop reaches: what its words name, and, where they name nothing
//! or only "this", what the place it was said in is about. When nothing
//! narrower is found it reaches everything, and Joel is told loudly.

use std::fmt;

use crate::words::has_run;

/// Something a saying's words name, or the place it was said in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// Every agent, in both realms.
    Everything,
    /// Every agent in one realm, work or personal.
    Realm(String),
    /// One pull request, by its repository where that is known.
    Pr {
        /// The repository as `owner/name`, if the words or the place say.
        repo: Option<String>,
        /// Its number.
        number: u64,
    },
    /// One branch.
    Branch {
        /// The repository as `owner/name`, if known.
        repo: Option<String>,
        /// The branch's name.
        name: String,
    },
    /// One repository, as `owner/name`.
    Repo(String),
    /// One quest and the quests under it, by id.
    Quest(String),
    /// A name the words gave, such as a zone or a lane, for the enforcer to
    /// find among quests and repositories.
    Named(String),
    /// "This PR": whichever the place it was said in is about.
    ThisPr,
    /// "This branch".
    ThisBranch,
    /// "This repo".
    ThisRepo,
    /// "This quest".
    ThisQuest,
}

impl fmt::Display for Target {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Everything => out.write_str("everything"),
            Self::Realm(realm) => write!(out, "realm:{realm}"),
            Self::Pr { repo, number } => {
                write!(out, "pr:{}#{number}", repo.as_deref().unwrap_or(""))
            }
            Self::Branch { repo, name } => {
                write!(out, "branch:{}@{name}", repo.as_deref().unwrap_or(""))
            }
            Self::Repo(repo) => write!(out, "repo:{repo}"),
            Self::Quest(quest) => write!(out, "quest:{quest}"),
            Self::Named(name) => write!(out, "named:{name}"),
            Self::ThisPr => out.write_str("this-pr"),
            Self::ThisBranch => out.write_str("this-branch"),
            Self::ThisRepo => out.write_str("this-repo"),
            Self::ThisQuest => out.write_str("this-quest"),
        }
    }
}

/// What a saying's own words name. `after` is where the phrase ends among
/// `words`, since a bare name ("on aquifer") is read only after it.
#[must_use]
pub fn targets(own: &str, words: &[String], after: usize) -> Vec<Target> {
    const EVERYTHING: [&[&str]; 9] = [
        &["everything"],
        &["all", "of", "it"],
        &["all", "of", "them"],
        &["whole", "swarm"],
        &["all", "agents"],
        &["all", "your", "agents"],
        &["every", "agent"],
        &["all", "robots"],
        &["all", "your", "robots"],
    ];
    if EVERYTHING.iter().any(|run| has_run(words, run)) {
        return vec![Target::Everything];
    }
    let mut found = Vec::new();
    for piece in own.split_whitespace() {
        if let Some(target) = from_piece(piece) {
            found.push(target);
        }
    }
    for (realm, run) in [
        ("work", ["work", "realm"]),
        ("personal", ["personal", "realm"]),
    ] {
        if has_run(words, &run) {
            found.push(Target::Realm(realm.to_owned()));
        }
    }
    for (run, target) in [
        (&["this", "pr"][..], Target::ThisPr),
        (&["this", "pull", "request"][..], Target::ThisPr),
        (&["this", "change"][..], Target::ThisPr),
        (&["this", "branch"][..], Target::ThisBranch),
        (&["this", "repo"][..], Target::ThisRepo),
        (&["this", "repository"][..], Target::ThisRepo),
        (&["this", "quest"][..], Target::ThisQuest),
    ] {
        if has_run(words, run) {
            found.push(target);
        }
    }
    found.extend(numbered_prs(words));
    found.extend(quest_ids(words));
    found.extend(bare_name(words, after));
    tidy(found)
}

/// A pull request, branch or repository a piece of text names: a link to
/// one on any host, `owner/name#12`, or `#12`.
fn from_piece(piece: &str) -> Option<Target> {
    // Slack wraps a link as <url|label>; take the url.
    let bare = piece.trim_start_matches(['<', '(']);
    let bare = bare.split(['|', '>']).next().unwrap_or(bare);
    let bare = bare.trim_end_matches(['.', ',', ';', ':', ')', '!', '?']);
    if let Some(rest) = bare
        .strip_prefix("https://")
        .or_else(|| bare.strip_prefix("http://"))
    {
        return from_link(rest);
    }
    if let Some((repo, number)) = bare.split_once('#') {
        let number = number.parse().ok()?;
        let repo =
            (repo.matches('/').count() == 1 && !repo.starts_with('/') && !repo.ends_with('/'))
                .then(|| repo.to_owned());
        if repo.is_none() && !repo_free(bare) {
            return None;
        }
        return Some(Target::Pr { repo, number });
    }
    None
}

/// Whether a `#12` stands alone, rather than after some word that isn't a
/// repository.
fn repo_free(piece: &str) -> bool {
    piece.starts_with('#')
}

/// What a link names, from the path after its host: a pull request, a
/// branch's tree or a repository. Links to Slack itself name nothing.
fn from_link(rest: &str) -> Option<Target> {
    let (host, path) = rest.split_once('/')?;
    if host.ends_with("slack.com") {
        return None;
    }
    let parts: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
    match parts.as_slice() {
        [owner, name, "pull" | "pulls", number, ..] => Some(Target::Pr {
            repo: Some(format!("{owner}/{name}")),
            number: number.parse().ok()?,
        }),
        [owner, name, "tree", branch @ ..] if !branch.is_empty() => Some(Target::Branch {
            repo: Some(format!("{owner}/{name}")),
            name: branch.join("/"),
        }),
        [owner, name] => Some(Target::Repo(format!("{owner}/{name}"))),
        _ => None,
    }
}

/// Pull requests named as "PR 12" or "pull request 12".
fn numbered_prs(words: &[String]) -> Vec<Target> {
    words
        .windows(2)
        .filter(|pair| pair[0] == "pr" || pair[0] == "request")
        .filter_map(|pair| pair[1].parse().ok())
        .map(|number| Target::Pr { repo: None, number })
        .collect()
}

/// Quests named by id, `QEST-20261113-OX4WFV`, which split into three
/// words.
fn quest_ids(words: &[String]) -> Vec<Target> {
    words
        .windows(3)
        .filter(|three| {
            three[0] == "qest"
                && three[1].len() == 8
                && three[1].chars().all(|c| c.is_ascii_digit())
                && three[2].len() == 6
        })
        .map(|three| Target::Quest(format!("QEST-{}-{}", three[1], three[2]).to_uppercase()))
        .collect()
}

/// A bare name just after the phrase, "pineapple override on aquifer",
/// leaving out the words that are only the sentence going on.
fn bare_name(words: &[String], after: usize) -> Option<Target> {
    const NOT_NAMES: [&str; 28] = [
        "this",
        "that",
        "the",
        "a",
        "an",
        "my",
        "your",
        "his",
        "our",
        "their",
        "all",
        "every",
        "everything",
        "now",
        "real",
        "me",
        "us",
        "you",
        "them",
        "it",
        "good",
        "sure",
        "pr",
        "pull",
        "branch",
        "repo",
        "quest",
        "realm",
    ];
    let lead = words.get(after)?;
    if !matches!(lead.as_str(), "on" | "for" | "in") {
        return None;
    }
    let name = words.get(after + 1)?;
    let is_number = name.chars().all(|c| c.is_ascii_digit());
    // A link or a quest's id names its target already, read elsewhere.
    let starts_another = matches!(name.as_str(), "http" | "https" | "qest");
    (!is_number && !starts_another && !NOT_NAMES.contains(&name.as_str()))
        .then(|| Target::Named(name.clone()))
}

/// The targets once each is said once, and a pull request named by number
/// alone dropped where the same number came with its repository.
fn tidy(found: Vec<Target>) -> Vec<Target> {
    let mut kept: Vec<Target> = Vec::new();
    for target in found {
        let covered = match &target {
            Target::Pr { repo: None, number } => kept.iter().any(
                |other| matches!(other, Target::Pr { repo: Some(_), number: theirs } if theirs == number),
            ),
            _ => false,
        };
        if !covered && !kept.contains(&target) {
            kept.push(target);
        }
    }
    kept
}

/// What the place a saying was said in is about, as its reader finds it.
#[derive(Clone, Debug, Default)]
pub struct Context {
    /// The pull request it sits on, or that its thread is about.
    pub pr: Option<u64>,
    /// The repository of that pull request or branch, as `owner/name`.
    pub repo: Option<String>,
    /// The branch its thread is about.
    pub branch: Option<String>,
    /// The quest its thread belongs to.
    pub quest: Option<String>,
    /// The quests its channel's lanes belong to.
    pub lanes: Vec<String>,
}

/// What a stop reaches, and whether it reaches everything only because
/// nothing narrower was found, which Joel is told loudly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reach {
    /// What it reaches, each named in full.
    pub targets: Vec<Target>,
    /// Whether it fell back to everything.
    pub fell_back: bool,
}

impl Reach {
    fn everything(fell_back: bool) -> Self {
        Self {
            targets: vec![Target::Everything],
            fell_back,
        }
    }
}

/// What a stop reaches, from what its words named and where it was said:
/// its words first, the place it was said in next, its channel's lanes
/// after, and everything when none of them names anything. A "this PR"
/// said where no PR is found reaches everything too, since stopping too
/// much costs a pause Joel lifts and stopping too little costs someone
/// else's work.
#[must_use]
pub fn reach(named: &[Target], context: &Context) -> Reach {
    if named.contains(&Target::Everything) {
        return Reach::everything(false);
    }
    let mut targets = Vec::new();
    for target in named {
        let resolved = match target {
            Target::ThisPr => context.pr.map(|number| Target::Pr {
                repo: context.repo.clone(),
                number,
            }),
            Target::ThisBranch => context.branch.clone().map(|name| Target::Branch {
                repo: context.repo.clone(),
                name,
            }),
            Target::ThisRepo => context.repo.clone().map(Target::Repo),
            Target::ThisQuest => context.quest.clone().map(Target::Quest),
            Target::Pr { repo: None, number } => Some(Target::Pr {
                repo: context.repo.clone(),
                number: *number,
            }),
            other => Some(other.clone()),
        };
        match resolved {
            Some(target) => targets.push(target),
            None => return Reach::everything(true),
        }
    }
    if !targets.is_empty() {
        return Reach {
            targets,
            fell_back: false,
        };
    }
    let from_place = if let Some(number) = context.pr {
        vec![Target::Pr {
            repo: context.repo.clone(),
            number,
        }]
    } else if let Some(name) = context.branch.clone() {
        vec![Target::Branch {
            repo: context.repo.clone(),
            name,
        }]
    } else if let Some(quest) = context.quest.clone() {
        vec![Target::Quest(quest)]
    } else {
        context.lanes.iter().cloned().map(Target::Quest).collect()
    };
    if from_place.is_empty() {
        Reach::everything(true)
    } else {
        Reach {
            targets: from_place,
            fell_back: false,
        }
    }
}
