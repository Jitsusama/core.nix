// How the launcher makes again the steps it holds as what they came from
// changes, run in Qt's own JavaScript engine, the one the shell runs it in.
// The `the-launcher-remakes-what-it-holds` check copies
// modules/quickshell/launcher/Held.js beside this file and runs
// `qml -platform offscreen held.qml`. Each case is a promise Joel leans on
// as agents change under a place he's stepped into: the step says the row
// as it is now, and a row that now offers nothing is left rather than
// kept showing what it offered before.
import QtQuick
import "Held.js" as Held

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    // A row known by its key, saying `says`, offering the rows `offers`
    // names, or nothing where it offers none.
    function row(key, says, offers) {
        return { key: key, says: says, offers: offers ?? null };
    }

    // Each step `ways.into` was asked about, by its depth, so a case can
    // say it was asked at the step it means.
    property var askedAt: []
    // Each step `ways.keep` was asked to keep, by its row's key and how
    // many rows it's kept with, so a case can say a step's rows were kept
    // as gone: with none, which the launcher's Steady.kept dims each of.
    property var keptWith: []

    // The launcher's ways, as Held.js asks them: a row is the one held by
    // its key, a thing offers what its verbs give, as the launcher's
    // stepRows reads them, or else its own rows, and rows are kept as
    // offered.
    // Each says what it was given, so a call with the wrong step fails.
    readonly property var ways: ({
        same: (one, was) => {
            if (!one || !was || one.key === undefined) {
                throw new Error(`same was given ${JSON.stringify([one, was])}`);
            }
            return one.key === was.key;
        },
        into: (thing, at, steps) => {
            if (!Array.isArray(steps) || at < 0 || at >= steps.length) {
                throw new Error(`into was given step ${at} of ${steps?.length}`);
            }
            askedAt = askedAt.concat([at]);
            const rows = typeof thing.verbs === "function" ? thing.verbs() : thing.offers;
            return Held.offers(rows, thing.places ?? 0, thing.empty === true);
        },
        keep: (step, into) => {
            if (!step?.row || !Array.isArray(into)) {
                throw new Error(`keep was given ${JSON.stringify([step, into])}`);
            }
            keptWith = keptWith.concat([`${step.row.key}:${into.length}`]);
            return into;
        }
    })

    // Each step as it reads: its row's key and what it says, then the keys
    // of the rows it offers.
    function read(steps) {
        return steps.map(step => `${step.row.key}=${step.row.says}[`
            + `${step.rows.map(one => one.key).join(" ")}]`).join(" ");
    }

    function run() {
        const failures = [];
        const check = (what, got, want) => {
            if (got !== want) {
                failures.push(`${what}: "${got}", not "${want}"`);
            }
        };

        // Sessions, stepped into a session, stepped into its verbs.
        const verbs = [row("stop", "Stop"), row("look", "Look")];
        const working = row("s1", "working", verbs);
        const steps = [
            { row: row("agents", "Agents"), rows: [working] },
            { row: working, rows: verbs }
        ];

        const asNow = Held.remade(steps, 1, [row("s1", "idle", [row("look", "Look")])], ways);
        check("a step held says its row as it is now, and what it offers now",
            read(asNow.steps), "agents=Agents[s1] s1=idle[look]");
        check("and every step was made again", `${asNow.all} ${asNow.leave}`, "true -1");

        const gone = Held.remade(steps, 1, [row("s2", "working", verbs)], ways);
        check("a step whose row is gone is kept as it was", read(gone.steps), read(steps));
        check("and isn't left", `${gone.all} ${gone.leave}`, "false -1");
        // The step above shows a row gone as a copy marked so, as
        // Steady.kept dims it, offering nothing; that's the row gone.
        const goneCopy = Object.assign(row("s1", "working", []), { gone: true, went: true });
        const dimmed = Held.remade(steps, 1, [goneCopy], ways);
        check("a step whose row the step above shows gone is kept, not left",
            `${dimmed.all} ${dimmed.leave} ${read(dimmed.steps)}`, `false -1 ${read(steps)}`);
        // A row its source serves dimmed itself, as Firmware's act done
        // with, is still that row, as it is now.
        const servedDim = Object.assign(row("s1", "done", [row("look", "Look")]), { gone: true });
        const doneNow = Held.remade(steps, 1, [servedDim], ways);
        check("a step whose row its source serves dimmed says the row as it is now",
            `${doneNow.all} ${read(doneNow.steps)}`, "true agents=Agents[s1] s1=done[look]");

        const nothing = Held.remade(steps, 1, [row("s1", "ended")], ways);
        check("a step whose row now offers nothing is left", nothing.leave, 1);
        check("and not every step was made again", nothing.all, false);

        const deeper = [
            ...steps,
            { row: row("look", "Look"), rows: [row("diff", "Diff")] }
        ];
        const below = Held.remade(deeper, 1, [row("s1", "idle", [row("look", "Look")])], ways);
        check("a step below whose row now offers nothing is left, the ones above kept",
            `${below.leave} ${read(below.steps.slice(0, 2))}`, "2 agents=Agents[s1] s1=idle[look]");

        const typed = [steps[0], Object.assign({}, steps[1], { typed: "s1" })];
        const found = Held.remade(typed, 1, [], ways);
        check("a step found by what was typed is made again from its own row",
            `${found.all} ${read(found.steps)}`, `true ${read(steps)}`);
        // Its own row offered now, it's taken as it is now.
        const typedNow = Held.remade(typed, 1, [row("s1", "idle", [row("look", "Look")])],
            ways);
        check("a step found by what was typed takes its row as offered now",
            read(typedNow.steps), "agents=Agents[s1] s1=idle[look]");
        // Its own row offering nothing now, as a session ended, is kept with
        // its rows gone, and the steps above it still made again.
        keptWith = [];
        const typedSpent = [steps[0], { row: row("s1", "ended"), rows: verbs, typed: "s1" }];
        const foundSpent = Held.remade(typedSpent, 1, [], ways);
        check("a step found by what was typed whose row offers nothing keeps its rows gone",
            `${foundSpent.all} ${foundSpent.leave} ${keptWith.join(" ")}`, "true -1 s1:0");

        // A source's own row, stepped into: Agents' session, from Agents.
        const ownNow = Held.own(steps, 1, [row("s1", "idle", [row("look", "Look")])], ways);
        check("a source's own row stepped into says the row as the source offers it now",
            read(ownNow.steps), "agents=Agents[s1] s1=idle[look]");
        // A place opened straight holds a step whose row has no key, as
        // Launcher.qml's placeStep makes it, so it's never one of its
        // members, though the launcher's same tells a keyless row by name.
        const byName = Object.assign({}, ways, { same: (one, was) => (was.key ?? "") === ""
            ? one.name === was.name : one.key === was.key });
        const place = { row: { key: "", name: "agents", title: "Agents", from: "Agents" },
            rows: [working] };
        const namedLikeIt = Object.assign(row("s9", "idle", []), { name: "agents" });
        const ownPlace = Held.own([place], 0, [namedLikeIt, working], byName);
        check("a place's own step stays the place, whatever a member is named",
            `${ownPlace.leave} ${ownPlace.steps[0].row.title}`, "-1 Agents");
        // Found by a search, the row says who gave it, which the source's own row doesn't.
        const given = [steps[0], Object.assign({}, steps[1], {
            row: Object.assign({}, working, { from: "Agents" })
        })];
        const ownGiven = Held.own(given, 1, [row("s1", "idle", [row("look", "Look")])], ways);
        check("a source's own row taken as it is now still says who gave it",
            `${ownGiven.steps[1].row.from} ${read(ownGiven.steps)}`,
            "Agents agents=Agents[s1] s1=idle[look]");
        // Found by a search, the launcher gave the row its own fields, as
        // its side and the verbs it found, said in `foundAs`; they're kept
        // over the row as the source offers it now, and nothing else the
        // source offered then and no longer does.
        const asked = () => [row("look", "Look")];
        const gathered = [steps[0], Object.assign({}, steps[1], {
            row: Object.assign({}, working, { from: "Agents", side: "in Agents", verbs: asked,
                targets: ["kiln"], foundAs: { from: "Agents", side: "in Agents", verbs: asked } })
        })];
        askedAt = [];
        const offeredNow = Object.assign(row("s1", "idle", [row("look", "Look")]), {
            side: "", verbs: () => []
        });
        // Its match, Look, is among what the source searches, not what it lists.
        const looked = () => [offeredNow, row("look", "Look now")];
        // The launcher's ways for a source that searches what `looked` gives.
        const lookingIn = all => Object.assign({}, ways, { looked: all });
        const ownGathered = Held.own(gathered, 1, [offeredNow], lookingIn(looked));
        const kept = ownGathered.steps[1].row;
        const keys = rows => rows.map(one => `${one.key}=${one.says}`).join(" ");
        check("a row found by a search keeps what the launcher gave it",
            `${kept.side} ${keys(kept.verbs())} ${kept.says}`, "in Agents look=Look now idle");
        check("but not what the source offered then and doesn't now", kept.targets, undefined);
        check("asked what it offers at the step it is", askedAt.join(" "), "1");
        // `given` is a row's way of asking for words (Launcher.qml's
        // asking), so what the launcher gave is never said in it.
        check("nor taken for a row asking for words", typeof kept.given, "undefined");
        // Made again a second time, it still knows what the launcher gave.
        askedAt = [];
        const keptAgain = Held.own(ownGathered.steps, 1, [offeredNow], lookingIn(looked))
            .steps[1].row;
        check("made again twice, it still keeps what the launcher gave it",
            `${keptAgain.side} ${keys(keptAgain.verbs())}`, "in Agents look=Look now");
        // What the search found is each as the source offers it now, so an
        // ask hidden as a share begins is listed hidden, never as it was
        // found; one the source neither lists nor searches is kept as
        // found, as a notice found past the page is, never dropped.
        const matched = () => [row("k1", "Kiln"), row("k2", "Kettle")];
        const searched = [steps[0], Object.assign({}, steps[1], {
            row: Object.assign({}, working, { verbs: matched, foundAs: { from: "Asks",
                verbs: matched, landsIn: working } })
        })];
        const hiddenNow = [row("s1", "hidden"), row("k1", "hidden")];
        const ownSearched = Held.own(searched, 1, hiddenNow, lookingIn(() => hiddenNow))
            .steps[1];
        check("what a search found is listed as the source offers it now",
            `${keys(ownSearched.row.verbs())} ${keys(ownSearched.rows)}`,
            "k1=hidden k2=Kettle k1=hidden k2=Kettle");
        check("and where it lands is the row as it is now", ownSearched.row.landsIn?.says,
            "hidden");
        const shownNow = [row("s1", "hidden"), row("k1", "shown")];
        const searchedAgain = Held.own([steps[0], ownSearched], 1, shownNow,
            lookingIn(() => shownNow)).steps[1].row;
        check("made again twice, it lists them as they are then", keys(searchedAgain.verbs()),
            "k1=shown k2=Kettle");
        // One the source searches beyond what it lists, as Sound's found
        // by ear, is as that search has it now.
        const beyond = Held.own(searched, 1, hiddenNow,
            lookingIn(() => hiddenNow.concat([row("k2", "boiling")]))).steps[1].row;
        check("one it searches beyond what it lists is as it stands there",
            keys(beyond.verbs()), "k1=hidden k2=boiling");
        // All it searches is read only for a match it doesn't list.
        let lookedFor = 0;
        Held.own(searched, 1, hiddenNow.concat([row("k2", "hidden")]),
            lookingIn(() => { lookedFor += 1; return []; }));
        check("what it searches isn't read while it lists every match", lookedFor, 0);
        // A match it ranks flat, as Agents' search gives a session found in
        // its tree, is ranked as it stands.
        const inTree = Object.assign(row("k1", "in the tree"),
            { ranked: () => row("k1", "flat") });
        const flat = Held.own(searched, 1, [row("s1", "hidden"), inTree], ways).steps[1].row;
        check("a match it ranks is ranked as it stands", keys(flat.verbs()),
            "k1=flat k2=Kettle");
        // A copy of such a row under a key of its own, as For You's want
        // row is, ranks to the row it was copied from, so it isn't ranked.
        const copied = Object.assign(row("k1", "wanted"), { ranked: () => row("a1", "flat") });
        const notRanked = Held.own(searched, 1, [row("s1", "hidden"), copied], ways)
            .steps[1].row;
        check("but not where ranking gives another row", keys(notRanked.verbs()),
            "k1=wanted k2=Kettle");
        // The step's own row gone, what it found is still as offered now.
        const bestGone = Held.own(searched, 1, [row("k1", "hidden")], ways).steps[1].row;
        check("the row it found by gone, its matches are still as offered now",
            keys(bestGone.verbs()), "k1=hidden k2=Kettle");
        const ownNothing = Held.own(steps, 1, [row("s1", "ended")], ways);
        check("a source's own row that now offers nothing is left",
            `${ownNothing.leave} ${ownNothing.all}`, "1 false");
        const ownGone = Held.own(steps, 1, [], ways);
        check("a source's own row gone is kept, saying what it offers as it was",
            `${ownGone.leave} ${ownGone.all} ${read(ownGone.steps)}`, `-1 true ${read(steps)}`);
        // Offering no rows is offering nothing, as a step in is refused,
        // but for one whose vocabulary names places, or that says what it
        // holds when empty.
        const ownEmptied = Held.own(steps, 1, [row("s1", "idle", [])], ways);
        check("a source's own row now offering no rows is left",
            `${ownEmptied.leave} ${ownEmptied.all}`, "1 false");
        const placed = Object.assign(row("s1", "idle", []), { places: 2 });
        const ownPlaced = Held.own(steps, 1, [placed], ways);
        check("but kept where it holds places its vocabulary names",
            `${ownPlaced.leave} ${read(ownPlaced.steps)}`, "-1 agents=Agents[s1] s1=idle[]");
        const saysEmpty = Object.assign(row("s1", "idle", []), { empty: true });
        check("or says what it holds when empty", Held.own(steps, 1, [saysEmpty], ways).leave,
            -1);
        check("and a place stepped into that holds nothing but places offers no rows",
            JSON.stringify(Held.offers(null, 1, false)), "[]");
        const spent = [steps[0], { row: row("s1", "ended"), rows: verbs }];
        // As a collection stepped into whose last row went: its own row is
        // never among what it offers, so its rows are kept, each gone.
        keptWith = [];
        const ownSpent = Held.own(spent, 1, [], ways);
        check("a source's own row gone, offering nothing, keeps its rows, each gone",
            `${ownSpent.leave} ${ownSpent.all} ${keptWith.join(" ")}`, "-1 true s1:0");
        // A step found by what was typed whose row is still offered, but
        // offers nothing now, is left as any other is.
        const typedEnded = Held.remade(typed, 1, [row("s1", "ended")], ways);
        check("a typed step whose row is offered but offers nothing is left",
            `${typedEnded.leave} ${typedEnded.all}`, "1 false");

        leaving(check, steps, nothing, below);
        guessing(check);

        for (const failure of failures) {
            console.warn(`FAIL ${failure}`);
        }
        Qt.exit(failures.length === 0 ? 0 : 1);
    }

    // Whether Return holds after a step left by itself, as an ask's during
    // a share: its held step offers nothing, so the launcher leaves it,
    // and the ask, hidden, is no row to land back on. Joel's second
    // Return, meant for its Release It, must never act on the row the
    // selection was left on instead. Each told to `check`.
    function guessing(check) {
        const ask = { key: "ask:7", at: 2 };
        const onIt = (from, landed) => Held.onItsRow(from, landed);
        check("a leave that lands back on the row stepped in from, as its key says, is on it",
            onIt(ask, "ask:7"), true);
        check("one that lands on another row, as one hidden leaves it, is not",
            onIt(ask, "copies"), false);
        check("and one that lands on no row at all", onIt(ask, ""), false);
        check("a row stepped in from with no key is never known to be landed on",
            [onIt({ key: "", at: 0 }, ""), onIt({ at: 0 }, ""), onIt(undefined, "x")].join(" "),
            "false false false");
        // A row clicked into isn't selected, so a step's record stands on
        // the headline under the clicked row's key: where the selection
        // lands back says nothing.
        check("a landing on the headline under another row's key is not on it",
            onIt({ key: "ask:7", at: -1 }, "named:answer"), false);
        // The guess a leave makes, as heldAgain asks for it: a leave at 5
        // keys, landing 2 deep.
        const shown = g => `${g.keys}@${g.at} d${g.depth}${g.broken ? " broken" : ""}`;
        const landing = (before, landed, more) => Held.landedAfter(before, ask, landed,
            Object.assign({ keys: 5, open: true, depth: 2, byKey: false }, more));
        const after = (before, landed, more) => shown(landing(before, landed, more));
        const none = Held.unguessed();
        const left = landing(none, "copies");
        check("a landing on another row is guessed at its keys and depth", shown(left),
            "5@0 d2");
        check("a landing back on its own row guesses nothing", after(none, "ask:7"),
            "-1@0 d-1");
        check("a shut launcher guesses nothing", after(none, "copies", { open: false }),
            "-1@0 d-1");
        // A leave made by the refresh of Joel's last key's act (Held.keyed)
        // is one he watched.
        check("a leave Joel's key caused guesses nothing",
            after(none, "copies", { byKey: true }), "-1@0 d-1");
        check("but keeps a guess an earlier leave left pending",
            after(left, "copies", { byKey: true, depth: 1 }), "5@0 d1");
        // A second leave before any key, landing back on its row a step
        // up, leaves the Return still meant for the first step's ask.
        check("a leave back on its row keeps a guess no key has answered, at the new depth",
            after(left, "ask:7", { depth: 1 }), "5@0 d1");
        // A first leave threw, and a second closed whole before any key: the
        // Return is still meant for the first step, but the selection is
        // whole again, so it holds once, not until another key.
        check("a leave back on its row keeps a broken guess whole again",
            after(Held.whileLeaving(5, true, none), "ask:7", { depth: 1 }), "5@0 d1");
        check("but not once a key came between", after(left, "ask:7", { keys: 6 }),
            "-1@0 d-1");
        check("a leave that left nothing keeps it too", after(left, null), "5@0 d2");
        check("a leave that left nothing, with nothing pending, guesses nothing",
            after(none, null), "-1@0 d-1");
        check("a leave runs broken, so a throw partway holds",
            [shown(Held.whileLeaving(5, true, none)),
                shown(Held.whileLeaving(5, false, none))].join(", "),
            "5@0 d-1 broken, -1@0 d-1");
        // A busy source throws again on each refresh: the guess stays at
        // the first throw's keys, so the one key its foot asks for ends it.
        const threw = Held.whileLeaving(5, true, none);
        check("a broken guess standing is kept at its keys, a key on",
            [shown(Held.whileLeaving(5, true, threw)),
                shown(Held.whileLeaving(6, true, threw))].join(", "),
            "5@0 d-1 broken, 5@0 d-1 broken");
        check("and a throw once Joel has pressed more is a new one",
            shown(Held.whileLeaving(7, true, threw)), "7@0 d-1 broken");
        // What the journal says once the leave is judged, as returnUnder
        // would answer the Return next: Joel's own key's leave holds
        // nothing, though it can still leave a pending guess, and no key's
        // act landing back on its row holds nothing.
        check("a leave is journalled by its act and whether a Return next holds",
            [Held.leaveSaid("a key's act", landing(none, "copies", { byKey: true })),
                Held.leaveSaid("a key's act", landing(left, "copies", { byKey: true })),
                Held.leaveSaid("no key's act", landing(none, "ask:7")),
                Held.leaveSaid("a key's act, joined", left)].join(" | "),
            "a key's act; nothing holds | a key's act; a Return next holds"
                + " | no key's act; nothing holds | a key's act, joined; a Return next holds");

        // Held.movesClock: only a key that may act moves the clock; a
        // modifier alone and a key the find types itself leave it at the
        // key before them.
        check("a modifier alone, and a letter, Space or é in the find, keep the clock",
            [Held.movesClock(true, false, ""), Held.movesClock(false, true, "a"),
                Held.movesClock(false, true, "Space"), Held.movesClock(false, true, "")]
                .join(" "), "false false false false");
        // Backspace on an empty find leaves it for normal, so it may act.
        check("Escape and Backspace in the find, j and an unnamed key in normal move it",
            [Held.movesClock(false, true, "Escape"), Held.movesClock(false, true, "Backspace"),
                Held.movesClock(false, false, "j"), Held.movesClock(false, false, "")]
                .join(" "), "true true true true");
        // Held.movedSaid: j the 4th key taken, the clock's. The prompt
        // clears its word as each key comes, and says a move once it's read.
        check("a move said of the clock's own key is its",
            Held.movedSaid(false, 4, 4, true), true);
        check("a Shift after j, clearing the prompt's word, leaves j's move",
            Held.movedSaid(true, 4, 5, false), true);
        check("a move said of another key isn't the clock's key's",
            Held.movedSaid(false, 4, 5, true), false);

        // Held.changeDoes, as refreshed and reanswer ask it of each change
        // and answer heard, keyedWithin 50 ms.
        const does = more => Held.changeDoes(Object.assign(
            { ago: 10, moved: false, within: 50, shown: true, waiting: false }, more)).says;
        check("a change shown within keyedWithin of an acting key is its act, to 49 ms",
            [does({}), does({ ago: 49 })].join(" "), "act act");
        check("one at 50 ms or after a move joins the source's acts",
            [does({ ago: 50 }), does({ moved: true })].join(" "), "joins joins");
        // A share's change heard before the key, waiting on the pace or a
        // fill's find, is taken into the key's refresh, so would empty the
        // step as if Joel's key had.
        check("an act whose source already waits on a refresh is recorded joined",
            does({ waiting: true }), "joined act");
        // A change to rows the launcher doesn't show, or an answer, which
        // never records an act, is the key's within keyedWithin, so leaves
        // its act his.
        check("one not shown within keyedWithin is kept, past it joins",
            [does({ shown: false }), does({ shown: false, ago: 50 })].join(" "),
            "kept joins");
        // A move or lateness outranks what waits, and not being shown does.
        check("a move joins whatever waits, and one not shown is kept though one waits",
            [does({ moved: true, waiting: true }), does({ shown: false, waiting: true })]
                .join(" "), "joins kept");
        check("what it says is told apart for the launcher, which matches no words",
            ["joins", "joined act", "act"].map(says => {
                const change = Held.changeDoes({ ago: says === "joins" ? 50 : 10,
                    moved: false, within: 50, shown: true, waiting: says === "joined act" });
                return `${change.joins} ${change.mixed}`;
            }).join(" | "), "true false | false true | false false");

        // Held.actOf, Held.keyed and Held.mixIn, as the launcher's keyedActs
        // hold a key's act until the refresh that shows it, each case on
        // acts of its own.
        const share = { name: "share" };
        const asks = { name: "asks" };
        const acts = () => [{ source: asks, heard: 40 }];
        check("a leave of the act's own source, under its key, is the key's",
            [Held.actOf(acts(), asks, 40), Held.keyed(acts(), asks, 40)].join(" | "),
            "a key's act | true");
        check("but not once another key was heard, nor another source's",
            [Held.actOf(acts(), asks, 90), Held.actOf(acts(), share, 40),
                Held.keyed(acts(), asks, 90), Held.keyed(acts(), share, 40)].join(" | "),
            "a key's act, another key since | no key's act | false | false");
        const shared = acts();
        Held.mixIn(shared, share);
        check("another source's change joins none of this one's acts",
            Held.keyed(shared, asks, 40), true);
        const joined = acts();
        Held.mixIn(joined, asks);
        check("a change the key doesn't own, joining its act, makes the leave no key's",
            [Held.actOf(joined, asks, 40), Held.keyed(joined, asks, 40)].join(" | "),
            "a key's act, joined | false");
        // The chain heldAgain runs: a share's change joins the key's act,
        // so the leave its refresh makes off its row is guessed, and the
        // journal says a Return next holds.
        const chained = landing(none, "copies", { byKey: Held.keyed(joined, asks, 40) });
        check("a joined act's leave off its row is guessed, and journalled as holding",
            Held.leaveSaid(Held.actOf(joined, asks, 40), chained),
            "a key's act, joined; a Return next holds");

        // Held.returnUnder, as the field's Return asks it: 2 deep, no words
        // asked for, and the key-timing wait 300 ms.
        const press = (guess, keys, at, more) => Held.returnUnder(guess,
            Object.assign({ keys, at, depth: 2, asking: false }, more), 300);
        const said = s => `${s.holds} ${s.next.keys}@${s.next.at}`;
        const first = press(left, 6, 1000);
        check("the first key after a guessed landing, a Return, holds, and marks when",
            said(first), "true 6@1000");
        check("a guess a Return was held under is no longer one a leave keeps",
            after(first.next, "ask:7", { keys: 6 }), "-1@0 d-1");
        // One Return more costs less than a row Joel never chose acting,
        // so the first holds however late it comes.
        check("a first Return holds however late", said(press(left, 6, 60000)),
            "true 6@60000");
        // A guess made before the field's first key is 0, a count still.
        check("a landing guessed before any key holds the first",
            said(press(Object.assign({}, left, { keys: 0 }), 1, 1000)), "true 1@1000");
        check("a Return after another key first acts, as Joel moved or typed since",
            said(press(left, 7, 1000)), "false -1@0");
        check("a Return with nothing guessed acts", said(press(none, 0, 1000)), "false -1@0");
        // Clicks, walks and chords end the guess themselves; the depth holds
        // it to where it landed should anything else move the steps
        // without a key, so the Return is then on a row Joel was brought to.
        check("a Return at another depth acts", said(press(left, 6, 1000, { depth: 3 })),
            "false -1@0");
        check("words asked for are given, and the guess ends",
            said(press(left, 6, 1000, { asking: true })), "false -1@0");
        // The Return just held, doubled as a key bounces or is pressed twice
        // for one, holds once more within the wait, then lets the next act.
        check("the Return straight after a held one, within the wait, holds, and ends it",
            said(press(first.next, 7, 1250)), "true -1@0");
        check("nor at the wait itself", said(press(first.next, 7, 1300)), "false -1@0");
        check("nor after another key between", said(press(first.next, 8, 1100)),
            "false -1@0");
        // After a throw, the selection is half made, so every Return holds,
        // at any depth and past the wait, until another key.
        const broken = press(threw, 6, 15000, { depth: 0 });
        const again = press(broken.next, 7, 40000, { depth: 0 });
        check("after a leave that threw, Return holds until another key, however late",
            [said(broken), said(again), said(press(again.next, 9, 40100))].join(", "),
            "true 6@15000, true 7@40000, false -1@0");
        // Shift or Ctrl alone chose nothing, so the Return after it holds.
        check("a modifier alone between leaves the Return held",
            said(press(Held.passedOver(left, true), 7, 1000)), "true 7@1000");
        check("and the doubled Return after a held one",
            said(press(Held.passedOver(first.next, true), 8, 1100)), "true -1@0");
        // Any other key, é and Delete among those with no name, may move
        // the rows, so Joel's Return is on the row it chose.
        check("any other key between lets it act",
            said(press(Held.passedOver(left, false), 7, 1000)), "false -1@0");
        check("a modifier alone with nothing guessed changes nothing",
            shown(Held.passedOver(none, true)), "-1@0 d-1");
    }

    // Where a step that offers nothing now leaves to, as `nothing` and
    // `below` made Agents' steps again, from `steps` and a step deeper,
    // each told to `check`.
    function leaving(check, steps, nothing, below) {
        const how = outermost => ({ outermost });
        const said = to => `${to.keep}`;

        check("nothing to leave keeps every step",
            said(Held.leaveTo({ steps: steps, leave: -1, all: true }, how(false))), "-1");
        check("a step offering nothing is left, back to the step above it",
            said(Held.leaveTo(nothing, how(false))), "1");
        check("and a step below one that does, back to it",
            said(Held.leaveTo(below, how(false))), "2");
        const spentTop = { steps: steps, leave: 0, all: false };
        check("a spent first step leaves to home", said(Held.leaveTo(spentTop, how(false))),
            "0");
        check("steps it doesn't leave are as made, and handed back as made",
            read(Held.leaveTo(below, how(false)).steps), read(below.steps));
        // A thing landed in from a search, `through`, is left back to the
        // search, past the collection it was found in, as Joel's - leaves it,
        // but never past a dialog's own collection, the outermost.
        const through = { steps: [steps[0], Object.assign({}, steps[1], { through: true })],
            leave: 1, all: false };
        check("a spent thing landed in from a search leaves past its collection",
            said(Held.leaveTo(through, how(false))), "0");
        check("but never past the outermost", said(Held.leaveTo(through, how(true))), "1");
        // The outermost is itself never left: a dialog's collection is a
        // place's own step, with no key, which own keeps with its rows
        // each gone, its row as entered, rather than leaving.
        const placeStep = { row: { key: "", says: "Chooser" }, rows: [row("f", "f")] };
        keptWith = [];
        const spentPlace = Held.own([placeStep], 0, [], ways);
        check("a place's own step offering nothing is kept, its rows each gone",
            `${spentPlace.leave} ${spentPlace.all} ${keptWith.join(" ")} `
                + read(spentPlace.steps), "-1 true :0 =Chooser[]");
        const left = Held.leaveTo(nothing, how(false));

        // What every history forgets: the places through the step that
        // offers nothing, as they read before it was made again and after,
        // and where Joel lands, read both ways, each as its depths.
        const before = steps.map(step => Object.assign({}, step, {
            row: Object.assign({}, step.row, { says: `${step.row.says} before` })
        }));
        const forgets = (made, how) => {
            const at = Held.forgets(made, Held.leaveTo(made, how), before, made.steps);
            return at === null ? "none" : [at.landed, at.left].map(both =>
                both.map(one => one.length).join(",")).join(" ");
        };
        check("nothing offering nothing forgets nothing",
            forgets({ steps: steps, leave: -1, all: true }, how(false)), "none");
        check("a step left forgets the places through it, landing above it",
            forgets(nothing, how(false)), "1,1 2,2");
        check("a spent first step forgets the places through it, landing at home",
            forgets(spentTop, how(false)), "0,0 1,1");
        check("a spent thing found by a search forgets only the places through it",
            forgets(through, how(false)), "0,0 2,2");
        check("where the outermost keeps its collection too", forgets(through, how(true)),
            "1,1 2,2");
        const spent = Held.forgets(nothing, left, before, nothing.steps);
        check("each read as it was before it was made again and as it is now",
            [spent.left[0], spent.left[1], spent.landed[0], spent.landed[1]]
                .map(one => one[one.length - 1].row.says).join(" | "),
            [before[1], nothing.steps[1], before[0], nothing.steps[0]]
                .map(step => step.row.says).join(" | "));

        // What the launcher's remaking answers its caller.
        check("every step made again is made",
            Held.answer({ steps, leave: -1, all: true }), "made");
        check("a step kept as it was is kept",
            Held.answer({ steps, leave: -1, all: false }), "kept");
        check("a step left is left", Held.answer(nothing), "left");
        check("as is one left past the outermost's search", Held.answer(through), "left");

        afterLeaving(check);
        thrownAs(check);
        answeredAs(check);
    }

    // What the launcher's showing call answers, each told to `check`: the
    // state as it's read, or JSON naming what threw, as the desktop tests
    // read it (tests/desktop/heard.py's shown), said for the journal too.
    function answeredAs(check) {
        const said = [];
        const say = threw => said.push(threw);
        check("a state read is answered as it came, nothing said",
            JSON.stringify([Held.answered(() => '{"open":true}', say), said]),
            JSON.stringify(['{"open":true}', []]));
        const broke = Object.assign(Object.create(TypeError.prototype), {
            message: "joel's secret", stack: "showing@file:///Launcher.qml:10021\nf@x:1"
        });
        const threw = "TypeError: thrown at showing@file:///Launcher.qml:10021";
        check("a read that threw answers what threw as JSON, said once, never its message",
            JSON.stringify([Held.answered(() => { throw broke; }, say), said]),
            JSON.stringify([JSON.stringify({ threw }), [threw]]));
        check("as does a thrown string",
            Held.answered(() => { throw "joel's secret"; }, () => {}),
            JSON.stringify({ threw: "Error: string thrown" }));
    }

    // A throw caught as a source refreshes, as the journal says it, each
    // told to `check`: never its message, and always with a kind ending
    // "Error:", as the VM tests' error nets look for.
    function thrownAs(check) {
        const shaped = (kind, stack) => Object.assign(Object.create(kind.prototype), {
            message: "joel's secret", stack
        });
        check("an Error says its kind and frame, never its message",
            Held.thrown(shaped(TypeError, "refreshNow@file:///Launcher.qml:5991\nwalk@x:1")),
            "TypeError: thrown at refreshNow@file:///Launcher.qml:5991");
        check("a frame with no function still says where",
            Held.thrown(shaped(Error, "@file:///Launcher.qml:12:4")),
            "Error: thrown at @file:///Launcher.qml:12:4");
        check("a first line shaped as another engine's message is left out",
            Held.thrown(shaped(Error, "Error: could not reach joel@box:22\nf@x:1")),
            "Error: thrown");
        check("as is a stack with none", Held.thrown(shaped(RangeError, undefined)),
            "RangeError: thrown");
        const named = shaped(Error, "f@x:1");
        named.name = "joel's secret";
        check("an Error named otherwise is said as Error", Held.thrown(named),
            "Error: thrown at f@x:1");
        check("a thrown string says its type, never itself", Held.thrown("joel's secret"),
            "Error: string thrown");
        check("as does a thrown object", Held.thrown({ message: "joel's secret" }),
            "Error: object thrown");
        check("and nothing thrown", Held.thrown(undefined), "Error: undefined thrown");
    }

    // What Ctrl+O, Ctrl+I, Ctrl+T and Tab keep once a step is left by
    // itself, each told to `check`. Places read as their steps joined by
    // "/", as the launcher's placeOf joins them.
    function afterLeaving(check) {
        // Where Joel stepped in from is now where he is, so the way back's
        // last places there go, as the place used last does there: each
        // would go nowhere, and show it as it was. Every place through the
        // step left goes too, wherever it is, since each would show that
        // spent step's rows as live. Where he is, and the step left, read
        // as they did before the leave and as they do now, since a row is
        // remade as offered now, and may be renamed so.
        // Home, which holds no steps, reads as "" and is drawn as "~".
        const steps = place => place === "" ? [] : place.split("/");
        const held = places => places.map(place => ({ held: steps(place) }));
        const at = one => one.held.length === 0 ? "~" : one.held.join("/");
        const after = (went, landed, left) => {
            const now = Held.afterLeave({
                jumps: held(went.jumps ?? []),
                ahead: held(went.ahead ?? []),
                tags: held(went.tags ?? []),
                last: went.last === null ? null : held([went.last ?? "elsewhere"])[0]
            }, { landed: landed.map(steps), left: left.map(steps) }, one => one.join("/"));
            return [now.jumps, now.ahead, now.tags].map(list => list.map(at).join(" "))
                .join(" | ") + `; ${now.last === null ? "none" : at(now.last)}`;
        };
        check("a way back and a place used last where a leave landed go",
            after({ jumps: ["q", "agents", "agents"], last: "agents" }, ["agents"],
                ["agents/s1"]), "q |  | ; none");
        check("only the last of the way back",
            after({ jumps: ["agents", "q", "agents"] }, ["agents"], ["agents/s1"]),
            "agents q |  | ; elsewhere");
        check("where it landed as it read before it was remade",
            after({ jumps: ["q", "Agents"] }, ["Agents", "Agents Now"], ["Agents/s1"]),
            "q |  | ; elsewhere");
        check("and as it reads now",
            after({ jumps: ["q", "Agents Now"] }, ["Agents", "Agents Now"],
                ["Agents/s1"]), "q |  | ; elsewhere");
        check("every place through the step left goes, the deepest last included",
            after({ jumps: ["q", "p", "p/s", "p/s/t"], last: "p/s/t" }, ["p"], ["p/s"]),
            "q |  | ; none");
        check("wherever it is on the way back",
            after({ jumps: ["p/s", "q", "p"] }, ["p"], ["p/s"]), "q |  | ; elsewhere");
        check("through the step left as it read now",
            after({ jumps: ["q", "p/S Now/t"] }, ["p"], ["p/s", "p/S Now"]),
            "q |  | ; elsewhere");
        check("a place through a step beside it stays",
            after({ jumps: ["q", "agents/s2"], last: "agents/s2" }, ["agents"],
                ["agents/s1"]), "q agents/s2 |  | ; agents/s2");
        check("Ctrl+I's places ahead and Ctrl+T's tags go alike",
            after({ ahead: ["q", "agents/s1/look", "agents"], tags: ["agents/s1", "agents"] },
                ["agents"], ["agents/s1"]), " | q | ; elsewhere");
        check("a spent first step forgets every place through it",
            after({ jumps: ["O/S2", "q", "O/S1", "O"], last: "O/S2" }, [""], ["O"]),
            "q |  | ; none");
        // Home is never a place a leave strands: going back to it brings
        // back what was typed there, which may be another search.
        check("a leave to home keeps home on the way back and as the place used last",
            after({ jumps: ["", "q", "agents", ""], last: "" }, [""], ["agents"]),
            "~ q ~ |  | ; ~");
        check("no place used last, as the launcher opens, is none still",
            after({ jumps: ["q", "agents"], last: null }, ["agents"], ["agents/s1"]),
            "q |  | ; none");
        check("a leave elsewhere leaves each as it was",
            after({ jumps: ["q", "agents"], last: "agents" }, ["quests"], ["quests/x"]),
            "q agents |  | ; agents");

        // Held.placeOf, as the launcher tells places apart for this: a row
        // by its key, as a row remade is matched, so a place recorded under
        // a name its row had before an earlier remaking is the same place.
        const at2 = (key, name) => ({ row: { key, name } });
        const agents = at2("agents", "Agents");
        const was = [[agents, at2("s1", "s1 working")], [agents, at2("s1", "s1 waiting")],
            [agents, at2("s2", "s2 working")]].map(one => ({ held: one }));
        const now = Held.afterLeave({ jumps: was, ahead: [], tags: [], last: was[1] },
            { landed: [[agents]], left: [[agents, at2("s1", "s1 idle")]] }, Held.placeOf);
        check("a place through the step left under an older name goes",
            `${now.jumps.map(one => one.held[1].row.name).join(" ")}; ${now.last}`,
            "s2 working; null");
        const same = (one, other) =>
            Held.placeOf([{ row: one }]) === Held.placeOf([{ row: other }]);
        check("a row with a key is told by it alone, whatever its name",
            [same({ key: "k", name: "a" }, { key: "k", name: "b" }),
                same({ key: "k", name: "a" }, { key: "j", name: "a" }),
                same({ key: "k", name: "a" }, { name: "k" })].join(" "), "true false false");
        // Every answer is the same "answer", so one with no key is told by
        // what it is and its name together.
        check("a row with no key is told by what it is and its name",
            [same({ name: "a" }, { name: "b" }), same({ name: "a" }, { key: "", name: "a" }),
                same({ is: "answer", name: "4" }, { is: "answer", name: "5" }),
                same({ is: "answer", name: "4" }, { is: "answer", name: "4" }),
                same({ is: "answer", name: "4" }, { name: "4" })].join(" "),
            "false true false true false");
        // An answer's name is the figure it says now, which every re-answer
        // changes (again()), so it's told by the words asked (its chip), as
        // again() keeps them: CPU's figure moving is no step to a new place,
        // and y's flash in it stays.
        const answer = (chip, name) => ({ is: "answer", chip, name });
        check("an answer is told by the words asked, whatever it says now",
            [same(answer("cpu", "4%"), answer("cpu", "5%")),
                same(answer("cpu", "4"), answer("mem", "4")),
                same(answer("cpu", "4"), { chip: "cpu", name: "4" })].join(" "),
            "true false false");

        // Held.forgets into Held.afterLeave, over steps as the launcher
        // holds them: a place's own step, with no key, renamed as it was
        // made again, so each history's places are found under each name.
        const step = (key, name) => ({ row: { key, name }, rows: [] });
        const old = [step("", "Agents"), step("s1", "s1 working")];
        const anew = [step("", "Agents Now"), step("s1", "s1 idle")];
        const made = { steps: anew, leave: 1, all: false };
        const forgetting = Held.forgets(made, Held.leaveTo(made, { outermost: false }), old,
            anew);
        const places = [[old[0], old[1]], [anew[0], anew[1]], [old[0], step("s2", "s2")],
            [anew[0]]].map(one => ({ held: one }));
        const across = Held.afterLeave(
            { jumps: places, ahead: [], tags: [], last: { held: [old[0]] } },
            forgetting, Held.placeOf);
        check("each history forgets the step left and where it landed, named either way",
            `${across.jumps.map(one => one.held.map(at => at.row.name).join("/")).join(" ")}; `
                + `${across.last}`, "Agents/s2; null");
    }
}
