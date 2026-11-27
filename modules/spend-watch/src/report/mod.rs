//! Reports: a period's spend cut one way, or a focused look at waste,
//! caps, quests, outliers or the forecast, rendered as text, markdown,
//! JSON or CSV. The collection's reports are these same ones, written
//! ahead, so what the launcher shows is what the command line says.

mod cuts;
mod focus;
mod rendering;
mod tallies;

pub use cuts::{Cut, Period, Report, Row, agent_name, cut};
pub use focus::{cap_line, caps, forecast, outliers, quests, summary, waste};
pub use rendering::{Format, render, render_text_in};
pub use tallies::{
    Family, Helper, SessionDay, Spark, Spent, by_family, by_session, by_sessions,
    helpers_by_session, spent_by_session, today_by_session,
};
