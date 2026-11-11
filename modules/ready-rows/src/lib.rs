//! The one form every daemon serves the launcher's rows in, so the
//! launcher holds no data of its own (Joel, 2026-11-10: "the daemons need
//! to be the data layer, and they need to serve the launcher"). What a
//! row's fields are, and why, is the ready-row form (Y2YBTS, 2026-11-10);
//! this holds what every daemon would otherwise write again:
//!
//! - [`order`]: the spaced `order` each row is placed by, so one row
//!   moving sends that row alone.
//! - [`kept`]: a place's rows as last told, each making answered with
//!   only the records that read differently and the keys that went.
//! - [`numbers`]: a measure said as the launcher says it, three
//!   significant digits grouped and pointed as the account's settings
//!   say, its cases in `numbers.json`.
//! - [`time`]: a time as it travels, for the shell to word, since a row
//!   that said the time itself would change every minute.

pub mod kept;
pub mod numbers;
pub mod order;
pub mod time;
