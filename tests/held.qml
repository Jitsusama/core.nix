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
        const ownGathered = Held.own(gathered, 1, [offeredNow], ways);
        const kept = ownGathered.steps[1].row;
        const keys = rows => rows.map(one => `${one.key}=${one.says}`).join(" ");
        check("a row found by a search keeps what the launcher gave it",
            `${kept.side} ${keys(kept.verbs())} ${kept.says}`, "in Agents look=Look idle");
        check("but not what the source offered then and doesn't now", kept.targets, undefined);
        check("asked what it offers at the step it is", askedAt.join(" "), "1");
        // `given` is a row's way of asking for words (Launcher.qml's
        // asking), so what the launcher gave is never said in it.
        check("nor taken for a row asking for words", typeof kept.given, "undefined");
        // Made again a second time, it still knows what the launcher gave.
        askedAt = [];
        const keptAgain = Held.own(ownGathered.steps, 1, [offeredNow], ways).steps[1].row;
        check("made again twice, it still keeps what the launcher gave it",
            `${keptAgain.side} ${keys(keptAgain.verbs())}`, "in Agents look=Look");
        // What the search found is each as the source offers it now, so an
        // ask hidden as a share begins is listed hidden, never as it was
        // found; one the source no longer offers is kept as found, as
        // People's, found beyond what it lists, are.
        const matched = () => [row("k1", "Kiln"), row("k2", "Kettle")];
        const searched = [steps[0], Object.assign({}, steps[1], {
            row: Object.assign({}, working, { foundAs: { from: "Asks", verbs: matched,
                landsIn: working } })
        })];
        const hiddenNow = [row("s1", "hidden"), row("k1", "hidden")];
        const ownSearched = Held.own(searched, 1, hiddenNow, ways).steps[1];
        check("what a search found is listed as the source offers it now",
            `${keys(ownSearched.row.verbs())} ${keys(ownSearched.rows)}`,
            "k1=hidden k2=Kettle k1=hidden k2=Kettle");
        check("and where it lands is the row as it is now", ownSearched.row.landsIn?.says,
            "hidden");
        const searchedAgain = Held.own([steps[0], ownSearched], 1, [row("s1", "hidden"),
            row("k1", "shown")], ways).steps[1].row;
        check("made again twice, it lists them as they are then", keys(searchedAgain.verbs()),
            "k1=shown k2=Kettle");
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

        for (const failure of failures) {
            console.warn(`FAIL ${failure}`);
        }
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
