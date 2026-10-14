//! Every dollar in a window, as pieces: an hour, a tool, a model, a
//! session and what it served, its dollars and where they came from. Every
//! cut (by quest, model, agent, day) is a sum of these, so every report
//! agrees with every other.
//!
//! The best figure for an hour, in order:
//!
//! - a feed's warehouse, once it has settled the hour: exact by tool and
//!   model, and by session when it gives the split;
//! - a feed's live meter: billed to the cent, but only by model, so the
//!   hour's split across sessions follows the logs' shares;
//! - pi's logs priced at list, times each model's ratio of bill to list
//!   learned while there was a bill: an estimate, and said so;
//! - pi's logs at list prices, where no feed has ever billed anything.
//!
//! A session the warehouse bills that no log here and no run knows is
//! "elsewhere": another machine, a one-shot run that keeps no log.

use crate::estimate::Ratios;
use crate::history::History;
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Source {
    Warehouse,
    Meter,
    Estimated,
    List,
}

impl Source {
    pub fn name(self) -> &'static str {
        match self {
            Source::Warehouse => "billed",
            Source::Meter => "billed, live",
            Source::Estimated => "estimated",
            Source::List => "list prices",
        }
    }

    /// Whether its dollars are a guess at a bill, and so read "about".
    /// List prices are exactly what pi charges at list; where they stand
    /// in for a bill, the source line says so.
    pub fn estimated(self) -> bool {
        matches!(self, Source::Estimated)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    pub hour: i64,
    pub tool: String,
    pub model: String,
    /// The session that spent it, "elsewhere" for one no log knows, or
    /// empty for another tool's spend.
    pub session: String,
    pub quest: Option<String>,
    pub repo: String,
    /// What kind of work: agent, compaction, side:<extension>,
    /// subagent:<kind>, or the tool's own word.
    pub kind: String,
    pub persona: String,
    pub dollars: f64,
    pub list: f64,
    pub source: Source,
}

