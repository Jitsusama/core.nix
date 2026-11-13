//! Quests' rows, made ready for the launcher, so the launcher holds and
//! sorts nothing of Quests (Joel, 2026-11-10: "the daemons need to be the
//! data layer, and they need to serve the launcher"). The daemon follows
//! the mycelium node's open and ended quests and its sessions, and makes
//! every row from them:
//!
//! - [`quest`]: what one quest's record says, as Quests reads it.
//! - [`order`]: Quests' one order, open before ended, by priority and
//!   rank, then the newest, then A to Z with numbers as numbers.
//! - [`holder`]: the session holding each quest, running before parked.
//! - [`served`]: each quest's row in the launcher's form, and the summary,
//!   the open quests' served and the ended ones' made beside them.
//! - [`search`]: what's typed found among every quest, open and ended.
//! - [`words`]: Quests' words, as the launcher's vocabulary says them.

pub mod holder;
pub mod order;
pub mod quest;
pub mod search;
pub mod served;
pub mod words;
