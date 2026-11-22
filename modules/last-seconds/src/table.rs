//! How `last-seconds` sets out a boot's seconds as a table: its columns,
//! each second's cells beneath them, and which give way where the room is
//! narrow. It changes when what a second shows does, apart from how the
//! boots are found and listed (src/main.rs).

use crate::record::Record;
use crate::ring::Header;
use terminal_kit::layout::Align;

/// The microseconds in a second.
const SECOND_US: u64 = 1_000_000;

/// A moment as the machine's clock reads it, to the second.
#[must_use]
pub fn clock(at_ms: i64) -> String {
    let seconds = at_ms.div_euclid(1_000);
    terminal_kit::local::at(seconds).map_or_else(
        || seconds.to_string(),
        |at| format!("{:02}:{:02}:{:02}", at.hour, at.minute, at.second),
    )
}

/// Tenths of a whole, from thousandths, as 14.2 or -14.2.
fn tenths(thousandths: i64) -> String {
    let sign = if thousandths <= -50 { "-" } else { "" };
    let tenths = (thousandths.unsigned_abs() + 50) / 100;
    format!("{sign}{}.{}", tenths / 10, tenths % 10)
}

/// A share of a second, from microseconds, in whole percent.
fn share(us: Option<u32>) -> String {
    us.map_or_else(String::new, |us| {
        format!("{}%", (u64::from(us) * 100 + SECOND_US / 2) / SECOND_US)
    })
}

/// Memory in GiB, from KiB, in tenths.
fn gib(kib: Option<u32>) -> String {
    kib.map_or_else(String::new, |kib| {
        tenths(i64::from(kib) * 1_000 / (1 << 20))
    })
}

/// What the battery takes, in milliwatts, where it says: less than none
/// where it gives.
fn battery_mw(record: &Record) -> Option<i64> {
    Some(i64::from(record.battery_mv?) * i64::from(record.battery_ma?) / 1_000)
}

/// The share of their time the processors spent in the deepest rest
/// state over the second.
fn deepest(record: &Record, header: &Header) -> Option<u32> {
    let at = header.rests.len().checked_sub(1)?;
    let rested = u64::from(record.rests.get(at).copied()??);
    let processors = u64::from(header.processors).max(1);
    u32::try_from(rested / processors).ok()
}

/// The table's columns, with the deepest rest state's name in its label.
#[must_use]
pub fn columns(header: Option<&Header>) -> Vec<(String, Align)> {
    let deepest = header
        .and_then(|header| header.rests.last())
        .map_or_else(|| "DEEPEST REST".to_owned(), |name| format!("IN {name}"));
    [
        ("TIME", Align::Left),
        ("PACKAGE W", Align::Right),
        ("PLATFORM W", Align::Right),
        ("HEAT °C", Align::Right),
        ("FANS", Align::Right),
        ("LOAD", Align::Right),
        ("CPU WAIT", Align::Right),
        ("MEMORY WAIT", Align::Right),
        // The memory standstill, said in full though its figure is four
        // cells at most, since at 80 columns the memory wait has given
        // way, and a bare standstill would read as the whole machine's.
        ("MEMORY STANDSTILL", Align::Right),
        ("DISK WAIT", Align::Right),
        ("FREE GIB", Align::Right),
        ("SWAP GIB", Align::Right),
        ("BATTERY W", Align::Right),
        (deepest.as_str(), Align::Right),
    ]
    .into_iter()
    .map(|(label, align)| (label.to_owned(), align))
    .collect()
}

/// The columns that give way first where the room is too narrow, least
/// useful first: the time, the package's draw and its heat stay longest,
/// and of memory, the standstill and the swap outlast the wait, since a
/// machine swapping hard enough to stand still is what they tell apart.
/// The platform's draw, which holds the package's, and the memory wait,
/// which the standstill sharpens, go before the fans and the load, so an
/// 80-column table still shows the fans and the load beside the standstill.
pub const GIVES_WAY: [usize; 13] = [12, 13, 10, 9, 6, 2, 7, 4, 5, 11, 8, 3, 1];