/// What pi's side knows of one hour: list-priced pieces from the logs and
/// the runs.
#[derive(Debug, Clone, PartialEq)]
struct Known {
    session: String,
    quest: Option<String>,
    repo: String,
    model: String,
    kind: String,
    persona: String,
    list: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct BilledRow {
    source: String,
    tool: String,
    model: String,
    session: String,
    dollars: f64,
}

/// What the history knows about one window, read once.
#[derive(Debug, Default)]
pub struct Inputs {
    known: BTreeMap<i64, Vec<Known>>,
    billed: BTreeMap<i64, Vec<BilledRow>>,
    /// Each source's settled hours: [first, through).
    settled: HashMap<String, (i64, i64)>,
    /// Sessions a log here knows.
    logged: HashSet<String>,
    /// A subagent's session, by id, to the run it was part of.
    subagents: HashMap<String, Known>,
    /// Whether any feed has ever billed anything.
    pub fed: bool,
}

fn fail(error: rusqlite::Error) -> String {
    error.to_string()
}

impl Inputs {
    pub fn read(history: &History, from: i64, to: i64) -> Result<Inputs, String> {
        let db = &history.db;
        let mut inputs = Inputs::default();
        let mut repos: HashMap<String, String> = HashMap::new();
        {
            let mut statement = db.prepare("SELECT id, repo FROM sessions").map_err(fail)?;
            let rows = statement
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(fail)?;
            for row in rows {
                let (id, repo) = row.map_err(fail)?;
                inputs.logged.insert(id.clone());
                repos.insert(id, repo);
            }
        }
        // The quest a session served in each hour, for its subagents.
        let mut quest_at: HashMap<(i64, String), Option<String>> = HashMap::new();
        {
            let mut statement = db
                .prepare(
                    "SELECT hour, session, quest, model, kind, SUM(cost) FROM turns
                     WHERE hour >= ?1 AND hour < ?2 GROUP BY hour, session, quest, model, kind",
                )
                .map_err(fail)?;
            let rows = statement
                .query_map([from, to], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, f64>(5)?,
                    ))
                })
                .map_err(fail)?;
            for row in rows {
                let (hour, session, quest, model, kind, list) = row.map_err(fail)?;
                quest_at.insert((hour, session.clone()), quest.clone());
                inputs.known.entry(hour).or_default().push(Known {
                    repo: repos.get(&session).cloned().unwrap_or_default(),
                    session,
                    quest,
                    model,
                    kind,
                    persona: String::new(),
                    list,
                });
            }
        }
        {
            let mut statement = db
                .prepare("SELECT kind, model, persona, parent, started, cost, sessions FROM runs")
                .map_err(fail)?;
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, f64>(4)?,
                        row.get::<_, f64>(5)?,
                        row.get::<_, String>(6)?,
                    ))
                })
                .map_err(fail)?;
            for row in rows {
                let (kind, model, persona, parent, started, cost, sessions) = row.map_err(fail)?;
                let hour = crate::time::hour_of(started);
                let parent = parent.unwrap_or_default();
                let known = Known {
                    quest: quest_at.get(&(hour, parent.clone())).cloned().flatten(),
                    repo: repos.get(&parent).cloned().unwrap_or_default(),
                    session: parent,
                    model,
                    kind: format!("subagent:{kind}"),
                    persona,
                    list: cost,
                };
                let ids: Vec<String> = serde_json::from_str(&sessions).unwrap_or_default();
                for id in ids {
                    inputs.subagents.insert(
                        id,
                        Known {
                            list: 0.0,
                            ..known.clone()
                        },
                    );
                }
                if (from..to).contains(&hour) {
                    inputs.known.entry(hour).or_default().push(known);
                }
            }
        }
        {
            let mut statement = db
                .prepare(
                    "SELECT hour, source, tool, model, session, SUM(dollars) FROM billed
                     WHERE hour >= ?1 AND hour < ?2 GROUP BY hour, source, tool, model, session",
                )
                .map_err(fail)?;
            let rows = statement
                .query_map([from, to], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        BilledRow {
                            source: row.get(1)?,
                            tool: row.get(2)?,
                            model: row.get(3)?,
                            session: row.get(4)?,
                            dollars: row.get(5)?,
                        },
                    ))
                })
                .map_err(fail)?;
            for row in rows {
                let (hour, billed) = row.map_err(fail)?;
                inputs.billed.entry(hour).or_default().push(billed);
            }
        }
        {
            let mut statement = db
                .prepare(
                    "SELECT s.source, s.through, (SELECT MIN(hour) FROM billed b
                     WHERE b.source = s.source AND b.feed = s.feed) FROM settled s",
                )
                .map_err(fail)?;
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                    ))
                })
                .map_err(fail)?;
            for row in rows {
                let (source, through, first) = row.map_err(fail)?;
                let first = first.unwrap_or(through);
                let entry = inputs.settled.entry(source).or_insert((first, through));
                *entry = (entry.0.min(first), entry.1.max(through));
            }
        }
        inputs.fed = db
            .query_row("SELECT EXISTS (SELECT 1 FROM billed)", [], |row| row.get(0))
            .map_err(fail)?;
        Ok(inputs)
    }

    fn settled(&self, source: &str, hour: i64) -> bool {
        self.settled
            .get(source)
            .is_some_and(|&(first, through)| hour >= first && hour < through)
    }

    /// The list-priced pieces of the window, by model, for learning ratios
    /// against the warehouse's pi spend.
    pub fn learning(&self) -> Vec<(i64, String, f64, f64)> {
        let mut out = Vec::new();
        for (&hour, known) in &self.known {
            if !self.settled("warehouse", hour) {
                continue;
            }
            let mut list: HashMap<&str, f64> = HashMap::new();
            for piece in known {
                *list.entry(&piece.model).or_default() += piece.list;
            }
            let mut billed: HashMap<&str, f64> = HashMap::new();
            for row in self.billed.get(&hour).into_iter().flatten() {
                if row.source == "warehouse" && row.tool == "pi" {
                    *billed.entry(&row.model).or_default() += row.dollars;
                }
            }
            for (model, list) in list {
                out.push((
                    hour,
                    model.to_owned(),
                    list,
                    billed.get(model).copied().unwrap_or(0.0),
                ));
            }
        }
        out
    }

    pub fn pieces(&self, ratios: &Ratios) -> Vec<Piece> {
        let hours: std::collections::BTreeSet<i64> = self
            .known
            .keys()
            .chain(self.billed.keys())
            .copied()
            .collect();
        let mut out = Vec::new();
        for hour in hours {
            self.hour_pieces(hour, ratios, &mut out);
        }
        out
    }

    fn piece(hour: i64, known: &Known, dollars: f64, source: Source) -> Piece {
        Piece {
            hour,
            tool: "pi".into(),
            model: known.model.clone(),
            session: known.session.clone(),
            quest: known.quest.clone(),
            repo: known.repo.clone(),
            kind: known.kind.clone(),
            persona: known.persona.clone(),
            dollars,
            list: known.list,
            source,
        }
    }

    fn other(
        hour: i64,
        tool: &str,
        model: &str,
        session: &str,
        dollars: f64,
        source: Source,
    ) -> Piece {
        Piece {
            hour,
            tool: tool.into(),
            model: model.into(),
            session: session.into(),
            quest: None,
            repo: String::new(),
            kind: tool.into(),
            persona: String::new(),
            dollars,
            list: 0.0,
            source,
        }
    }

    fn hour_pieces(&self, hour: i64, ratios: &Ratios, out: &mut Vec<Piece>) {
        let known: &[Known] = self.known.get(&hour).map_or(&[], Vec::as_slice);
        let billed: &[BilledRow] = self.billed.get(&hour).map_or(&[], Vec::as_slice);
        if self.settled("warehouse", hour) {
            let rows: Vec<&BilledRow> = billed
                .iter()
                .filter(|row| row.source == "warehouse")
                .collect();
            let mut unsplit = 0.0;
            let mut by_session: HashMap<&str, f64> = HashMap::new();
            for row in &rows {
                if row.tool != "pi" {
                    out.push(Self::other(
                        hour,
                        &row.tool,
                        &row.model,
                        "",
                        row.dollars,
                        Source::Warehouse,
                    ));
                } else if row.session.is_empty() {
                    unsplit += row.dollars;
                } else {
                    *by_session.entry(&row.session).or_default() += row.dollars;
                }
            }
            // A session's own bill goes to its pieces by their list shares.
            let mut covered: HashSet<&str> = HashSet::new();
            for (session, dollars) in &by_session {
                let mine: Vec<&Known> = known
                    .iter()
                    .filter(|piece| piece.session == *session)
                    .collect();
                let mine_list: f64 = mine.iter().map(|piece| piece.list).sum();
                if !mine.is_empty() && mine_list > 0.0 {
                    covered.insert(session);
                    for piece in mine {
                        out.push(Self::piece(
                            hour,
                            piece,
                            dollars * piece.list / mine_list,
                            Source::Warehouse,
                        ));
                    }
                } else if let Some(run) = self.subagents.get(*session) {
                    out.push(Self::piece(hour, run, *dollars, Source::Warehouse));
                } else if self.logged.contains(*session) {
                    // Billed in an hour its log has no turn in, near an hour's edge.
                    let model = rows
                        .iter()
                        .find(|row| row.session == *session)
                        .map_or("", |row| &row.model);
                    let mut piece =
                        Self::other(hour, "pi", model, session, *dollars, Source::Warehouse);
                    piece.kind = "agent".into();
                    out.push(piece);
                } else {
                    let model = rows
                        .iter()
                        .find(|row| row.session == *session)
                        .map_or("", |row| &row.model);
                    let mut piece =
                        Self::other(hour, "pi", model, "elsewhere", *dollars, Source::Warehouse);
                    piece.kind = "elsewhere".into();
                    out.push(piece);
                }
            }
            // What came without a split follows the uncovered pieces' shares.
            let rest: Vec<&Known> = known
                .iter()
                .filter(|piece| !covered.contains(piece.session.as_str()))
                .collect();
            let rest_list: f64 = rest.iter().map(|piece| piece.list).sum();
            if unsplit > 0.0 {
                if rest_list > 0.0 {
                    for piece in rest {
                        out.push(Self::piece(
                            hour,
                            piece,
                            unsplit * piece.list / rest_list,
                            Source::Warehouse,
                        ));
                    }
                } else {
                    let mut piece =
                        Self::other(hour, "pi", "", "elsewhere", unsplit, Source::Warehouse);
                    piece.kind = "elsewhere".into();
                    out.push(piece);
                }
            }
            return;
        }
        let meter: f64 = billed
            .iter()
            .filter(|row| row.source == "meter")
            .map(|row| row.dollars)
            .sum();
        let metered = billed.iter().any(|row| row.source == "meter");
        let estimate = |piece: &Known| ratios.of(&piece.model).map(|ratio| piece.list * ratio);
        if metered {
            // The meter is the whole bill; pi's share of it follows the
            // estimate, and what's left is other tools'.
            let pi: f64 = known
                .iter()
                .map(|piece| estimate(piece).unwrap_or(piece.list))
                .sum();
            let scale = if pi > meter && pi > 0.0 {
                meter / pi
            } else {
                1.0
            };
            for piece in known {
                let dollars = estimate(piece).unwrap_or(piece.list) * scale;
                out.push(Self::piece(hour, piece, dollars, Source::Meter));
            }
            if meter > pi {
                out.push(Self::other(
                    hour,
                    "other",
                    "",
                    "",
                    meter - pi,
                    Source::Meter,
                ));
            }
            return;
        }
        for piece in known {
            match (self.fed, estimate(piece)) {
                (true, Some(dollars)) => {
                    out.push(Self::piece(hour, piece, dollars, Source::Estimated))
                }
                _ => out.push(Self::piece(hour, piece, piece.list, Source::List)),
            }
        }
    }
}

