//! What rhythm says: to Joel in a terminal, in plain words, and to other
//! programs in its profile file, as one JSON object.

use crate::nights::{ENOUGH, Profile, clock_of, weekday_name};

/// How long before his likely stop the evening's wind-down starts, in
/// minutes: two and a half hours, so the screen is half as bright two
/// hours before bed, and a midnight guess starts it at 21:30, where the
/// backlight's evening starts until it follows this.
pub const LEAD: u32 = 150;

/// The sun's day where he is, by the local clock, in minutes past
/// midnight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sun {
    pub rise: u32,
    pub set: u32,
}

/// The wind-down's start for a likely stop, as the clock says it.
pub fn wind_down(after_six: u32) -> String {
    clock_of((after_six + 1440 - LEAD) % 1440)
}

/// A minute past midnight as the clock says it.
fn clock(minute: u32) -> String {
    format!("{:02}:{:02}", minute / 60 % 24, minute % 60)
}

/// What Joel reads in a terminal, for an evening of this weekday.
pub fn said(profile: &Profile, weekday: u32, sun: Option<Sun>) -> String {
    let (tonight, learned) = profile.tonight(weekday);
    let mut lines = Vec::new();
    if learned {
        if let Some(usual) = profile.usual {
            lines.push(format!(
                "You usually stop for the night around {}, from {} nights in the last four weeks.",
                clock_of(usual),
                profile.nights
            ));
        }
        lines.push(format!(
            "Tonight, a {}, it's likely around {}, so the wind-down would start about {}.",
            weekday_name(weekday),
            clock_of(tonight),
            wind_down(tonight)
        ));
    } else {
        lines.push(format!(
            "Not enough nights yet ({} of {ENOUGH}), so it guesses you stop around {}, and \
             the wind-down would start about {}.",
            profile.nights,
            clock_of(tonight),
            wind_down(tonight)
        ));
    }
    let weekdays: Vec<String> = (1..=7)
        .map(|day| day % 7)
        .filter_map(|day| {
            let stop = profile.weekdays[day as usize]?;
            Some(format!("{} {}", weekday_name(day), clock_of(stop)))
        })
        .collect();
    if !weekdays.is_empty() {
        lines.push(format!("By weekday: {}.", weekdays.join(", ")));
    }
    lines.push(match sun {
        Some(sun) => {
            let daylight = sun.set.saturating_sub(sun.rise);
            format!(
                "Today the sun rises at {} and sets at {}: {} h {} min of daylight.",
                clock(sun.rise),
                clock(sun.set),
                daylight / 60,
                daylight % 60
            )
        }
        None => "Where you are isn't known yet, so there's no sunset to say.".to_owned(),
    });
    lines.push(
        "It learns from when you're at the keys and when you write to pi: times only, \
         never what you type."
            .to_owned(),
    );
    lines.join("\n")
}

/// The profile as one JSON object, for the backlight and anything else
/// that reads it.
pub fn profile_json(profile: &Profile, weekday: u32, sun: Option<Sun>) -> String {
    let (tonight, learned) = profile.tonight(weekday);
    let text = |value: Option<String>| value.map_or("null".to_owned(), |it| format!("\"{it}\""));
    let weekdays: Vec<String> = (0..7)
        .map(|day| {
            format!(
                "\"{}\": {}",
                weekday_name(day),
                text(profile.weekdays[day as usize].map(clock_of))
            )
        })
        .collect();
    format!(
        "{{\"v\": 1, \"basis\": \"{}\", \"nights\": {}, \"usual\": {}, \"weekday\": \"{}\", \
         \"tonight\": \"{}\", \"windDown\": \"{}\", \"sunrise\": {}, \"sunset\": {}, \
         \"weekdays\": {{{}}}}}\n",
        if learned { "learned" } else { "guess" },
        profile.nights,
        text(profile.usual.map(clock_of)),
        weekday_name(weekday),
        clock_of(tonight),
        wind_down(tonight),
        text(sun.map(|sun| clock(sun.rise))),
        text(sun.map(|sun| clock(sun.set))),
        weekdays.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn learned() -> Profile {
        let mut weekdays = [None; 7];
        weekdays[5] = Some(8 * 60 + 30);
        Profile {
            nights: 23,
            usual: Some(6 * 60 + 40),
            weekdays,
        }
    }

    #[test]
    fn a_midnight_stop_winds_down_from_half_past_nine() {
        assert_eq!(wind_down(6 * 60), "21:30", "two and a half hours before");
        assert_eq!(wind_down(60), "16:30", "an early stop winds down earlier");
    }

    #[test]
    fn joel_reads_his_usual_stop_tonight_s_and_the_sun() {
        let read = said(
            &learned(),
            5,
            Some(Sun {
                rise: 425,
                set: 1011,
            }),
        );
        assert!(
            read.contains("around 00:40, from 23 nights"),
            "the usual stop: {read}"
        );
        assert!(
            read.contains(
                "a Friday, it's likely around 02:30, so the wind-down would start about 00:00"
            ),
            "tonight's: {read}"
        );
        assert!(
            read.contains("By weekday: Friday 02:30."),
            "the weekdays: {read}"
        );
        assert!(
            read.contains("rises at 07:05 and sets at 16:51: 9 h 46 min"),
            "the sun: {read}"
        );
    }

    #[test]
    fn too_few_nights_say_they_guess() {
        let few = Profile {
            nights: 3,
            usual: Some(6 * 60 + 40),
            weekdays: [None; 7],
        };
        let read = said(&few, 4, None);
        assert!(
            read.contains("Not enough nights yet (3 of 7), so it guesses you stop around 00:00"),
            "the guess: {read}"
        );
        assert!(read.contains("21:30"), "from half past nine: {read}");
        assert!(read.contains("isn't known yet"), "no place: {read}");
    }

    #[test]
    fn the_profile_file_says_tonight_and_each_weekday() {
        let json = profile_json(&learned(), 4, None);
        for part in [
            "\"basis\": \"learned\"",
            "\"nights\": 23",
            "\"usual\": \"00:40\"",
            "\"weekday\": \"Thursday\"",
            "\"tonight\": \"00:40\"",
            "\"windDown\": \"22:10\"",
            "\"sunset\": null",
            "\"Friday\": \"02:30\"",
            "\"Monday\": null",
        ] {
            assert!(json.contains(part), "{part} in {json}");
        }
    }
}
