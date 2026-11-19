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

        for (const failure of failures) {
            console.warn(`FAIL ${failure}`);
        }
        Qt.exit(failures.length === 0 ? 0 : 1);
    }

    // Where a step that offers nothing now leaves to, as `nothing` and
    // `below` made Agents' steps again, from `steps` and a step deeper,
    // each told to `check`.
    function leaving(check, steps, nothing, below) {
        const how = outermost => ({ outermost });
        const said = to => `${to.keep}`;

        check("nothing to leave keeps every step",
            said(Held.leaveTo({ steps: steps, leave: -1, all: true }, how(false), ways)), "-1");
        check("a step offering nothing is left, back to the step above it",
            said(Held.leaveTo(nothing, how(false), ways)), "1");
        check("and a step below one that does, back to it",
            said(Held.leaveTo(below, how(false), ways)), "2");
        // A dialog's own collection, the outermost, is never left, for
        // the home it never showed; it stays, its rows each gone, rather
        // than showing what it offered before. Its row stays as it was
        // entered, as the step a source's change leaves alone does.
        const spentTop = { steps: steps, leave: 0, all: false };
        keptWith = [];
        const top = Held.leaveTo(spentTop, how(true), ways);
        check("the outermost step is kept, the steps below it left", said(top), "1");
        check("its rows each gone", keptWith.join(" "), "agents:0");
        check("as it hands them back", read(top.steps.slice(0, 1)), "agents=Agents[]");
        check("the steps below it as they were", read(top.steps.slice(1)),
            "s1=working[stop look]");
        check("and what it was given is as it was", read(steps),
            "agents=Agents[s1] s1=working[stop look]");
        keptWith = [];
        const alone = Held.leaveTo({ steps: [steps[0]], leave: 0, all: false }, how(true), ways);
        check("the outermost alone is kept with its rows gone, nothing left",
            `${said(alone)} ${keptWith.join(" ")} ${read(alone.steps)}`,
            "-1 agents:0 agents=Agents[]");
        check("any other leaves to home", said(Held.leaveTo(spentTop, how(false), ways)), "0");
        check("steps it doesn't leave are as made",
            read(Held.leaveTo(below, how(false), ways).steps.slice(0, 2)),
            read(below.steps.slice(0, 2)));
        // A thing landed in from a search, `through`, is left back to the
        // search, past the collection it was found in, as Joel's - leaves it.
        const through = { steps: [steps[0], Object.assign({}, steps[1], { through: true })],
            leave: 1, all: false };
        check("a spent thing landed in from a search leaves past its collection",
            said(Held.leaveTo(through, how(false), ways)), "0");
        check("but never past the outermost", said(Held.leaveTo(through, how(true), ways)), "1");

        // What the launcher's remaking answers its caller.
        const answer = made => Held.answer(made, Held.leaveTo(made, how(false), ways));
        check("every step made again is made", answer({ steps, leave: -1, all: true }), "made");
        check("a step kept as it was is kept", answer({ steps, leave: -1, all: false }), "kept");
        check("a step left is left", answer(nothing), "left");
        check("the outermost alone, kept with its rows gone, is made",
            Held.answer({ steps: [steps[0]], leave: 0, all: false }, alone), "made");
        check("the outermost kept, with steps below it left, is left",
            Held.answer(spentTop, top), "left");

        // A step left by itself goes on no way back, and the way back's
        // last places, where Joel stepped in from, are now where he is, so
        // they go too, as the place used last does: Ctrl+O or Tab would go
        // nowhere, and show it as it was. Where he is reads as it did
        // before the leave and as it does now, since a row is remade as
        // offered now, and may be renamed so.
        const at = one => one.held.join("/");
        const jumped = places => places.map(held => ({ held: [held] }));
        const after = (jumps, last, landed) => {
            const now = Held.afterLeave({ jumps: jumped(jumps), last: jumped([last])[0] },
                landed, at);
            return `${now.jumps.map(at).join(" ")}; ${now.last === null ? "none" : at(now.last)}`;
        };
        check("a way back and a place used last where a leave landed go",
            after(["home", "agents", "agents"], "agents", ["agents"]), "home; none");
        check("only the last of the way back",
            after(["agents", "home", "agents"], "agents", ["agents"]), "agents home; none");
        check("where it landed as it read before it was remade",
            after(["home", "Agents"], "Agents", ["Agents", "Agents Now"]), "home; none");
        check("one to elsewhere stays",
            after(["home", "agents"], "agents", ["quests"]), "home agents; agents");
    }
}
