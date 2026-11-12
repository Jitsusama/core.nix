//! Session files written as Chrome writes them, for tests: this crate's
//! own, and, with the `written` feature, the programs daemon's, which
//! stands a profile in for Chrome's and changes it as Chrome would.

use crate::session::{
    LAST_ACTIVE_TIME, SET_SELECTED_NAVIGATION_INDEX, SET_TAB_INDEX_IN_WINDOW, SET_TAB_WINDOW,
    TO_UNIX, UPDATE_TAB_NAVIGATION, VERSION,
};

/// A command as Chrome writes it: its id and what it carries.
pub type Command = (u8, Vec<u8>);

/// A session file holding these commands, as Chrome writes them.
///
/// # Panics
///
/// When a command carries more than a command's two-byte size can say.
#[must_use]
pub fn session(commands: &[Command]) -> Vec<u8> {
    let mut file = b"SNSS".to_vec();
    file.extend(VERSION.to_le_bytes());
    for (id, carried) in commands {
        let size = u16::try_from(carried.len() + 1).expect("a command Chrome could write");
        file.extend(size.to_le_bytes());
        file.push(*id);
        file.extend(carried);
    }
    file
}

/// Integers as a command carries them.
#[must_use]
pub fn ints(values: &[i32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

fn padded(mut bytes: Vec<u8>) -> Vec<u8> {
    bytes.resize(bytes.len().next_multiple_of(4), 0);
    bytes
}

fn size(of: usize) -> i32 {
    i32::try_from(of).expect("a size a pickle can say")
}

/// A tab's page at an index of its pages back and forward.
#[must_use]
pub fn page(tab: i32, index: i32, address: &str, title: &str) -> Command {
    let mut pickle = ints(&[tab, index, size(address.len())]);
    pickle.extend(padded(address.as_bytes().to_vec()));
    let units: Vec<u16> = title.encode_utf16().collect();
    pickle.extend(ints(&[size(units.len())]));
    pickle.extend(padded(
        units.iter().flat_map(|unit| unit.to_le_bytes()).collect(),
    ));
    // What Chrome writes after the title, which nothing here reads.
    pickle.extend(ints(&[0, 0]));
    let mut carried = ints(&[size(pickle.len())]);
    carried.extend(pickle);
    (UPDATE_TAB_NAVIGATION, carried)
}

/// A tab in a window at a place, on one page titled by its id, looked at
/// `looked` milliseconds after 1970.
#[must_use]
pub fn tab(window: i32, tab: i32, index: i32, address: &str, looked: i64) -> Vec<Command> {
    let mut active = ints(&[tab, 0]);
    active.extend((TO_UNIX + looked * 1000).to_le_bytes());
    vec![
        (SET_TAB_WINDOW, ints(&[window, tab])),
        (SET_TAB_INDEX_IN_WINDOW, ints(&[tab, index])),
        page(tab, 0, address, &format!("Title {tab}")),
        (SET_SELECTED_NAVIGATION_INDEX, ints(&[tab, 0])),
        (LAST_ACTIVE_TIME, active),
    ]
}
