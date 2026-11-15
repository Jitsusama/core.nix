//! The day's switches, read from update-to-main's journal: how long each
//! took, the one still building, and the pieces built on the laptop, so
//! the parsing changes with what update-to-main says and nothing else.

use serde::Deserialize;

/// The day's switches, from `journalctl -u update-to-main.service -o
/// short-unix` up to `to`.
#[derive(Debug, PartialEq)]
pub struct Switches {
    /// Each switch's length in minutes, from the first ask after the last
    /// update ended to the step that switched.
    pub lengths: Vec<f64>,
    /// How long the update still building at `to` has run, in minutes: on
    /// 11 November one built the kernel on the laptop from 16:42 past
    /// 19:00, and a day read from finished switches alone missed it.
    pub under_way: Option<f64>,
    /// The pieces the day's updates built on the laptop.
    pub here: u32,
    /// Of them, those that aren't light: the ones meant for the builders
    /// that an update built here once the builders didn't.
    pub heavy: u32,
}

#[must_use]
pub fn switches(journal: &str, to: f64) -> Switches {
    #[derive(Deserialize)]
    struct Said {
        step: Option<String>,
    }
    let mut lengths = Vec::new();
    let mut here = 0;
    let mut heavy = 0;
    // The pieces the update building now sent to the builders.
    let mut away = 0;
    // What the fallback took to build here, until its build says how many
    // it built here rather than on the builders it still uses.
    let mut fallback: Option<u32> = None;
    let mut asked: Option<f64> = None;
    // Whether the update asked for got as far as building, so an ask the
    // journal says no more of isn't taken for one under way.
    let mut building = false;
    for line in journal.lines() {
        let Some((stamp, rest)) = line.split_once(' ') else {
            continue;
        };
        let Ok(at) = stamp.parse::<f64>() else {
            continue;
        };
        if rest.contains("asks for an update") {
            asked.get_or_insert(at);
        } else if let Some(needs) = rest.split_once("the system needs ") {
            let pending = fallback.take().unwrap_or_default();
            here += pending;
            heavy += pending;
            let (light, sent) = pieces_here(needs.1).unwrap_or_default();
            here += light;
            away = sent;
            building = asked.is_some();
        } else if let Some((_, again)) = rest.split_once("so they go where the first were sent: ") {
            // The second listing, just before the build here, sends again
            // what went missing since the first.
            away = sent_again(again).unwrap_or(away);
        } else if rest.contains(", so they build here") {
            // update-to-main's fallback: what the builders didn't build
            // builds here, light or not, though the builders still take
            // what they can of it.
            fallback = Some(away);
            away = 0;
        } else if let Some((_, built)) = rest.split_once("building the rest here: ")
            && let Some(sent) = fallback.take()
        {
            let built = built_here(built).unwrap_or(sent);
            here += built;
            heavy += built;
        } else if let Some(brace) = rest.find('{')
            && let Ok(said) = serde_json::from_str::<Said>(&rest[brace..])
        {
            if matches!(
                said.step.as_deref(),
                Some("done" | "unchanged" | "refused" | "failed")
            ) {
                let pending = fallback.take().unwrap_or_default();
                here += pending;
                heavy += pending;
            }
            match said.step.as_deref() {
                Some("done") => {
                    if let Some(asked) = asked.take() {
                        lengths.push((at - asked) / 60.0);
                    }
                    building = false;
                }
                Some("unchanged" | "refused" | "failed") => {
                    asked = None;
                    building = false;
                }
                _ => {}
            }
        }
    }
    let pending = fallback.unwrap_or_default();
    let under_way = asked.filter(|_| building).map(|asked| (to - asked) / 60.0);
    Switches {
        lengths,
        under_way,
        here: here + pending,
        heavy: heavy + pending,
    }
}

/// From update-to-main's "N pieces in Ts, M built: a on here, b on
/// ssh-ng://..., and K more", how many it built on here: those named on
/// here, and the unnamed at the named ones' share. Nothing when the build
/// failed, since it says then only what it was sent.
fn built_here(said: &str) -> Option<u32> {
    if said.contains(", failed after ") {
        return None;
    }
    let Some((_, listed)) = said.split_once(" built: ") else {
        return Some(0);
    };
    let (listed, more) = match listed.rsplit_once(" and ") {
        Some((listed, more)) if more.ends_with(" more") => {
            (listed, more.trim_end_matches(" more").parse::<u32>().ok()?)
        }
        _ => (listed, 0),
    };
    let names: Vec<&str> = listed.split(", ").collect();
    let all = u32::try_from(names.len()).ok()?;
    let on_here = u32::try_from(
        names
            .iter()
            .filter(|name| name.ends_with(" on here"))
            .count(),
    )
    .ok()?;
    Some(on_here + (more * on_here + all / 2) / all.max(1))
}

/// From "N pieces built: K kept apart, R on the builders, the rest here",
/// N less K and R, built here, and K and R, sent away.
fn pieces_here(said: &str) -> Option<(u32, u32)> {
    let number = |text: &str| text.trim().split(' ').next()?.parse::<u32>().ok();
    let (all, rest) = said.split_once(" pieces built: ")?;
    let (kept, rest) = rest.split_once(" kept apart, ")?;
    let (away, _) = rest.split_once(" on the builders")?;
    let sent = number(kept)? + number(away)?;
    Some((number(all)?.saturating_sub(sent), sent))
}

