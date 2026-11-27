// How a row that ends the session arms before it's done, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `an-ending-row-arms-before-it-ends` check copies Arming.js beside this
// file and runs `qml -platform offscreen arming.qml`. Each case presses
// Return or clicks on a row, as the launcher hears them, and says what each
// press did and whether the row stands armed after it.
import QtQuick
import "Arming.js" as Arming

QtObject {
    // A Return's doubled press comes within this many ms of the first
    // (the key language's wait).
    readonly property int wait: 300

    // What a run of presses did, each { id, keys, at, counted } with the
    // keys the line had taken by then, said as "armed", "ends" or "" for
    // a doubled one, and whether `id` stood armed once they were done.
    function presses(steps) {
        let armed = Arming.rest();
        const acts = [];
        for (const step of steps) {
            const did = Arming.pressed(armed, step.id, { keysTaken: step.keys,
                counted: step.counted ?? 1, now: step.at, wait });
            armed = did.armed;
            acts.push(did.act);
        }
        const last = steps[steps.length - 1];
        return `${acts.join(" ")} | ${Arming.holds(armed, last.id, last.keys) ? "armed" : "not"}`;
    }

    Component.onCompleted: {
        const failures = [];
        let asked = 0;
        const expect = (what, got, wanted) => {
            asked++;
            if (got !== wanted)
                failures.push(`${what} gave ${JSON.stringify(got)}, not ${JSON.stringify(wanted)}`);
        };

        expect("nothing is armed at rest", Arming.holds(Arming.rest(), "", 0), false);
        // The first Return arms it and leaves it armed; the next, with no
        // key between, does it.
        expect("one Return arms", presses([{ id: "reboot", keys: 5, at: 0 }]), "armed | armed");
        expect("a second Return does it",
            presses([{ id: "reboot", keys: 5, at: 0 }, { id: "reboot", keys: 6, at: 2000 }]),
            "armed ends | not");
        // However long after: there's no time limit.
        expect("a second Return an hour later does it",
            presses([{ id: "reboot", keys: 5, at: 0 }, { id: "reboot", keys: 6, at: 3600000 }]),
            "armed ends | not");
        // Any other key lets it go, so the Return after arms it again.
        expect("a key between lets it go",
            presses([{ id: "reboot", keys: 5, at: 0 }, { id: "reboot", keys: 7, at: 2000 }]),
            "armed armed | armed");
        expect("any key lets it stand unarmed",
            Arming.holds(Arming.pressed(Arming.rest(), "reboot", { keysTaken: 5, counted: 1,
                now: 0, wait }).armed, "reboot", 6), false);
        // A Return on another row arms that one instead.
        expect("another row arms instead",
            presses([{ id: "reboot", keys: 5, at: 0 }, { id: "logout", keys: 6, at: 2000 }]),
            "armed armed | armed");
        // A Return doubled with the first, a key bounced, does nothing and
        // leaves it armed for a deliberate second.
        expect("a doubled Return does nothing",
            presses([{ id: "reboot", keys: 5, at: 0 }, { id: "reboot", keys: 6, at: 100 }]),
            "armed  | armed");
        expect("a Return after the doubled one does it",
            presses([{ id: "reboot", keys: 5, at: 0 }, { id: "reboot", keys: 6, at: 100 },
                { id: "reboot", keys: 7, at: 2000 }]),
            "armed  ends | not");
        // A click counts no key, so a click on the row a Return armed does
        // it, and a second click does what a second Return does.
        expect("a click after a Return does it",
            presses([{ id: "reboot", keys: 5, at: 0 },
                { id: "reboot", keys: 5, at: 2000, counted: 0 }]),
            "armed ends | not");
        expect("two clicks do it",
            presses([{ id: "reboot", keys: 5, at: 0, counted: 0 },
                { id: "reboot", keys: 5, at: 2000, counted: 0 }]),
            "armed ends | not");
        expect("a click after a key arms it again",
            presses([{ id: "reboot", keys: 5, at: 0 },
                { id: "reboot", keys: 6, at: 2000, counted: 0 }]),
            "armed armed | armed");

        // Return held still, as on an ask hidden while Joel shares: a row
        // saying it has nothing to do yet, with no run, whatever else it
        // holds; never a still row that runs, nor one not still with no run
        // whose Return does something else.
        const nothing = () => {};
        const still = [
            ["a hidden ask, still with no run", { still: true }, true],
            ["a still row with no run that asks for words", { still: true, given: nothing },
                true],
            ["a still row with no run that starts a command", { still: true, start: "x" }, true],
            ["a still row that runs, as the file chooser's", { still: true, stays: true,
                run: nothing }, false],
            ["one staying with no run that asks for words", { stays: true, given: nothing },
                false],
            ["one staying with no run that starts a command", { stays: true, start: "x" },
                false],
            ["one staying with no run that holds rows", { stays: true, category: {} }, false],
            ["one staying with no run that opens a place", { stays: true, opens: {} }, false],
            ["one with no run that closes", { name: "Ended" }, false],
            // A row gone, as Steady keeps it dimmed until the next key,
            // does nothing on Return, held or not, and says so nowhere.
            ["a hidden ask gone, kept dimmed", { still: true, gone: true, went: true }, false],
            ["no row", null, false]
        ];
        for (const [what, row, held] of still)
            expect(`Return on ${what} ${held ? "is" : "isn't"} held still`,
                Arming.holdsStill(row), held);
        // Asked from a terminal, a row held still is refused, saying why:
        // its own reason, or the launcher's when it gives none; any other
        // row isn't refused for it, which says null, so a held row with no
        // words anywhere is still refused rather than run.
        expect("a row held still is refused for its own reason",
            Arming.heldWhy({ still: true, stillWhy: "not while shared" }, "nothing sent"),
            "not while shared");
        expect("or the launcher's, giving none",
            Arming.heldWhy({ still: true }, "nothing sent"), "nothing sent");
        expect("while a row that acts isn't refused for it",
            Arming.heldWhy({ still: true, run: nothing, stillWhy: "x" }, "nothing sent"), null);
        expect("and one held with no words anywhere is held, not run",
            Arming.heldWhy({ still: true }, ""), "");
        // A way in from outside is refused where any step on it is held,
        // not only the last, or where the last one's Return's act is, so
        // nothing a held row holds is reached past it.
        const held = { still: true, stillWhy: "not while shared" };
        const acts = { stays: true, run: nothing };
        expect("a way through a held row is refused for it",
            Arming.wayHeldWhy([acts, held, acts], acts, "nothing sent"), "not while shared");
        expect("as is a way to a row whose Return's act is held",
            Arming.wayHeldWhy([acts, acts], held, "nothing sent"), "not while shared");
        expect("while a way held nowhere isn't refused for it",
            Arming.wayHeldWhy([acts, acts], acts, "nothing sent"), null);
        expect("nor is one that reaches nothing",
            Arming.wayHeldWhy([], null, "nothing sent"), null);

        // The foot's word for a held Return is let go only while the foot
        // still says it: another word said since, another place's, stays.
        const heldWord = [["\u21b5", "not while shared"]];
        expect("the foot saying why Return held lets it go",
            Arming.letsHeldGo([["\u21b5", "not while shared"]], heldWord), true);
        expect("but not another word said since",
            Arming.letsHeldGo([["q", "quit"]], heldWord), false);
        expect("nor an empty foot", Arming.letsHeldGo([], heldWord), false);
        expect("nor a word with no held Return said",
            Arming.letsHeldGo([["q", "quit"]], []), false);

        // A share ending lets go of any arm as well, as its start does
        // (untold): one made on an ask's row in the paced refresh after a
        // share began would let the first Return after it, no key between,
        // release on a line read before the share. This is what
        // Launcher.qml's unheld is told to leave; that it applies it isn't
        // run here, nor by any check, since none shares the screen.
        const ended = Arming.shareTurns(heldWord, heldWord);
        expect("shareTurns tells the launcher to leave nothing armed",
            JSON.stringify(ended.armed), JSON.stringify(Arming.rest()));
        expect("a share's end lets go of the foot's word for a held Return",
            ended.quiet, true);
        expect("but a share's end keeps another word said since",
            Arming.shareTurns([["q", "quit"]], heldWord).quiet, false);
        // Whether an arm a share's turn lets go of still stood, so the
        // launcher's journal says it went (Launcher.qml's unheld, untold).
        const standing = { key: "reboot", keys: 4, at: 100 };
        expect("an arm with no key taken since stood", Arming.stood(standing, 4), true);
        expect("one with a key taken since didn't", Arming.stood(standing, 5), false);
        expect("nor did none", Arming.stood(Arming.rest(), -1), false);

        // Which places may be kept to draw at once at the next opening
        // (Launcher.qml's keepPlace): never one that says it's never kept,
        // as Asks does, a second guard beside Wanted.keptPlaceListed's
        // refusing kept rows under a share (ISXEG8, 2026-11-19).
        expect("a place that says it's never kept isn't",
            Arming.keepsPlace({ keptNever: true }), false);
        expect("any other place is", Arming.keepsPlace({}), true);
        expect("and so is one no source has", Arming.keepsPlace(null), true);
        // Judged by the sources its steps come from too, so a place a
        // chord names otherwise than its source, its lookup unfound, still
        // isn't kept through a step of one that says it's never kept.
        const never = { keptNever: true };
        const stepOf = from => ({ row: { from } });
        expect("a place whose step is a never-kept source's isn't, its name unfound",
            Arming.keepsSteps(null, [stepOf({}), stepOf(never)]), false);
        expect("nor one whose named source says so",
            Arming.keepsSteps(never, [stepOf({})]), false);
        expect("nor one holding a never-kept source's row by shownBy, as For You an ask",
            Arming.keepsSteps(null, [stepOf({}), { row: { key: "ask", shownBy: never } }]),
            false);
        expect("nor one whose row says a kept from but a never-kept shownBy",
            Arming.keepsSteps(null, [{ row: { key: "ask", from: {}, shownBy: never } }]), false);
        expect("nor one holding an ask made hidden, which says no source but holds still",
            Arming.keepsSteps(null, [stepOf({}), { row: { key: "ask", still: true } }]), false);
        expect("nor one holding any row marked still, though it runs or has gone",
            [{ still: true, run: () => {} }, { key: "ask", still: true, gone: true }]
                .map(row => Arming.keepsSteps(null, [stepOf({}), { row }])).join(" "),
            "false false");
        expect("one whose steps and name keep is, a step with no source among them",
            Arming.keepsSteps(null, [stepOf({}), { row: {} }]), true);

        // Whether run changes no source, in its own order, for the line's
        // key run from inside a step: what only asks, steps in or does
        // nothing changes none, and the rest may act.
        const run = () => {};
        // Each is [choice, inside, heldStill], said as what each gave.
        const none = cases => cases.map(one => Arming.changesNone(...one)).join(" ");
        expect("a verb asking for a target inside a step, or for words, changes none",
            none([[{ targets: true }, true, false], [{ targets: true, run, verb: "more" }, true,
                false], [{ given: true, run }, false, false]]), "true true true");
        expect("nor does a gone row, one held still, or a category or group stepped into",
            none([[{ gone: true, run }, true, false], [{ run, returns: "run" }, true, true],
                [{ category: true }, true, false], [{ group: true }, false, false]]),
            "true true true true");
        // Each in run's order: gone before a Return's act or a start, and a
        // category's step-in after a start.
        expect("a gone row changes none whatever its Return does, and a category's start acts",
            none([[{ gone: true, returns: { run } }, true, false],
                [{ gone: true, start: "vol " }, true, false],
                [{ category: true, start: "vol " }, true, false]]), "true true false");
        expect("a start put in the field, or a Return's act, may act, though they ask too",
            none([[{ start: "vol ", given: true }, true, false],
                [{ returns: { run }, targets: true }, true, false]]), "false false");
        expect("but a Return's act that only asks for words changes none, as ranReturn asks",
            none([[{ returns: { given: true, run }, run }, true, false]]), "true");
        expect("but a target verb with its own run, one outside a step, or a plain run may act",
            none([[{ targets: true, run, verb: "close" }, true, false],
                [{ targets: true }, false, false], [{ run }, true, false]]), "false false false");
        // The rest of run's order: held still before a start, a start before a
        // Return's act, and a Return's act before a category's step-in.
        expect("a row held still changes none though it starts, a start acts though Return asks",
            none([[{ start: "x" }, true, true], [{ start: "x", returns: { given: true } }, true,
                false], [{ category: true, returns: { run } }, true, false]]), "true false false");

        // What Return does on the selected row, as onAccepted asks: a held
        // row is held before anything else it might be stepped into,
        // lined or landed on, as every other way in asks it first.
        const every = { arming: true, steps: true, ends: true, lands: true };
        expect("a held row is held, whatever else it is",
            Arming.returnAct(Object.assign({ held: true }, every)), "held");
        expect("but an answer being given is given",
            Arming.returnAct(Object.assign({ asking: true, held: true }, every)), "gave");
        expect("even just after a landing",
            Arming.returnAct(Object.assign({ asking: true, landed: true }, every)), "gave");
        expect("and a Return doubling a landing does nothing more",
            Arming.returnAct(Object.assign({ landed: true, held: true }, every)), "");
        expect("one not held arms first", Arming.returnAct(every), "arming");
        expect("then steps in", Arming.returnAct({ steps: true, ends: true, lands: true }),
            "entered");
        expect("then lines", Arming.returnAct({ ends: true, lands: true }), "lined");
        expect("then lands", Arming.returnAct({ lands: true }), "landed");
        expect("and otherwise runs", Arming.returnAct({}), "ran");
        // A row whose thing has gone, as Firmware's Install once it's in
        // ("restart to finish"), is run, which does nothing, rather than
        // armed, stepped into, landed on or lined: a line written for it
        // would do it again (C2597D63, 2026-11-20).
        expect("a gone row is only run",
            Arming.returnAct(Object.assign({ gone: true }, every)), "ran");
        expect("even one held",
            Arming.returnAct(Object.assign({ gone: true, held: true }, every)), "ran");
        expect("though an answer being given is still given",
            Arming.returnAct(Object.assign({ gone: true, asking: true }, every)), "gave");
        expect("and a Return doubling a landing still does nothing more",
            Arming.returnAct(Object.assign({ gone: true, landed: true }, every)), "");

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        console.log(`${asked - failures.length} of ${asked} passed`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