/// One second's cells, beneath the columns in their order.
#[must_use]
pub fn cells(record: &Record, header: Option<&Header>) -> Vec<String> {
    let watts = |mw: Option<u32>| mw.map_or_else(String::new, |mw| tenths(i64::from(mw)));
    let fans: Vec<String> = record.fans.iter().flatten().map(u16::to_string).collect();
    vec![
        clock(record.at_ms),
        watts(record.package_mw),
        watts(record.platform_mw),
        record
            .package_mc
            .map_or_else(String::new, |mc| tenths(i64::from(mc))),
        fans.join(" "),
        record.load.map_or_else(String::new, |load| {
            format!("{}.{:02}", load / 100, load % 100)
        }),
        share(record.stalls.cpu_some),
        share(record.stalls.memory_some),
        share(record.stalls.memory_full),
        share(record.stalls.io_some),
        gib(record.available_kib),
        gib(record.swapped_kib),
        battery_mw(record).map_or_else(String::new, tenths),
        header
            .and_then(|header| deepest(record, header))
            .map_or_else(String::new, |us| share(Some(us))),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::Stalls;

    fn two_processors() -> Header {
        Header {
            slots: 8,
            began: 0,
            processors: 2,
            rests: vec!["C1".to_owned(), "C10".to_owned()],
        }
    }

    #[test]
    fn the_deepest_rest_is_a_share_of_every_processors_second() {
        let record = Record {
            rests: [Some(0), Some(1_500_000), None, None, None, None, None, None],
            ..Record::default()
        };
        assert_eq!(
            share(deepest(&record, &two_processors())),
            "75%",
            "the deepest rest wasn't said as a share of both processors' second"
        );
    }

    /// Each figure apart from every other, as the last second before the
    /// reset of 21 November read, so a cell under the wrong label shows.
    fn the_last_second_before_the_reset() -> Record {
        Record {
            at_ms: 1_795_241_514_000,
            package_mw: Some(12_000),
            platform_mw: Some(31_000),
            package_mc: Some(83_000),
            fans: [Some(5_454), Some(5_352), None, None],
            load: Some(21_594),
            stalls: Stalls {
                cpu_some: Some(897_254),
                memory_some: Some(429_297),
                memory_full: Some(179_928),
                io_some: Some(446_168),
                io_full: Some(6_793),
            },
            available_kib: Some(15_309_000),
            swapped_kib: Some(51_275_000),
            battery_mv: Some(17_000),
            battery_ma: Some(-2_000),
            rests: [Some(0), Some(500_000), None, None, None, None, None, None],
            ..Record::default()
        }
    }

    #[test]
    fn each_column_says_its_own_figure() {
        let header = two_processors();
        let record = the_last_second_before_the_reset();
        assert_eq!(
            columns(Some(&header)).len(),
            cells(&record, Some(&header)).len(),
            "a second has another number of cells than the table has columns"
        );
        // The time is left out: it reads as the machine's own zone.
        let said: Vec<(String, String)> = columns(Some(&header))
            .into_iter()
            .map(|(label, _)| label)
            .zip(cells(&record, Some(&header)))
            .skip(1)
            .collect();
        let named = |list: &[(&str, &str)]| -> Vec<(String, String)> {
            list.iter()
                .map(|(label, cell)| ((*label).to_owned(), (*cell).to_owned()))
                .collect()
        };
        assert_eq!(
            said,
            named(&[
                ("PACKAGE W", "12.0"),
                ("PLATFORM W", "31.0"),
                ("HEAT °C", "83.0"),
                ("FANS", "5454 5352"),
                ("LOAD", "215.94"),
                ("CPU WAIT", "90%"),
                ("MEMORY WAIT", "43%"),
                ("MEMORY STANDSTILL", "18%"),
                ("DISK WAIT", "45%"),
                ("FREE GIB", "14.6"),
                ("SWAP GIB", "48.9"),
                ("BATTERY W", "-34.0"),
                ("IN C10", "25%"),
            ]),
            "a column's label stood over another's figure"
        );
    }

    #[test]
    fn every_column_but_the_time_gives_way_in_order() {
        let columns = columns(None);
        let giving: Vec<&str> = GIVES_WAY.iter().map(|at| columns[*at].0.as_str()).collect();
        assert_eq!(
            giving,
            [
                "BATTERY W",
                "DEEPEST REST",
                "FREE GIB",
                "DISK WAIT",
                "CPU WAIT",
                "PLATFORM W",
                "MEMORY WAIT",
                "FANS",
                "LOAD",
                "SWAP GIB",
                "MEMORY STANDSTILL",
                "HEAT °C",
                "PACKAGE W",
            ],
            "the columns don't give way in the order their comment says"
        );
        assert_eq!(
            GIVES_WAY.len() + 1,
            columns.len(),
            "a column but the time doesn't give way"
        );
    }

    #[test]
    fn an_80_column_table_keeps_the_fans_and_the_load_at_a_reset() {
        let header = two_processors();
        let columns = columns(Some(&header));
        let labels: Vec<(&str, Align)> = columns
            .iter()
            .map(|(label, align)| (label.as_str(), *align))
            .collect();
        let lines = terminal_kit::layout::table_in(
            &labels,
            &[cells(&the_last_second_before_the_reset(), Some(&header))],
            &terminal_kit::Pen::plain(),
            terminal_kit::Room::Cells(80),
            &GIVES_WAY,
        );
        let kept: Vec<&str> = lines
            .first()
            .map(|head| {
                head.split("  ")
                    .map(str::trim)
                    .filter(|label| !label.is_empty())
            })
            .into_iter()
            .flatten()
            .collect();
        assert_eq!(
            kept,
            [
                "TIME",
                "PACKAGE W",
                "HEAT °C",
                "FANS",
                "LOAD",
                "MEMORY STANDSTILL",
                "SWAP GIB"
            ],
            "an 80-column table kept other columns than the fans and the load beside the memory"
        );
    }

    #[test]
    fn figures_are_said_in_tenths() {
        for (thousandths, said) in [
            (14_250, "14.3"),
            (71_000, "71.0"),
            (40, "0.0"),
            (-14_250, "-14.3"),
            (-40, "0.0"),
        ] {
            assert_eq!(
                tenths(thousandths),
                said,
                "{thousandths} thousandths weren't said as {said}"
            );
        }
    }

    #[test]
    fn a_battery_giving_power_is_said_below_none() {
        let giving = Record {
            battery_mv: Some(16_000),
            battery_ma: Some(-1_000),
            ..Record::default()
        };
        assert_eq!(
            battery_mw(&giving),
            Some(-16_000),
            "a battery giving power wasn't said below none"
        );
    }
}
