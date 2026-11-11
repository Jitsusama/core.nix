//! spend-watch keeps an account told what its AI use costs.
//!
//! It reads what pi writes as it works (each session's log, the harness's
//! runs of subagents, its session registry and its quest store) and
//! whatever feeds a machine adds, programs that speak for a provider's bill
//! over the contract in `feed`. From those it keeps a history, kept for
//! good and rebuilt from its sources when its shape changes, and from the
//! history it traces every dollar to its session, agent and quest, finds
//! where money bought nothing, forecasts what's coming, and watches for
//! what wants the person: a cap close or lapsing, a credential nobody
//! knew spending under their name.
//!
//! Every figure carries where it came from. A source that's missing leaves
//! the rest working, down to pi's logs alone, which price at pi's own list
//! rates; a feed's bill, while there is one, teaches each model's ratio of
//! bill to list, so the estimate after it goes is close and says so.
//!
//! The daemon writes what the shell shows to its runtime directory, a
//! small status file with a heartbeat for the overview's row and the
//! margin, and a larger one for the launcher's collection, every string in
//! it ready to draw, which its socket tells as records too, each pass only
//! those it changed (`interface`, cut as `records` says), and takes the
//! shell's acts on them (`acts`). `spend-watch report` answers from the
//! history alone.
//!
//! Everything but the edges (the files, the feeds' processes, D-Bus and
//! the clock) is plain functions over plain data, tested without them.

pub mod acts;
pub mod backtest;
pub mod budgets;
pub mod caps;
pub mod cli;
pub mod collection;
pub mod config;
pub mod daemon;
pub mod detect;
pub mod estimate;
pub mod feed;
pub mod findings;
pub mod forecast;
pub mod go;
pub mod history;
pub mod ingest;
pub mod interface;
pub mod logs;
pub mod money;
pub mod notices;
pub mod over;
pub mod picture;
pub mod quests;
pub mod random;
pub mod records;
pub mod report;
pub mod runs;
pub mod series;
pub mod standing;
pub mod stats;
pub mod status;
pub mod time;
pub mod tokens;
pub mod waste;
