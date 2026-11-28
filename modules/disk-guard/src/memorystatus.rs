//! How a Mac's kernel judges its memory, from `kern.memorystatus_level`,
//! the share it counts free, and `kern.memorystatus_vm_pressure_level`,
//! the level it tells apps to shed memory by. Read on any platform, so
//! Linux's tests hold it.

use crate::moments::{MemoryLevel, MemoryRoom};

/// The room the two values say, or nothing when either is outside what the
/// kernel gives (a share from 0 to 100; a level of 1, 2 or 4), so a value
/// it never gives can't pass for one it does.
pub fn room(free_percent: i32, level: i32) -> Option<MemoryRoom> {
    let free_percent = u8::try_from(free_percent)
        .ok()
        .filter(|&percent| percent <= 100)?;
    let level = match level {
        1 => MemoryLevel::Normal,
        2 => MemoryLevel::Warning,
        4 => MemoryLevel::Critical,
        _ => return None,
    };
    Some(MemoryRoom {
        free_percent,
        level,
    })
}
