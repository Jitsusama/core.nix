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

        // A program's own keys named as it is are one Neovim; a layer named
        // otherwise, or with no program beside it, stays.
        const neovim = row("program:nvim", "Neovim", { ownLayer: "neovim" });
        const kitty = row("program:kitty", "kitty", { ownLayer: "terminal" });
        const keysOfNeovim = row("shortcut-level:shortcuts:neovim", "Neovim", { layer: "neovim" });
        const keysOfTerminal = row("shortcut-level:shortcuts:terminal", "Terminal",
            { layer: "terminal" });
        expect("one Neovim", keys(Rank.keysHeld([keysOfNeovim, neovim, kitty, keysOfTerminal])),
            "program:nvim program:kitty shortcut-level:shortcuts:terminal");
        expect("its keys alone", keys(Rank.keysHeld([keysOfNeovim, kitty])),
            "shortcut-level:shortcuts:neovim program:kitty");

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
        // What's learned inside a step is kept apart from the top: "n" in
        // kitty's page meaning New Window Below lends nothing to "n" typed
        // at the top, and latches nothing, so it only orders that page.
        const inKitty = Rank.stepped("program:kitty", "n");
        expect("a step's query", [Rank.stepped("Program:Kitty", " N ") === inKitty,
            Rank.stepped("program:kitty", " "), Rank.isStepped(inKitty), Rank.isStepped("n")],
            [true, "", true, false]);
        const apart = picks({ [inKitty]: { "act:kitty:below": 4 }, n: { "program:notes": 2 } });
        expect("a step lends the top nothing", at(apart, "n"), "program:notes");
        expect("a step latches nothing", Rank.relatch({ n: "program:notes" }, apart, inKitty,
            all), { n: "program:notes" });
        // A category reads what was picked in it held alone, under what's
        // typed, and in it reached from another step, under its stepped
        // query, added together.
        expect("a category's picks joined", Rank.joined(
            { "part:cpu": { weight: 2, times: 2 } },
            { "part:cpu": { weight: 1, times: 1 }, "part:disk": { weight: 3, times: 3 } }), {
            "part:cpu": { weight: 3, times: 3 }, "part:disk": { weight: 3, times: 3 }
        });
        expect("a category's picks, none", Rank.joined({}, undefined), {});
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

        // A program's own act found from the top comes after anything else
        // found, whatever its tier, unless Joel picked it for what's typed
        // (docs/design.md): "image" is the Clipboard's picture before
        // GIMP's New Image from the Copied Picture.
        const act = (tier, fields) => ({
            choice: row("program-act:gimp:paste", "New Image from the Copied Picture",
                { sinks: true }),
            tier: tier, score: 9
        });
        const among = [act(0.5), f("program:magick", "Image Magick", 1, 1)];
        expect("an act after the rest", keys(Rank.order(among, {})),
            "program:magick program-act:gimp:paste");
        expect("an act picked keeps its tier", keys(Rank.order(among, {
            picked: { "program-act:gimp:paste": { weight: 0.5 } }
        })), "program-act:gimp:paste program:magick");
        const sinking = [row("program:a", "A", { tier: 0 }),
            row("program-act:gimp:paste", "New Image", { tier: 0.5, sinks: true })];
        expect("late above an act", keys(Rank.placed(sinking,
            [row("copy:image", "Image", { tier: 1 })], "")),
            "program:a copy:image program-act:gimp:paste");
        const picked = sinking.map(one => Object.assign({}, one, { kept: true }));
        expect("late below an act picked", keys(Rank.placed(picked,
            [row("copy:image", "Image", { tier: 1 })], "")),
            "program:a program-act:gimp:paste copy:image");

        // The selection stays on its row as rows come before it.
        const before = [row("a", "A"), row("b", "B", { is: "window:2" })];
        expect("pinned", Rank.pinned([row("x", "X")].concat(before), "window:2", 1), 2);
        expect("gone, kept in place", Rank.pinned(before, "window:9", 5), 1);

        // A collection's best find, gathered into the main search, says
        // what it belongs to and how many more it found, and its
        // collection's noun only where no kind says what it is already, so
        // a notice found at home reads "notice" once, never "notices"
        // beside it too (VKCF3Z, 2026-11-08).
        expect("one of a kind says no noun", Rank.gatheredSide("", 1, "notices", true), "");
        expect("one with no kind says where it's from",
            Rank.gatheredSide("", 1, "actions", false), "actions");
        expect("more found say how many", Rank.gatheredSide("", 3, "notices", true),
            "2 more notices");
        expect("what it belongs to leads", Rank.gatheredSide("REEF", 3, "sessions", true),
            "REEF \u00b7 2 more sessions");
        expect("and stands alone for one", Rank.gatheredSide("REEF", 1, "sessions", true),
            "REEF");

        // Home's likeliest picks, most first, ties by name.
        expect("likeliest", Rank.likeliest({ "b": 3, "a": 3, "c": 9, "gone": 50 },
            { a: "Alpha", b: "Beta", c: "Gamma" }), ["c", "a", "b"]);

        // The rows found are made in their order, each kind timed apart:
        // a clock that moves 1 ms a read and 3 more for each password made.
        let now = 0;
        const twoKinds = [row("p1", "Pass", { what: "password" }),
            row("a1", "Ada", { what: "person" }), row("p2", "Pin", { what: "password" })];
        const byKind = Rank.madeByKind(twoKinds, one => one.what, one => {
            now += one.what === "password" ? 3 : 0;
            return Object.assign({ copied: true }, one);
        }, () => now++);
        expect("made in the order found", byKind.made.map(one => one.copied && one.key),
            ["p1", "a1", "p2"]);
        expect("each kind's time", byKind.took, { password: 7, person: 1 });

        // A key named as Object's own is a key like any other, so a thing
        // named so is one row, said as what it threw where it throws so a
        // failure doesn't end the run.
        const tried = work => {
            try {
                return work();
            } catch (e) {
                return `threw ${e}`;
            }
        };
        expect("a thing named as Object's own is one row", tried(() => keys(Rank.once([
            row("constructor", "A"), row("__proto__", "B"), row("constructor", "A", { rich: 1 })
        ]))), "constructor __proto__");

        // Latching a pick costs a few walks over the rows picked, never a
        // walk for each latch: a Set in Qt's engine looks a key up by
        // walking every key it holds (H6EDLC, 2026-11-11). It's held to a
        // multiple of one walk making and reading an object of the keys,
        // the fastest of five batches, since a slower one is the builder's
        // other work or the collector.
        const fastest = work => {
            let least = Infinity;
            for (let batch = 0; batch < 5; batch += 1) {
                const began = Date.now();
                for (let round = 0; round < 10; round += 1) {
                    work();
                }
                least = Math.min(least, (Date.now() - began) / 10);
            }
            return least;
        };
        const everyRow = Array.from({ length: 1914 }, (_, at) =>
            row(`emoji:${at}`, `Emoji ${at}`));
        const everyKey = {};
        everyRow.forEach((one, at) => { everyKey[one.key] = at; });
        const walk = fastest(() => {
            const places = Object.create(null);
            everyRow.forEach((one, at) => { places[one.key] = at; });
            return everyRow.filter(one => places[one.key] !== undefined).length;
        });
        const pickedOf = {};
        everyRow.forEach((one, at) => {
            const asked = `q${at % 40}`;
            (pickedOf[asked] ?? (pickedOf[asked] = {}))[one.key] = { n: 1, c: 1, t: 0 };
        });
        const latchedAt = {};
        everyRow.forEach((one, at) => { latchedAt[`r${at}`] = one.key; });
        const there = Rank.standing(everyKey);
        const took = fastest(() => Rank.relatch(latchedAt, pickedOf, "em", there));
        console.info(`latching a pick took ${took} ms against a walk's ${walk}`);
        expect(`latching a pick takes ${took} ms, at most ten walks of ${walk} ms`,
            took <= 10 * walk + 1, true);

        // Said as warnings, since Qt keeps ordinary logging quiet.
        for (const failure of failures)
            console.warn(failure);
        console.warn(failures.length > 0 ? `${failures.length} went wrong of ${asked}`
            : `all ${asked} held`);
        Qt.exit(failures.length > 0 ? 1 : 0);
    }
}