/// Dollars summed by some key, largest first.
pub fn by<F: Fn(&Piece) -> String>(pieces: &[Piece], key: F) -> Vec<(String, f64)> {
    let mut sums: HashMap<String, f64> = HashMap::new();
    for piece in pieces {
        *sums.entry(key(piece)).or_default() += piece.dollars;
    }
    let mut out: Vec<(String, f64)> = sums.into_iter().collect();
    out.sort_by(|one, other| other.1.total_cmp(&one.1).then_with(|| one.0.cmp(&other.0)));
    out
}

pub fn total(pieces: &[Piece]) -> f64 {
    pieces.iter().map(|piece| piece.dollars).sum()
}

/// Whether any of a window's dollars are estimated rather than billed.
pub fn estimated(pieces: &[Piece]) -> bool {
    pieces
        .iter()
        .any(|piece| piece.source.estimated() && piece.dollars > 0.005)
}

/// The worst source among a window's pieces, by how far it is from a bill.
pub fn weakest(pieces: &[Piece]) -> Option<Source> {
    pieces
        .iter()
        .filter(|piece| piece.dollars > 0.005)
        .map(|piece| piece.source)
        .max()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feed;

    fn history() -> History {
        History::memory()
    }

    fn turn(
        history: &History,
        session: &str,
        hour: i64,
        model: &str,
        cost: f64,
        quest: Option<&str>,
    ) {
        history
            .db
            .execute(
                "INSERT INTO turns VALUES (?1, ?2, ?3, ?4, 'anthropic', 'agent', ?5,
                 0, 0, 0, 0, 0, ?6, 0, 0, NULL, 0, 0, NULL)",
                rusqlite::params![session, hour as f64 * 3600.0, hour, model, quest, cost],
            )
            .unwrap();
        history
            .db
            .execute(
                "INSERT OR IGNORE INTO sessions VALUES
                 (?1, '/src/github.com/me/thing', 'github.com/me/thing', 0)",
                [session],
            )
            .unwrap();
    }

    fn fed(history: &History, line: &str) {
        feed::store(history, "proxy", &feed::parse(line).unwrap(), 0.0).unwrap();
    }

    fn hour_text(hour: i64) -> String {
        crate::time::rfc3339(hour as f64 * 3600.0)
    }

    fn billed(
        history: &History,
        source: &str,
        hour: i64,
        tool: &str,
        model: &str,
        session: Option<&str>,
        dollars: f64,
    ) {
        let line = serde_json::json!({
            "v": 1, "kind": "billed", "source": source, "hour": hour_text(hour), "tool": tool,
            "model": model, "session": session, "dollars": dollars,
        });
        fed(history, &line.to_string());
    }

    fn settle(history: &History, through: i64) {
        fed(
            history,
            &format!(
                r#"{{"v":1,"kind":"settled","source":"warehouse","through":"{}"}}"#,
                hour_text(through)
            ),
        );
    }

    const H: i64 = 490_000;

    #[test]
    fn logs_alone_are_list_prices() {
        let history = history();
        turn(&history, "s1", H, "opus", 10.0, Some("Q1"));
        let pieces = Inputs::read(&history, H, H + 1)
            .unwrap()
            .pieces(&Ratios::default());
        assert_eq!(total(&pieces), 10.0);
        assert_eq!(weakest(&pieces), Some(Source::List));
    }

    #[test]
    fn a_settled_hour_is_the_bill_split_by_session() {
        let history = history();
        turn(&history, "s1", H, "opus", 10.0, Some("Q1"));
        turn(&history, "s2", H, "opus", 30.0, Some("Q2"));
        billed(&history, "warehouse", H, "pi", "opus", Some("s1"), 8.0);
        billed(&history, "warehouse", H, "pi", "opus", Some("s2"), 25.0);
        billed(&history, "warehouse", H, "pi", "opus", Some("gone"), 3.0);
        billed(&history, "warehouse", H, "river", "opus", None, 1.0);
        settle(&history, H + 1);
        let pieces = Inputs::read(&history, H, H + 1)
            .unwrap()
            .pieces(&Ratios::default());
        assert_eq!(total(&pieces), 37.0);
        let quests = by(&pieces, |piece| piece.quest.clone().unwrap_or_default());
        assert_eq!(quests[0], ("Q2".into(), 25.0));
        assert!(
            pieces
                .iter()
                .any(|piece| piece.session == "elsewhere" && piece.dollars == 3.0)
        );
        assert!(!estimated(&pieces));
    }

    #[test]
    fn an_unsplit_bill_follows_the_logs_shares() {
        let history = history();
        turn(&history, "s1", H, "opus", 10.0, Some("Q1"));
        turn(&history, "s2", H, "opus", 30.0, Some("Q2"));
        billed(&history, "warehouse", H, "pi", "opus", None, 20.0);
        settle(&history, H + 1);
        let pieces = Inputs::read(&history, H, H + 1)
            .unwrap()
            .pieces(&Ratios::default());
        let quests = by(&pieces, |piece| piece.quest.clone().unwrap_or_default());
        assert_eq!(quests, vec![("Q2".into(), 15.0), ("Q1".into(), 5.0)]);
    }

    #[test]
    fn a_live_hour_is_the_meter_with_the_rest_other_tools() {
        let history = history();
        turn(&history, "s1", H, "opus", 10.0, None);
        billed(&history, "meter", H, "all", "opus", None, 12.0);
        let ratios = Ratios::fixed(&[("opus", 0.85)]);
        let pieces = Inputs::read(&history, H, H + 1).unwrap().pieces(&ratios);
        assert!((total(&pieces) - 12.0).abs() < 1e-9);
        let tools = by(&pieces, |piece| piece.tool.clone());
        assert!((tools[0].1 - 8.5).abs() < 1e-9 && tools[0].0 == "pi");
        assert_eq!(weakest(&pieces), Some(Source::Meter));
    }

    #[test]
    fn after_the_bill_the_logs_are_estimated_by_the_ratio_learned() {
        let history = history();
        turn(&history, "s1", H, "opus", 10.0, None);
        billed(&history, "warehouse", H - 10, "pi", "opus", None, 1.0);
        settle(&history, H - 9);
        let ratios = Ratios::fixed(&[("opus", 0.8)]);
        let pieces = Inputs::read(&history, H, H + 1).unwrap().pieces(&ratios);
        assert!((total(&pieces) - 8.0).abs() < 1e-9);
        assert_eq!(weakest(&pieces), Some(Source::Estimated));
    }
}
