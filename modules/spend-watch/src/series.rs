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
//!
//! An agent's session is its own agent: a run's (a council's reviewer, a
//! judge) is shown as that run's work, under the session that started it
//! and in its quest, at the hours its log has turns in, and the run's own
//! total counts only what no log of it covers; a swarm's is its kind's,
//! named by its session.
//!
//! Every piece draws on a budget (budgets.rs): a turn's provider's, else
//! its log's, else, for a run's agent whose log says none, the budget its
//! session drew on at that hour by its log, else the default. Each
//! budget's hour is priced on its own: a feed's bill for a budget is
//! shared only among that budget's pieces, a budget no feed bills is
//! estimated from pi's logs, and a subscription's turns cost nothing more
//! than its price, their list prices kept as what they're worth.

use crate::budgets::Budgets;
use crate::estimate::Ratios;

use crate::history::{History, sql as fail};
use rusqlite::Connection;
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
    /// The budget it drew on, by name, declared or not.
    pub budget: String,
    pub dollars: f64,
    pub list: f64,
    pub source: Source,
}

/// What pi's side knows of one hour: list-priced pieces from the logs and
/// the runs.
#[derive(Debug, Clone, PartialEq)]
struct Known {
    /// The session it's shown under: the one that started a run, for a
    /// run's agents.
    session: String,
    /// The session that spent it, which a bill names.
    own: String,
    quest: Option<String>,
    repo: String,
    model: String,
    kind: String,
    persona: String,
    /// The provider its turns went to, which may claim their budget.
    provider: String,
    /// What its log said it drew on: none where nothing did, empty for
    /// the default.
    logged: Option<String>,
    budget: String,
    list: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct BilledRow {
    source: String,
    tool: String,
    model: String,
    session: String,
    /// The budget it was drawn from: the record's, else its feed's.
    budget: String,
    dollars: f64,
}

/// What the history knows about one window, read once.
#[derive(Debug)]
pub struct Inputs {
    budgets: Budgets,
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
    /// The budgets a feed bills: each feed's own, and any a record named.
    fed_budgets: HashSet<String>,
}

/// Each session's repo, by its id.
type Repos = HashMap<String, String>;
/// An agent's session's kind and name, by its id.
type Agents = HashMap<String, (String, String)>;
/// The quest a session served in an hour, by the hour and the session.
type QuestAt = HashMap<(i64, String), Option<String>>;
/// What each session's log said it drew on in each hour it spent, by the
/// session: the costliest word of the hour, beside its cost.
type LoggedAt = HashMap<String, BTreeMap<i64, (f64, Option<String>)>>;
/// Pieces pi's side knows, each by its hour.
type ByHour = Vec<(i64, Known)>;

impl Inputs {
    pub fn read(
        history: &History,
        budgets: &Budgets,
        from: i64,
        to: i64,
    ) -> Result<Inputs, String> {
        let db = &history.db;
        let mut inputs = Inputs {
            budgets: budgets.clone(),
            known: BTreeMap::new(),
            billed: BTreeMap::new(),
            settled: HashMap::new(),
            logged: HashSet::new(),
            subagents: HashMap::new(),
            fed: false,
            fed_budgets: HashSet::new(),
        };
        let (repos, agents) = inputs.read_sessions(db)?;
        let (quest_at, logged_at, turns) = inputs.read_turns(db, from, to, &repos)?;
        inputs.read_runs(db, (from, to), &repos, &quest_at, &logged_at)?;
        for (hour, turn) in turns {
            let known = inputs.labelled(hour, turn, &agents, &logged_at);
            inputs.known.entry(hour).or_default().push(known);
        }
        // Each hour's are by budget, so each budget's are a run of them.
        for known in inputs.known.values_mut() {
            known.sort_by(|one, other| one.budget.cmp(&other.budget));
        }
        inputs.read_billed(db, from, to)?;
        inputs.read_fed_budgets(db, &budgets.of_feeds())?;
        inputs.read_settled(db)?;
        inputs.fed = db
            .query_row("SELECT EXISTS (SELECT 1 FROM billed)", [], |row| row.get(0))
            .map_err(fail)?;
        Ok(inputs)
    }

    /// Every session a log here knows, with its repo, and an agent's kind
    /// and name, by where its log was found.
    fn read_sessions(&mut self, db: &Connection) -> Result<(Repos, Agents), String> {
        let mut repos = Repos::new();
        let mut agents = Agents::new();
        let mut statement = db
            .prepare("SELECT id, repo, kind, name FROM sessions")
            .map_err(fail)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .map_err(fail)?;
        for row in rows {
            let (id, repo, kind, name) = row.map_err(fail)?;
            self.logged.insert(id.clone());
            if let Some(kind) = kind {
                agents.insert(id.clone(), (kind, name.unwrap_or_default()));
            }
            repos.insert(id, repo);
        }
        Ok((repos, agents))
    }

