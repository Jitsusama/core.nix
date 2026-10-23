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

        // Tiers: the name typed whole, then a word start, then a synonym,
        // whatever the score.
        expect("the name itself first", Rank.tier("Slack", " slack ", false), 0);
        expect("a word start", Rank.tier("Google Chrome", "chro", false), 1);
        expect("by another word", Rank.tier("Suspend", "sleep", true), 2);
        const found = [f("session:suspend", "Suspend", 2, 900), f("program:slack", "Slack", 1, 10),
            f("action:sl", "Sl", 0, 1)];
        expect("a tier wins over a score", keys(Rank.order(found, {})),
            "action:sl program:slack session:suspend");

        // The latch: the first pick holds at every start of the word.
        let latches = Rank.latch({}, "slack", "program:slack", 1);
        for (const typed of ["s", "sl", "sla", "slac", "slack"])
            expect(`latched at ${typed}`, Rank.latched(latches, typed), "program:slack");
        expect("not past it", Rank.latched(latches, "slacks"), "");
        // What's latched comes first, over a better tier.
        expect("the latch first", keys(Rank.order([f("action:sl", "Sl", 0), f("program:slack", "Slack", 1)],
            { latched: Rank.latched(latches, "sl") })), "program:slack action:sl");
        // One slip teaches nothing; a second pick running moves it.
        latches = Rank.latch(latches, "sl", "session:sleep", 1);
        expect("one slip", Rank.latched(latches, "sl"), "program:slack");
        latches = Rank.latch(latches, "sl", "program:slack", 1);
        latches = Rank.latch(latches, "sl", "session:sleep", 1);
        expect("a slip, back, a slip", Rank.latched(latches, "sl"), "program:slack");
        latches = Rank.latch(latches, "sl", "session:sleep", 1);
        expect("twice running", Rank.latched(latches, "sl"), "session:sleep");
        expect("the longer kept", Rank.latched(latches, "slack"), "program:slack");
        // Words latch whole, not by their starts.
        latches = Rank.latch({}, "vol up", "action:up", 1);
        expect("words whole", [Rank.latched(latches, "vol up"), Rank.latched(latches, "vol")],
            ["action:up", ""]);
        // What can't be taken back latches only past its first word.
        latches = Rank.latch({}, "power", "session:off", Rank.firstWord("Power off").length);
        expect("never short", [Rank.latched(latches, "pow"), Rank.latched(latches, "power")],
            ["", "session:off"]);

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