/// From "K kept apart, R on the builders", K and R.
fn sent_again(said: &str) -> Option<u32> {
    let number = |text: &str| text.trim().parse::<u32>().ok();
    let (kept, rest) = said.split_once(" kept apart, ")?;
    let (away, _) = rest.split_once(" on the builders")?;
    Some(number(kept)? + number(away)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_switch_runs_from_its_first_ask_to_its_switch() {
        let journal = "\
100.0 laptop u[1]: uid 1000 asks for an update
160.0 laptop u[1]: uid 1000 asks for an update
200.0 laptop u[1]: the system needs 15 pieces built: 2 kept apart, 3 on the builders, the rest here
700.0 laptop u[1]: {\"step\":\"done\",\"commit\":\"c\"}
800.0 laptop u[1]: uid 1000 asks for an update
900.0 laptop u[1]: {\"step\":\"refused\",\"why\":\"a call\"}
1000.0 laptop u[1]: uid 1000 asks for an update
1300.0 laptop u[1]: {\"step\":\"done\",\"commit\":\"d\"}
";
        let read = switches(journal, 2000.0);
        assert_eq!(
            read.lengths,
            vec![10.0, 5.0],
            "a refused ask was counted into the next switch, or a later ask restarted one"
        );
        assert_eq!(
            read.here, 10,
            "the pieces built here weren't 15 less 2 and 3"
        );
        assert_eq!(
            read.under_way, None,
            "a day of finished switches had one under way"
        );
        assert_eq!(
            read.heavy, 0,
            "pieces built on the builders were heavy here"
        );
    }

    #[test]
    fn what_the_builders_didnt_build_and_the_laptop_did_is_heavy() {
        let journal = "\
100.0 laptop u[1]: uid 1000 asks for an update
110.0 laptop u[1]: the system needs 15 pieces built: 2 kept apart, 3 on the builders, the rest here
120.0 laptop u[1]: building the system on the machine's builders failed, so they build here, after:
700.0 laptop u[1]: {\"step\":\"done\",\"commit\":\"c\"}
";
        let read = switches(journal, 2000.0);
        assert_eq!(
            (read.here, read.heavy),
            (15, 5),
            "the pieces the builders didn't build weren't counted heavy here"
        );
        let again = "\
100.0 laptop u[1]: uid 1000 asks for an update
110.0 laptop u[1]: the system needs 15 pieces built: 0 kept apart, 3 on the builders, the rest here
130.0 laptop u[1]: before the build here, 1 pieces that aren't light are still to build, so they \
go where the first were sent: 0 kept apart, 1 on the builders
140.0 laptop u[1]: building the system on the machine's builders failed, so they build here, after:
";
        assert_eq!(
            switches(again, 2000.0).heavy,
            1,
            "what the second listing sent and the laptop built wasn't the heavy count"
        );
    }

    #[test]
    fn only_what_the_fallback_built_on_here_is_heavy() {
        // On 14 November the 06:42 switch's fallback built 4 of its 158
        // pieces, 2 of them on here, and the day counted all 158 heavy.
        let journal = "\
100.0 laptop u[1]: uid 1000 asks for an update
110.0 laptop u[1]: the system needs 328 pieces built: 0 kept apart, 158 on the builders, \
the rest here
120.0 laptop u[1]: building the system on the machine's builders failed (exit status: 1), so they \
build here, after:
130.0 laptop u[1]: building the rest here: 158 pieces in 16s, 4 built: spend-watch-0.1.0 on \
ssh-ng://hook@b?compress=true, initrd-udev-rules on here, initrd-linux-7.3.3 on \
ssh-ng://hook@b?compress=true, initrd-linux-7.3.3 on here
700.0 laptop u[1]: {\"step\":\"done\",\"commit\":\"c\"}
";
        let read = switches(journal, 2000.0);
        assert_eq!(
            (read.here, read.heavy),
            (172, 2),
            "what the fallback built on the builders still was counted heavy here"
        );
        let unnamed = journal.replace(
            "initrd-linux-7.3.3 on here\n",
            "initrd-linux-7.3.3 on here and 4 more\n",
        );
        assert_eq!(
            switches(&unnamed, 2000.0).heavy,
            4,
            "the unnamed pieces weren't counted at the named pieces' share built here"
        );
        let failed = journal.replace(
            "158 pieces in 16s, 4 built:",
            "158 pieces, failed after 16s\n131.0 laptop u[1]: x",
        );
        assert_eq!(
            switches(&failed, 2000.0).heavy,
            158,
            "a fallback that failed didn't count what it was sent as heavy"
        );
    }

    #[test]
    fn an_update_still_building_is_under_way_and_an_ask_alone_isnt() {
        let building = "\
100.0 laptop u[1]: uid 1000 asks for an update
160.0 laptop u[1]: the system needs 1 pieces built: 0 kept apart, 0 on the builders, the rest here
";
        assert_eq!(
            switches(building, 700.0).under_way,
            Some(10.0),
            "an update building at the day's end wasn't under way from its ask"
        );
        let asked = "100.0 laptop u[1]: uid 1000 asks for an update\n";
        assert_eq!(
            switches(asked, 700.0).under_way,
            None,
            "an ask the journal said no more of was taken for an update under way"
        );
    }
}