    /// Each hour's turns, and the quest a session served and the budget
    /// its log said in each hour, for its subagents.
    fn read_turns(
        &self,
        db: &Connection,
        from: i64,
        to: i64,
        repos: &Repos,
    ) -> Result<(QuestAt, LoggedAt, ByHour), String> {
        let mut quest_at = QuestAt::new();
        let mut logged_at = LoggedAt::new();
        let mut turns = ByHour::new();
        let mut statement = db
            .prepare(
                "SELECT hour, session, quest, model, kind, provider, budget, cost FROM turn_hours
                 WHERE hour >= ?1 AND hour < ?2
                 ORDER BY hour, session, quest, model, kind, provider, budget",
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
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, f64>(7)?,
                ))
            })
            .map_err(fail)?;
        for row in rows {
            let (hour, session, quest, model, kind, provider, logged, list) = row.map_err(fail)?;
            quest_at.insert((hour, session.clone()), quest.clone());
            let said = logged_at.entry(session.clone()).or_default().entry(hour);
            let said = said.or_insert((f64::NEG_INFINITY, None));
            if list > said.0 {
                *said = (list, logged.clone());
            }
            turns.push((
                hour,
                Known {
                    repo: repos.get(&session).cloned().unwrap_or_default(),
                    own: session.clone(),
                    session,
                    quest,
                    model,
                    kind,
                    persona: String::new(),
                    budget: self
                        .budgets
                        .of_turn(&provider, logged.as_deref())
                        .unwrap_or_else(|| self.budgets.default.clone()),
                    provider,
                    logged,
                    list,
                },
            ));
        }
        Ok((quest_at, logged_at, turns))
    }

    /// The budget a session drew on by its log at an hour: what it said
    /// that hour, or at the last hour before that it spent in, or the
    /// default.
    fn session_budget(&self, logged_at: &LoggedAt, session: &str, hour: i64) -> String {
        logged_at
            .get(session)
            .and_then(|hours| hours.range(..=hour).next_back())
            .and_then(|(_, (_, logged))| logged.as_deref())
            .map_or_else(
                || self.budgets.default.clone(),
                |logged| self.budgets.named(logged),
            )
    }

    /// Each run of subagents: the sessions that were its agents, and, at
    /// the hour it started, what of its cost no log of it covers.
    fn read_runs(
        &mut self,
        db: &Connection,
        (from, to): (i64, i64),
        repos: &Repos,
        quest_at: &QuestAt,
        logged_at: &LoggedAt,
    ) -> Result<(), String> {
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
                budget: self.session_budget(logged_at, &parent, hour),
                own: parent.clone(),
                session: parent,
                model,
                kind: format!("subagent:{kind}"),
                persona,
                provider: String::new(),
                logged: None,
                list: cost,
            };
            let ids: Vec<String> = serde_json::from_str(&sessions).unwrap_or_default();
            for id in &ids {
                self.subagents.insert(
                    id.clone(),
                    Known {
                        own: id.clone(),
                        list: 0.0,
                        ..known.clone()
                    },
                );
            }
            if !(from..to).contains(&hour) {
                continue;
            }
            // What the run's logs hold is counted at their own hours.
            let mut logged = 0.0;
            for id in ids.iter().filter(|id| self.logged.contains(*id)) {
                logged += db
                    .query_row(
                        "SELECT COALESCE(SUM(cost), 0) FROM turns WHERE session = ?1",
                        [id],
                        |row| row.get::<_, f64>(0),
                    )
                    .map_err(fail)?;
            }
            let rest = known.list - logged;
            if rest > 1e-9 {
                self.known.entry(hour).or_default().push(Known {
                    list: rest,
                    ..known
                });
            }
        }
        Ok(())
    }

    /// What each hour billed, by source, tool, model, session and budget,
    /// each hour's by budget.
    fn read_billed(&mut self, db: &Connection, from: i64, to: i64) -> Result<(), String> {
        let mut statement = db
            .prepare(
                "SELECT hour, source, tool, model, session, feed, budget, SUM(dollars) FROM billed
                 WHERE hour >= ?1 AND hour < ?2
                 GROUP BY hour, source, tool, model, session, feed, budget",
            )
            .map_err(fail)?;
        let rows = statement
            .query_map([from, to], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(5)?,
                    BilledRow {
                        source: row.get(1)?,
                        tool: row.get(2)?,
                        model: row.get(3)?,
                        session: row.get(4)?,
                        budget: row.get(6)?,
                        dollars: row.get(7)?,
                    },
                ))
            })
            .map_err(fail)?;
        for row in rows {
            let (hour, feed, mut billed) = row.map_err(fail)?;
            if billed.budget.is_empty() {
                self.budgets.of_feed(&feed).clone_into(&mut billed.budget);
            }
            self.billed.entry(hour).or_default().push(billed);
        }
        for billed in self.billed.values_mut() {
            billed.sort_by(|one, other| one.budget.cmp(&other.budget));
        }
        Ok(())
    }

    /// The budgets a feed bills: each configured feed's own, and every
    /// budget a feed's record has named or a feed no longer configured
    /// billed.
    fn read_fed_budgets(&mut self, db: &Connection, feeds: &[String]) -> Result<(), String> {
        let mut statement = db
            .prepare("SELECT DISTINCT feed, budget FROM billed")
            .map_err(fail)?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(fail)?;
        for row in rows {
            let (feed, budget) = row.map_err(fail)?;
            let budget = if budget.is_empty() {
                self.budgets.of_feed(&feed).to_owned()
            } else {
                budget
            };
            self.fed_budgets.insert(budget);
        }
        self.fed_budgets.extend(feeds.iter().cloned());
        Ok(())
    }

    /// Each source's settled hours, from the first it billed.
    fn read_settled(&mut self, db: &Connection) -> Result<(), String> {
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
            let entry = self.settled.entry(source).or_insert((first, through));
            *entry = (entry.0.min(first), entry.1.max(through));
        }
        Ok(())
    }

    /// A turn as the work it was part of: a run's, shown as the run's under
    /// the session that started it, drawing on that session's budget at
    /// the hour where neither its provider nor its log says another; an
    /// agent's, as its kind's and named. A compaction or an extension's
    /// call stays what it is.
    fn labelled(
        &self,
        hour: i64,
        turn: Known,
        agents: &HashMap<String, (String, String)>,
        logged_at: &LoggedAt,
    ) -> Known {
        let agent = turn.kind == "agent";
        if let Some(run) = self.subagents.get(&turn.own) {
            let budget = self
                .budgets
                .of_turn(&turn.provider, turn.logged.as_deref())
                .unwrap_or_else(|| self.session_budget(logged_at, &run.session, hour));
            return Known {
                session: run.session.clone(),
                quest: turn.quest.or_else(|| run.quest.clone()),
                repo: run.repo.clone(),
                kind: if agent { run.kind.clone() } else { turn.kind },
                persona: run.persona.clone(),
                budget,
                ..turn
            };
        }
        match agents.get(&turn.own) {
            Some((kind, name)) => Known {
                kind: if agent {
                    format!("subagent:{kind}")
                } else {
                    turn.kind
                },
                persona: name.clone(),
                ..turn
            },
            None => turn,
        }
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
            // Only what a feed bills is learned from: a budget it doesn't
            // isn't in its bill.
            let mut list: HashMap<&str, f64> = HashMap::new();
            for piece in known
                .iter()
                .filter(|piece| self.fed_budgets.contains(&piece.budget))
            {
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
            budget: known.budget.clone(),
            dollars,
            list: known.list,
            source,
        }
    }

    /// Spend no log here knows the turns of: another tool's, a session's
    /// elsewhere, or what a bill holds past pi's.
    fn other(
        hour: i64,
        tool: &str,
        model: &str,
        session: &str,
        budget: &str,
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
            budget: budget.into(),
            dollars,
            list: 0.0,
            source,
        }
    }

    /// An hour the meter read: the meter is the whole bill, by model. pi's
    /// pieces on a model take their estimate, scaled to the model's figure
    /// when they'd pass it, and the rest of each model's is other tools'
    /// on it. Pieces on a model the meter doesn't name, as when it names a
    /// model differently, are drawn evenly from what's left over.
    fn metered(
        hour: i64,
        budget: &str,
        known: &[Known],
        meter: &BTreeMap<&str, f64>,
        estimate: impl Fn(&Known) -> Option<f64>,
        out: &mut Vec<Piece>,
    ) {
        let dollars = |piece: &Known| estimate(piece).unwrap_or(piece.list);
        let mut pi: HashMap<&str, f64> = HashMap::new();
        for piece in known {
            *pi.entry(piece.model.as_str()).or_default() += dollars(piece);
        }
        let mut left: Vec<(&str, f64)> = Vec::new();
        for (&model, &figure) in meter {
            let logged = pi.get(model).copied().unwrap_or(0.0);
            if figure > logged {
                left.push((model, figure - logged));
            }
        }
        let unnamed: f64 = known
            .iter()
            .filter(|piece| !meter.contains_key(piece.model.as_str()))
            .map(dollars)
            .sum();
        let over: f64 = left.iter().map(|(_, dollars)| dollars).sum();
        // How much of each piece on an unnamed model is kept, and how much
        // of what's left over stays other tools'.
        let (kept, stays) = if unnamed <= 0.0 {
            (1.0, 1.0)
        } else if unnamed <= over {
            (1.0, (over - unnamed) / over)
        } else {
            (over / unnamed, 0.0)
        };
        for piece in known {
            let scale = match meter.get(piece.model.as_str()) {
                Some(&figure) => {
                    let logged = pi[piece.model.as_str()];
                    if logged > figure && logged > 0.0 {
                        figure / logged
                    } else {
                        1.0
                    }
                }
                None => kept,
            };
            out.push(Self::piece(
                hour,
                piece,
                dollars(piece) * scale,
                Source::Meter,
            ));
        }
        for (model, dollars) in left {
            if dollars * stays > 0.0 {
                out.push(Self::other(
                    hour,
                    "other",
                    model,
                    "",
                    budget,
                    dollars * stays,
                    Source::Meter,
                ));
            }
        }
    }

    /// A settled hour's pieces, from the warehouse's bill: another tool's
    /// as its own, a session's to its pieces, and what came without a split
    /// to the pieces no session's bill covered.
    fn warehouse_pieces(
        &self,
        hour: i64,
        budget: &str,
        known: &[Known],
        billed: &[BilledRow],
        out: &mut Vec<Piece>,
    ) {
        let rows: Vec<&BilledRow> = billed
            .iter()
            .filter(|row| row.source == "warehouse")
            .collect();
        let mut unsplit: HashMap<&str, f64> = HashMap::new();
        let mut by_session: HashMap<&str, f64> = HashMap::new();
        for row in &rows {
            if row.tool != "pi" {
                out.push(Self::other(
                    hour,
                    &row.tool,
                    &row.model,
                    "",
                    budget,
                    row.dollars,
                    Source::Warehouse,
                ));
            } else if row.session.is_empty() {
                *unsplit.entry(&row.model).or_default() += row.dollars;
            } else {
                *by_session.entry(&row.session).or_default() += row.dollars;
            }
        }
        // A session's own bill goes to its pieces by their list shares.
        let mut covered: HashSet<&str> = HashSet::new();
        for (session, dollars) in &by_session {
            let mine: Vec<&Known> = known.iter().filter(|piece| piece.own == *session).collect();
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
                // The bill says whose budget it drew on, whatever its run's.
                out.push(Piece {
                    budget: budget.into(),
                    ..Self::piece(hour, run, *dollars, Source::Warehouse)
                });
            } else if self.logged.contains(*session) {
                // Billed in an hour its log has no turn in, near an hour's edge.
                let model = rows
                    .iter()
                    .find(|row| row.session == *session)
                    .map_or("", |row| &row.model);
                out.push(Self::billed_alone(
                    hour, model, session, "agent", budget, *dollars,
                ));
            } else {
                let model = rows
                    .iter()
                    .find(|row| row.session == *session)
                    .map_or("", |row| &row.model);
                out.push(Self::elsewhere(hour, model, budget, *dollars));
            }
        }
        // What came without a split follows the shares of the uncovered
        // pieces on its model, and where no log names that model, every
        // uncovered piece's: a model's price differs, so its bill is its
        // own pieces'.
        let rest: Vec<&Known> = known
            .iter()
            .filter(|piece| !covered.contains(piece.own.as_str()))
            .collect();
        let mut unsplit: Vec<(&str, f64)> = unsplit.into_iter().collect();
        unsplit.sort_by(|one, other| one.0.cmp(other.0));
        for (model, dollars) in unsplit.into_iter().filter(|(_, dollars)| *dollars > 0.0) {
            let on_model: Vec<&Known> = rest
                .iter()
                .copied()
                .filter(|piece| piece.model == model && piece.list > 0.0)
                .collect();
            let shared = if on_model.is_empty() {
                &rest
            } else {
                &on_model
            };
            let list: f64 = shared.iter().map(|piece| piece.list).sum();
            if list > 0.0 {
                for piece in shared {
                    out.push(Self::piece(
                        hour,
                        piece,
                        dollars * piece.list / list,
                        Source::Warehouse,
                    ));
                }
            } else {
                out.push(Self::elsewhere(hour, model, budget, dollars));
            }
        }
    }

    /// pi's spend the warehouse billed that no log here has turns for.
    fn billed_alone(
        hour: i64,
        model: &str,
        session: &str,
        kind: &str,
        budget: &str,
        dollars: f64,
    ) -> Piece {
        Piece {
            kind: kind.into(),
            ..Self::other(
                hour,
                "pi",
                model,
                session,
                budget,
                dollars,
                Source::Warehouse,
            )
        }
    }

    /// pi's spend no log here knows: another machine's, or a one-shot run's.
    fn elsewhere(hour: i64, model: &str, budget: &str, dollars: f64) -> Piece {
        Self::billed_alone(hour, model, "elsewhere", "elsewhere", budget, dollars)
    }

    /// An hour's pieces, each budget's priced on its own.
    fn hour_pieces(&self, hour: i64, ratios: &Ratios, out: &mut Vec<Piece>) {
        let known: &[Known] = self.known.get(&hour).map_or(&[], Vec::as_slice);
        let billed: &[BilledRow] = self.billed.get(&hour).map_or(&[], Vec::as_slice);
        let mut budgets: Vec<&str> = known
            .iter()
            .map(|piece| piece.budget.as_str())
            .chain(billed.iter().map(|row| row.budget.as_str()))
            .collect();
        budgets.sort_unstable();
        budgets.dedup();
        for budget in budgets {
            let known = of_budget(known, budget, |piece| &piece.budget);
            let billed = of_budget(billed, budget, |row| &row.budget);
            self.budget_pieces(hour, budget, known, billed, ratios, out);
        }
    }

    /// One budget's pieces in an hour: from a feed's bill for it where
    /// there is one, else estimated from pi's logs, and nothing for a
    /// subscription's turns no bill counts.
    fn budget_pieces(
        &self,
        hour: i64,
        budget: &str,
        known: &[Known],
        billed: &[BilledRow],
        ratios: &Ratios,
        out: &mut Vec<Piece>,
    ) {
        if self.fed_budgets.contains(budget) && self.settled("warehouse", hour) {
            self.warehouse_pieces(hour, budget, known, billed, out);
            return;
        }
        let mut meter: BTreeMap<&str, f64> = BTreeMap::new();
        for row in billed.iter().filter(|row| row.source == "meter") {
            *meter.entry(&row.model).or_default() += row.dollars;
        }
        let estimate = |piece: &Known| ratios.of(&piece.model).map(|ratio| piece.list * ratio);
        if !meter.is_empty() {
            Self::metered(hour, budget, known, &meter, estimate, out);
            return;
        }
        if self.budgets.subscription(budget) {
            out.extend(
                known
                    .iter()
                    .map(|piece| Self::piece(hour, piece, 0.0, Source::List)),
            );
            return;
        }
        for piece in known {
            match (self.fed, estimate(piece)) {
                (true, Some(dollars)) => {
                    out.push(Self::piece(hour, piece, dollars, Source::Estimated));
                }
                _ => out.push(Self::piece(hour, piece, piece.list, Source::List)),
            }
        }
    }
}

