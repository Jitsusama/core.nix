// How the launcher must order what it finds, run in Qt's own JavaScript
// engine, the one the shell runs it in. The `the-launcher-lands-where-it-did`
// check copies Rank.js beside this file and runs `qml -platform offscreen
// ranking.qml`. Each case is a promise Joel leans on without looking: the
// same letters land on the same row, a thing is one row, and one slip
// teaches nothing.
import QtQuick
import "Rank.js" as Rank

QtObject {
    function row(key, name, fields) {
        return Object.assign({ key: key, name: name }, fields ?? {});
    }

    function f(key, name, tier, score) {
        return { choice: row(key, name), tier: tier, score: score ?? 0 };
    }

    function keys(rows) {
        return rows.map(one => one.choice ? one.choice.key : one.key).join(" ");
    }

    // Picks as Picks.qml keeps them, from { query -> { key -> picks } }: each
    // made at `t`, in milliseconds.
    function picks(table, t) {
        const out = {};
        for (const query of Object.keys(table)) {
            out[query] = {};
            for (const key of Object.keys(table[query]))
                out[query][key] = { n: table[query][key], c: table[query][key], t: t ?? 0 };
        }
        return out;
    }

    // What's latched, as Picks.qml asks Rank.js.
    function latched(kept, typed, there, whole, held) {
        return Rank.latched(kept, typed, { there: there, whole: whole, held: held ?? "" });
    }

    // One more pick of `key` for `typed`, as a new table.
    function pick(kept, typed, key) {
        const out = JSON.parse(JSON.stringify(kept));
        const answers = out[typed] ?? (out[typed] = {});
        const was = answers[key] ?? { n: 0, c: 0, t: 0 };
        answers[key] = { n: was.n + 1, c: was.c + 1, t: 0 };
        return out;
    }

    Component.onCompleted: {
        const failures = [];
        let asked = 0;
        const expect = (what, got, wanted) => {
            asked++;
            if (JSON.stringify(got) !== JSON.stringify(wanted))
                failures.push(`${what} gave ${JSON.stringify(got)}, not ${JSON.stringify(wanted)}`);
        };

        // A thing found three ways is one row, where the first was, saying
        // what the richest says.
        const window = row("window:kitty:pi", "\u03c0 - TESTQA", { is: "window:7" });
        const quest = row("quest:QEST-1", "A quest", { is: "window:7", rich: 1 });
        const agent = row("agent:TESTQA", "A quest", { is: "window:7", rich: 2 });
        const slack = row("program:slack", "Slack");
        expect("one row a thing", keys(Rank.once([window, slack, quest, agent])),
            "agent:TESTQA program:slack");
        // Two windows titled alike are two things.
        expect("alike but two", keys(Rank.once([
            row("window:kitty:zsh", "zsh", { is: "window:1" }),
            row("window:kitty:zsh", "zsh", { is: "window:2" })
        ])), "window:kitty:zsh window:kitty:zsh");

        // Tiers: the name typed whole, then its start, then a later word's,
        // then a synonym, whatever the score.
        expect("the name itself first", Rank.tier("Slack", " slack ", false), 0);
        expect("the name's start", Rank.tier("Power off the machine", "pow", false), 0.5);
        expect("a later word's start", Rank.tier("Use power saver", "pow", false), 1);
        expect("a word start", Rank.tier("Google Chrome", "chro", false), 1);
        expect("by another word", Rank.tier("Suspend", "sleep", true), 2);
        // Another word typed whole before one only started.
        expect("another word whole", Rank.wordTier("Turn on do not disturb dnd quiet", "quiet"), 2);
        expect("another word started", Rank.wordTier("Turn the volume down quieter", "quiet"),
            2.5);
        expect("every word whole", Rank.says("Clipboard copies pictures", "pi"), false);
        expect("a short word whole", Rank.says("Agents sessions pi ai", "pi"), true);

        // Return's verb: an act's first word, and the way a turn goes.
        expect("a first word", Rank.verbIn("Switch to it"), "switch");
        expect("a turn's way", Rank.verbIn("Turn the volume down"), "turn down");
        expect("a turn's way first", Rank.verbIn("Turn on do not disturb"), "turn on");
        expect("a turn's way last", Rank.verbIn("Turn the keyboard's lights off"), "turn off");
        expect("a way only for a turn", Rank.verbIn("Power off the machine"), "power");
        const found = [f("session:suspend", "Suspend", 2, 900), f("program:slack", "Slack", 1, 10),
            f("action:sl", "Sl", 0, 1)];
        expect("a tier wins over a score", keys(Rank.order(found, {})),
            "action:sl program:slack session:suspend");

        // The latch, worked out from the picks. Every row is there unless a
        // case says, each learned from one letter.
        const all = Rank.maybeThere({});
        const at = (kept, typed, held) => latched(kept, typed, all, [], held);
        // A lead of two: one pick latches nothing, so one stray pick can't
        // take a word; two latch it at every start of the word.
        expect("one pick, nothing", at(picks({ slack: { "program:slack": 1 } }), "slack"), "");
        const twice = picks({ slack: { "program:slack": 2 } });
        for (const typed of ["s", "sl", "sla", "slac", "slack"])
            expect(`latched at ${typed}`, at(twice, typed), "program:slack");
        expect("not past it", at(twice, "slacks"), "");
        expect("one ahead, nothing", at(picks({ sl: { "program:slack": 3, "session:sleep": 2 } }),
            "sl"), "");
        expect("two ahead", at(picks({ sl: { "program:slack": 4, "session:sleep": 2 } }), "sl"),
            "program:slack");
        // What it was left on holds until another leads it by two, so one
        // slip neither takes it nor loosens it.
        const slip = picks({ sl: { "program:slack": 2, "session:sleep": 1 } });
        expect("one slip", at(slip, "sl", "program:slack"), "program:slack");
        const close = picks({ sl: { "program:slack": 3, "session:sleep": 4 } });
        expect("held, one behind", at(close, "sl", "program:slack"), "program:slack");
        expect("taken, two ahead", at(pick(close, "sl", "session:sleep"), "sl", "program:slack"),
            "session:sleep");
        // A latch seeded from one pick isn't honoured.
        expect("a seed of one", at(picks({ slack: { "byhand:slack": 1 } }), "slack", "byhand:slack"),
            "");
        // As picks come, the latch moves only when another leads by two.
        let kept = {};
        let latches = {};
        const picking = (typed, key) => {
            kept = pick(kept, typed, key);
            latches = Rank.relatch(latches, kept, typed, all);
        };
        picking("slack", "program:slack");
        expect("first pick, no latch", Rank.heldAt(latches, "sl"), "");
        picking("slack", "program:slack");
        expect("second pick, latched", [Rank.heldAt(latches, "s"), Rank.heldAt(latches, "slack")],
            ["program:slack", "program:slack"]);
        for (let n = 0; n < 3; n++)
            picking("sl", "session:sleep");
        expect("three slips, one ahead", Rank.heldAt(latches, "sl"), "program:slack");
        picking("sl", "session:sleep");
        expect("two ahead, moved", Rank.heldAt(latches, "sl"), "session:sleep");
        expect("the longer kept", at(kept, "slack", Rank.heldAt(latches, "slack")),
            "program:slack");
        // The latches a launcher before kept read as the row each was on.
        expect("kept before", Rank.heldAt({ sl: { on: "program:slack", rival: "", times: 0 } },
            " SL "), "program:slack");
        // Words typed together latch whole, not by their starts, and lend
        // nothing to their first word.
        const words = picks({ "vol up": { "action:up": 2 }, "time in gmt": { "notice:x": 3 } });
        expect("words whole", [at(words, "vol up"), at(words, "vol"), at(words, "time")],
            ["action:up", "", ""]);
        // What can't be taken back latches only past its first word.
        const off = Rank.foundNow({
            "session:off": Rank.leastOf({ confirm: true, name: "Power off" })
        });
        const powering = picks({ pow: { "session:off": 3 }, power: { "session:off": 2 } });
        expect("never short", [latched(powering, "pow", off, []),
            latched(powering, "power", off, [])], ["", "session:off"]);
        expect("learned from one letter", Rank.leastOf({ name: "Slack" }), 1);

        // A word typed whole: the row it names is first over another latched,
        // unless that one was picked for exactly it at least twice.
        const named = ["program:slack"];
        const theme = picks({ sla: { "program:slack": 16 }, slack: { "program:slack": 1,
            "byhand:slack": 1, "window:slack:Now": 1 } });
        expect("slack is Slack", latched(theme, "slack", all, named, "byhand:slack"), "");
        expect("its name first", keys(Rank.order([f("byhand:slack", "Slack's Theme", 0.5),
            f("program:slack", "Slack", 0)], { latched: "" })), "program:slack byhand:slack");
        expect("sla is still Slack", latched(theme, "sla", all, named), "program:slack");
        const longer = picks({ slackers: { "byhand:slack": 5 } });
        expect("a longer word's picks", [latched(longer, "slack", all, named),
            latched(longer, "slack", all, [])], ["", "byhand:slack"]);
        const meant = picks({ slack: { "byhand:slack": 2 } });
        expect("twice for it exactly", latched(meant, "slack", all, named),
            "byhand:slack");

        // A row that's gone counts for nothing: it can't hold a latch, nor
        // keep one from another.
        const keyed = picks({ time: { "keyboard:key:Numb:49": 3 } });
        expect("a key renamed", [latched(keyed, "time", Rank.foundNow({}), []),
            latched(keyed, "time", Rank.foundNow({ "keyboard:key:Numb:49": 1 }), [])],
            ["", "keyboard:key:Numb:49"]);
        const windowed = picks({ slac: { "program:slack": 6, "window:slack:Unread": 9 } });
        expect("a window closed", latched(windowed, "slac", Rank.foundNow({
            "program:slack": 1 }), [], "window:slack:Unread"), "program:slack");
        // And its picks are dropped when picks are next written: one not
        // among the rows there, of a kind that is, not picked for a while.
        // A kind not there at all (not read yet, or off) keeps its own.
        const day = 24 * 60 * 60 * 1000;
        const known = { "program:slack": 1, "window:slack:Now": 1, "notice:Slack:Hi": 1,
            "tuple:open": 1 };
        const old = picks({ slac: { "program:slack": 6, "window:slack:Unread": 1 },
            "time in gmt": { "notice:Slack:Gone": 1 }, shrim: { "emoji:shrimp": 1 } });
        const recent = picks({ leave: { "tuple:leave": 1 } }, 30 * day);
        const forgotten = Rank.forget(Object.assign(old, recent), known, 31 * day, 14 * day);
        expect("gone, forgotten", Object.keys(forgotten).sort(), ["leave", "shrim", "slac"]);
        expect("the rest kept", Object.keys(forgotten.slac), ["program:slack"]);
        const standing = Rank.standing(known);
        expect("who stands", [standing("window:slack:Unread"), standing("emoji:shrimp"),
            standing("program:slack")], [0, 1, 1]);
        expect("a latch with no picks left", Rank.relatch({ "time in gmt": "notice:Slack:Gone",
            slac: "program:slack" }, forgotten, "leave", standing), { slac: "program:slack" });

        // A shorter start counts every word it starts together, so a stray
        // pick for "light" holds neither "light" nor "l", and "l" is what
        // its words were picked for most.
        const ell = picks({ light: { "program:virt-manager": 1 } });
        expect("one light", [at(ell, "l"), at(ell, "light")], ["", ""]);
        const locks = picks({ light: { "program:virt-manager": 1 }, lo: { "session:Lock": 1 },
            loc: { "session:Lock": 2 }, lock: { "session:Lock": 2 } });
        expect("l is Lock", at(locks, "l", "program:virt-manager"), "session:Lock");
        const even = picks({ light: { "program:virt-manager": 2 }, lock: { "session:Lock": 3 } });
        expect("l, no lead", at(even, "l"), "");
        expect("lock, its own", at(even, "lock"), "session:Lock");

        // What's latched comes first, over a better tier, and nothing
        // learned leaves a row out.
        const both = [f("action:sl", "Sl", 0), f("program:slack", "Slack", 1)];
        expect("the latch first", keys(Rank.order(both, { latched: at(twice, "sl") })),
            "program:slack action:sl");
        expect("every match shows", Rank.order(both, { latched: "session:gone" }).length, 2);

        // Within a tier: picked for what's typed, then picked most, then the
        // better match, the shorter name and the alphabet.
        const tied = [f("program:b", "Bb", 1, 5), f("program:a", "Aa", 1, 5), f("program:c", "C", 1, 5),
            f("program:d", "Dd", 1, 9)];
        expect("ties by score, length, name", keys(Rank.order(tied, {})),
            "program:d program:c program:a program:b");
        expect("picked for it first", keys(Rank.order(tied, {
            picked: { "program:b": { weight: 0.5 } },
            often: { "program:a": 40 }
        })), "program:b program:a program:d program:c");

        // Later rows go after their tier, never above it, but for the latched.
        const ordered = [f("program:a", "A", 0), f("program:b", "B", 1), f("program:c", "C", 2)]
            .map(one => Object.assign({ tier: one.tier }, one.choice));
        expect("late below", keys(Rank.placed(ordered, [row("copy:x", "X", { tier: 1 })], "")),
            "program:a program:b copy:x program:c");
        expect("late but latched", keys(Rank.placed(ordered, [row("copy:x", "X", { tier: 1 })],
            "copy:x")), "copy:x program:a program:b program:c");

        // The selection stays on its row as rows come before it.
        const before = [row("a", "A"), row("b", "B", { is: "window:2" })];
        expect("pinned", Rank.pinned([row("x", "X")].concat(before), "window:2", 1), 2);
        expect("gone, kept in place", Rank.pinned(before, "window:9", 5), 1);

        // Home's likeliest picks, most first, ties by name.
        expect("likeliest", Rank.likeliest({ "b": 3, "a": 3, "c": 9, "gone": 50 },
            { a: "Alpha", b: "Beta", c: "Gamma" }), ["c", "a", "b"]);

        // Said as warnings, since Qt keeps ordinary logging quiet.
        for (const failure of failures)
            console.warn(failure);
        console.warn(failures.length > 0 ? `${failures.length} went wrong of ${asked}`
            : `all ${asked} held`);
        Qt.exit(failures.length > 0 ? 1 : 0);
    }
}