/// The run of `items`, sorted by budget, that are `budget`'s.
fn of_budget<'a, T>(items: &'a [T], budget: &str, of: impl Fn(&T) -> &String) -> &'a [T] {
    let first = items.partition_point(|item| of(item).as_str() < budget);
    let last = items.partition_point(|item| of(item).as_str() <= budget);
    &items[first..last]
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

    fn read(history: &History, from: i64, to: i64) -> Result<Inputs, String> {
        Inputs::read(history, &Budgets::implicit(), from, to)
    }

    /// A proxy's cap the default, a Claude plan claiming its provider, and
    /// a team's token.
    fn declared() -> Budgets {
        let config: crate::config::Config = serde_json::from_value(serde_json::json!({
            "sessions": "/", "runs": "/", "registry": "/", "quests": "/", "state": "/",
            "budgets": [
                {"name": "proxy", "label": "Your proxy cap", "kind": "cap",
                 "whose": "employer", "default": true},
                {"name": "claude", "label": "Claude Pro", "kind": "subscription",
                 "whose": "own", "providers": ["claude-code"]},
                {"name": "delta", "label": "Delta team token", "kind": "team", "whose": "team"},
            ],
        }))
        .unwrap();
        Budgets::of(&config)
    }

    /// A turn on a provider, under what its log said of its budget.
    fn budgeted(
        history: &History,
        session: &str,
        hour: i64,
        provider: &str,
        logged: Option<&str>,
        cost: f64,
    ) {
        history
            .db
            .execute(
                "INSERT INTO turns VALUES (?1, ?2, ?3, 'opus', ?4, 'agent', NULL,
                 0, 0, 0, 0, 0, ?5, 0, 0, NULL, 0, 0, NULL, ?6)",
                rusqlite::params![
                    session,
                    crate::time::hour_start(hour),
                    hour,
                    provider,
                    cost,
                    logged
                ],
            )
            .unwrap();
        history
            .db
            .execute(
                "INSERT OR IGNORE INTO sessions (id, cwd, repo, started) VALUES (?1, '/x', 'x', 0)",
                [session],
            )
            .unwrap();
    }

    fn by_budget(pieces: &[Piece]) -> Vec<(String, f64)> {
        let mut sums = by(pieces, |piece| piece.budget.clone());
        sums.sort_by(|one, other| one.0.cmp(&other.0));
        sums
    }

    // A Claude plan's provider spends the plan whatever the log says, and
    // costs nothing past its price, though its list price is kept as what
    // it was worth; the log names the rest, its word for none being the
    // default, and a name nobody declared still counts, so the budgets sum
    // to the whole.
    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "whole-dollar list prices pass through untouched"
    )]
    fn each_turn_draws_on_its_provider_s_budget_then_its_log_s() {
        let history = history();
        budgeted(&history, "s1", H, "claude-code", Some("delta"), 1.0);
        budgeted(&history, "s2", H, "anthropic", Some("delta"), 2.0);
        budgeted(&history, "s3", H, "anthropic", Some(""), 4.0);
        budgeted(&history, "s4", H, "anthropic", None, 8.0);
        budgeted(&history, "s5", H, "anthropic", Some("probe"), 16.0);
        let pieces = Inputs::read(&history, &declared(), H, H + 1)
            .unwrap()
            .pieces(&Ratios::default());
        assert_eq!(
            by_budget(&pieces),
            [
                ("claude".into(), 0.0),
                ("delta".into(), 2.0),
                ("probe".into(), 16.0),
                ("proxy".into(), 12.0),
            ]
        );
        assert_eq!(total(&pieces), 30.0);
        let plan: f64 = pieces
            .iter()
            .filter(|p| p.budget == "claude")
            .map(|p| p.list)
            .sum();
        assert_eq!(plan, 1.0);
    }

    fn fed_declared() -> Budgets {
        let config: crate::config::Config = serde_json::from_value(serde_json::json!({
            "sessions": "/", "runs": "/", "registry": "/", "quests": "/", "state": "/",
            "budgets": [
                {"name": "proxy", "label": "Your proxy cap", "kind": "cap",
                 "whose": "employer", "default": true},
                {"name": "delta", "label": "Delta team token", "kind": "team", "whose": "team"},
            ],
            "feeds": [{"name": "proxy", "label": "the AI proxy", "command": ["f"]}],
        }))
        .unwrap();
        Budgets::of(&config)
    }

    // A bill that doesn't say its session is shared only among the pieces
    // of the budget it's for, the feed's own unless the record names one.
    #[test]
    fn an_unsplit_bill_is_shared_within_its_budget() {
        let history = history();
        budgeted(&history, "me", H, "anthropic", Some(""), 10.0);
        budgeted(&history, "team", H, "anthropic", Some("delta"), 30.0);
        billed(&history, "warehouse", H, "pi", "opus", None, 20.0);
        let line = serde_json::json!({
            "v": 1, "kind": "billed", "source": "warehouse", "hour": hour_text(H),
            "tool": "pi", "model": "opus", "budget": "delta", "dollars": 15.0,
        });
        fed(&history, &line.to_string());
        settle(&history, H + 1);
        let pieces = Inputs::read(&history, &fed_declared(), H, H + 1)
            .unwrap()
            .pieces(&Ratios::default());
        assert_eq!(
            by_budget(&pieces),
            [("delta".into(), 15.0), ("proxy".into(), 20.0)]
        );
    }

    // A budget no feed bills is priced from pi's logs, and the feed's bill
    // is still its own budget's alone.
    #[test]
    fn a_budget_no_feed_bills_is_estimated_from_the_logs() {
        let history = history();
        budgeted(&history, "me", H, "anthropic", Some(""), 10.0);
        budgeted(&history, "team", H, "anthropic", Some("delta"), 30.0);
        billed(&history, "warehouse", H, "pi", "opus", None, 20.0);
        settle(&history, H + 1);
        let ratios = Ratios::fixed(&[("opus", 0.5)]);
        let pieces = Inputs::read(&history, &fed_declared(), H, H + 1)
            .unwrap()
            .pieces(&ratios);
        assert_eq!(
            by_budget(&pieces),
            [("delta".into(), 15.0), ("proxy".into(), 20.0)]
        );
        let delta = pieces.iter().find(|piece| piece.budget == "delta").unwrap();
        assert_eq!(delta.source, Source::Estimated);
    }

    // A council's reviewer whose log says nothing draws on what the session
    // that asked for it drew on at that hour, and the run's total beyond
    // its logs on what the session drew on as it started.
    #[test]
    fn a_run_s_agent_draws_on_its_session_s_budget() {
        let history = history();
        budgeted(&history, "me", H, "anthropic", Some("delta"), 10.0);
        budgeted(&history, "me", H + 2, "anthropic", Some(""), 10.0);
        budgeted(&history, "r1", H + 1, "anthropic", None, 4.0);
        budgeted(&history, "r1", H + 2, "anthropic", None, 2.0);
        budgeted(&history, "r2", H + 1, "anthropic", Some("aquifer"), 1.0);
        run(&history, "council", "me", H, 9.0, &["r1", "r2"]);
        let pieces = Inputs::read(&history, &declared(), H, H + 3)
            .unwrap()
            .pieces(&Ratios::default());
        let at = |hour: i64| -> Vec<(String, f64)> {
            let pieces: Vec<Piece> = pieces.iter().filter(|p| p.hour == hour).cloned().collect();
            by_budget(&pieces)
        };
        // The run's $9 less its logs' $7 stands at its start, as delta's.
        assert_eq!(at(H), [("delta".into(), 12.0)]);
        assert_eq!(at(H + 1), [("aquifer".into(), 1.0), ("delta".into(), 4.0)]);
        assert_eq!(at(H + 2), [("proxy".into(), 12.0)]);
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
                 0, 0, 0, 0, 0, ?6, 0, 0, NULL, 0, 0, NULL, NULL)",
                rusqlite::params![
                    session,
                    crate::time::hour_start(hour),
                    hour,
                    model,
                    quest,
                    cost
                ],
            )
            .unwrap();
        history
            .db
            .execute(
                "INSERT OR IGNORE INTO sessions (id, cwd, repo, started) VALUES
                 (?1, '/src/github.com/me/thing', 'github.com/me/thing', 0)",
                [session],
            )
            .unwrap();
    }

    fn fed(history: &History, line: &str) {
        feed::store(history, "proxy", &feed::parse(line).unwrap(), 0.0).unwrap();
    }

    fn hour_text(hour: i64) -> String {
        crate::time::rfc3339(crate::time::hour_start(hour))
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
    #[expect(
        clippy::float_cmp,
        reason = "one turn's whole-dollar list price is passed through untouched"
    )]
    fn logs_alone_are_list_prices() {
        let history = history();
        turn(&history, "s1", H, "opus", 10.0, Some("Q1"));
        let pieces = read(&history, H, H + 1).unwrap().pieces(&Ratios::default());
        assert_eq!(total(&pieces), 10.0);
        assert_eq!(weakest(&pieces), Some(Source::List));
    }

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "each session's bill goes whole to its one piece, so the sums are of whole \
                  dollars and exact"
    )]
    fn a_settled_hour_is_the_bill_split_by_session() {
        let history = history();
        turn(&history, "s1", H, "opus", 10.0, Some("Q1"));
        turn(&history, "s2", H, "opus", 30.0, Some("Q2"));
        billed(&history, "warehouse", H, "pi", "opus", Some("s1"), 8.0);
        billed(&history, "warehouse", H, "pi", "opus", Some("s2"), 25.0);
        billed(&history, "warehouse", H, "pi", "opus", Some("gone"), 3.0);
        billed(&history, "warehouse", H, "river", "opus", None, 1.0);
        settle(&history, H + 1);
        let pieces = read(&history, H, H + 1).unwrap().pieces(&Ratios::default());
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
        let pieces = read(&history, H, H + 1).unwrap().pieces(&Ratios::default());
        let quests = by(&pieces, |piece| piece.quest.clone().unwrap_or_default());
        assert_eq!(quests, vec![("Q2".into(), 15.0), ("Q1".into(), 5.0)]);
    }

    /// A run started from `parent` at `hour`, its agents' sessions `ids`.
    fn run(history: &History, kind: &str, parent: &str, hour: i64, cost: f64, ids: &[&str]) {
        history
            .db
            .execute(
                "INSERT INTO runs VALUES (?1, 'a1', ?2, 'opus', 'critic', ?3, ?4, NULL, ?5,
                 0, 0, 0, 0, ?6)",
                rusqlite::params![
                    format!("{kind}-1"),
                    kind,
                    parent,
                    crate::time::hour_start(hour),
                    cost,
                    serde_json::to_string(ids).unwrap()
                ],
            )
            .unwrap();
    }

    // A council whose reviewers keep logs counts once, at the hours they
    // worked, as the council's work under the session that asked for it.
    #[test]
    fn a_logged_council_is_counted_once_as_its_work() {
        let history = history();
        turn(&history, "me", H, "opus", 10.0, Some("Q1"));
        turn(&history, "r1", H + 1, "opus", 4.0, None);
        run(&history, "council", "me", H, 5.0, &["r1", "r2"]);
        let pieces = read(&history, H, H + 2).unwrap().pieces(&Ratios::default());
        assert!((total(&pieces) - 15.0).abs() < 1e-9, "{pieces:?}");
        let reviewer: Vec<&Piece> = pieces.iter().filter(|p| p.hour == H + 1).collect();
        assert_eq!(reviewer.len(), 1);
        assert_eq!(
            (
                reviewer[0].kind.as_str(),
                reviewer[0].persona.as_str(),
                reviewer[0].session.as_str(),
                reviewer[0].quest.as_deref(),
            ),
            ("subagent:council", "critic", "me", Some("Q1"))
        );
        let lump: f64 = pieces
            .iter()
            .filter(|p| p.hour == H && p.kind == "subagent:council")
            .map(|p| p.dollars)
            .sum();
        assert!((lump - 1.0).abs() < 1e-9, "{pieces:?}");
    }

    // A fleet keeps no logs, so its total stands where it started.
    #[test]
    fn a_fleet_without_logs_keeps_its_total() {
        let history = history();
        turn(&history, "me", H, "opus", 10.0, Some("Q1"));
        run(&history, "fleet", "me", H, 5.0, &["f1"]);
        let pieces = read(&history, H, H + 1).unwrap().pieces(&Ratios::default());
        assert!((total(&pieces) - 15.0).abs() < 1e-9);
        let fleet = by(&pieces, |piece| piece.kind.clone());
        assert!(fleet.contains(&("subagent:fleet".into(), 5.0)), "{fleet:?}");
        let quests = by(&pieces, |piece| piece.quest.clone().unwrap_or_default());
        assert_eq!(quests, vec![("Q1".into(), 15.0)]);
    }

    // A swarm's agent is its own agent, named, in its quest, and takes its
    // share of a bill that came without a split.
    #[test]
    fn a_swarm_s_agent_takes_its_share() {
        let history = history();
        turn(&history, "me", H, "opus", 10.0, Some("Q1"));
        turn(&history, "acp", H, "sonnet", 30.0, Some("Q2"));
        history
            .db
            .execute(
                "UPDATE sessions SET kind = 'swarm', name = 'acp@team' WHERE id = 'acp'",
                [],
            )
            .unwrap();
        billed(&history, "warehouse", H, "pi", "opus", None, 5.0);
        billed(&history, "warehouse", H, "pi", "sonnet", None, 15.0);
        settle(&history, H + 1);
        let pieces = read(&history, H, H + 1).unwrap().pieces(&Ratios::default());
        let agent = pieces.iter().find(|p| p.session == "acp").unwrap();
        assert_eq!(
            (
                agent.kind.as_str(),
                agent.persona.as_str(),
                agent.quest.as_deref()
            ),
            ("subagent:swarm", "acp@team", Some("Q2"))
        );
        assert!((agent.dollars - 15.0).abs() < 1e-9);
        assert!((total(&pieces) - 20.0).abs() < 1e-9);
    }

    // A bill without a split goes to the pieces on its own model, since
    // models are priced apart; a model no log names follows every piece.
    #[test]
    fn an_unsplit_bill_stays_on_its_model() {
        let history = history();
        turn(&history, "s1", H, "opus", 10.0, Some("Q1"));
        turn(&history, "s2", H, "sonnet", 30.0, Some("Q2"));
        billed(&history, "warehouse", H, "pi", "opus", None, 8.0);
        billed(&history, "warehouse", H, "pi", "sonnet", None, 20.0);
        billed(&history, "warehouse", H, "pi", "other", None, 4.0);
        settle(&history, H + 1);
        let pieces = read(&history, H, H + 1).unwrap().pieces(&Ratios::default());
        let quests = by(&pieces, |piece| piece.quest.clone().unwrap_or_default());
        assert_eq!(quests, vec![("Q2".into(), 23.0), ("Q1".into(), 9.0)]);
    }

    #[test]
    fn a_live_hour_is_the_meter_with_the_rest_other_tools() {
        let history = history();
        turn(&history, "s1", H, "opus", 10.0, None);
        billed(&history, "meter", H, "all", "opus", None, 12.0);
        let ratios = Ratios::fixed(&[("opus", 0.85)]);
        let pieces = read(&history, H, H + 1).unwrap().pieces(&ratios);
        assert!((total(&pieces) - 12.0).abs() < 1e-9);
        let tools = by(&pieces, |piece| piece.tool.clone());
        assert!((tools[0].1 - 8.5).abs() < 1e-9 && tools[0].0 == "pi");
        assert_eq!(weakest(&pieces), Some(Source::Meter));
    }

    // The meter says each model's spend, so what pi's logs don't account
    // for keeps its model: a model no log names is other tools' on it, and
    // a model the meter doesn't name is drawn from what's left over.
    #[test]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a few dollars in rounded cents are far inside i64"
    )]
    fn a_live_hour_s_other_tools_keep_their_models() {
        let history = history();
        turn(&history, "s1", H, "opus", 10.0, None);
        turn(&history, "s2", H, "sonnet", 2.0, None);
        billed(&history, "meter", H, "all", "opus", None, 12.0);
        billed(&history, "meter", H, "all", "gemini", None, 5.0);
        let ratios = Ratios::fixed(&[("opus", 0.85), ("sonnet", 1.0)]);
        let pieces = read(&history, H, H + 1).unwrap().pieces(&ratios);
        assert!((total(&pieces) - 17.0).abs() < 1e-9);
        let mut models = by(&pieces, |piece| format!("{} {}", piece.tool, piece.model));
        models.sort_by(|one, other| one.0.cmp(&other.0));
        let rounded: Vec<(String, i64)> = models
            .into_iter()
            .map(|(name, dollars)| (name, (dollars * 100.0).round() as i64))
            .collect();
        assert_eq!(
            rounded,
            [
                // $8.50 left over, of which sonnet's $2 is drawn evenly.
                ("other gemini".into(), 382),
                ("other opus".into(), 268),
                ("pi opus".into(), 850),
                ("pi sonnet".into(), 200),
            ]
        );
    }

    #[test]
    fn after_the_bill_the_logs_are_estimated_by_the_ratio_learned() {
        let history = history();
        turn(&history, "s1", H, "opus", 10.0, None);
        billed(&history, "warehouse", H - 10, "pi", "opus", None, 1.0);
        settle(&history, H - 9);
        let ratios = Ratios::fixed(&[("opus", 0.8)]);
        let pieces = read(&history, H, H + 1).unwrap().pieces(&ratios);
        assert!((total(&pieces) - 8.0).abs() < 1e-9);
        assert_eq!(weakest(&pieces), Some(Source::Estimated));
    }
}
