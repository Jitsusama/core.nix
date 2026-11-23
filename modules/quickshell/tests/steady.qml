// How a list the launcher shows takes a change under it, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `the-launcher-holds-still-as-it-refreshes` check copies
// modules/quickshell/launcher/Steady.js beside this file and runs
// `qml -platform offscreen steady.qml`. Each case is a promise Joel leans on
// while a dozen agents change around him: rows keep the order they're shown
// in, and the selection stays on its row.
import QtQuick
import "Steady.js" as Steady

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

    function rows(said) {
        return said.split(" ").filter(one => one !== "").map(one => {
            const gone = one.endsWith("-");
            const [key, side] = (gone ? one.slice(0, -1) : one).split(":");
            return gone ? { key: key, side: side ?? "", gone: true } : { key: key, side: side ?? "" };
        });
    }

    // Each row as it reads, a row gone marked by a dash after it.
    function read(list) {
        return list.map(row => (row.side === "" ? row.key : `${row.key}:${row.side}`)
            + (row.gone ? "-" : "")).join(" ");
    }

    function run() {
        const failures = [];
        const id = row => row.key;
        // [what, the rows shown, the rows now, how they're shown after]
        const orders = [
            ["nothing changed is as it was", "a b c", "a b c", "a b c"],
            ["a row that would rank first stays where it's shown", "a b c", "c a b", "a b c"],
            ["each row says what it says now", "a:working b c", "c a:waiting b",
                "a:waiting b c"],
            ["a new row comes at the end", "a b", "c a b", "a b c"],
            ["new rows come in their own order", "a", "d c a b", "a d c b"],
            ["a row gone stays where it was, gone", "a b c", "c a", "a b- c"],
            ["and says what it said", "a b:working", "a", "a b:working-"],
            ["everything gone stays, gone", "a b", "", "a- b-"],
            ["a row gone is still gone", "a b-", "a", "a b-"],
            ["a row gone and back is there again", "a b-", "b a", "a b"],
            ["nothing shown takes the rows as they are", "", "b a", "b a"],
            ["rows alike keep their places", "x:1 x:2 y", "y x:3 x:4", "x:3 x:4 y"]
        ];
        for (const [what, shown, now, want] of orders) {
            const got = read(Steady.kept(rows(shown), rows(now), id));
            if (got !== want)
                failures.push(`${what}: "${got}", not "${want}"`);
        }

        // [what, the rows shown, the rows now, how they're shown while
        // held, how many wait]: in visual, operator-pending or on the
        // command line the rows hold still and a new one waits behind
        // "2 new" until it ends (VKCF3Z's spec), each shown row saying
        // what it says now and one gone staying, as ever.
        const holding = [
            ["nothing new waits for nothing", "a b", "b a", "a b", 0],
            ["a new row waits", "a b", "c a b", "a b", 1],
            ["and two", "a", "d c a", "a", 2],
            ["a row gone stays, gone, a new one waiting", "a b", "c a", "a b-", 1],
            ["each says what it says now", "a:working b", "b a:waiting", "a:waiting b", 0]
        ];
        for (const [what, shown, now, want, waiting] of holding) {
            const held = Steady.held(rows(shown), rows(now), id);
            const got = [read(held.rows), held.waiting];
            if (JSON.stringify(got) !== JSON.stringify([want, waiting]))
                failures.push(`${what}: ${JSON.stringify(got)}, not ${JSON.stringify([want, waiting])}`);
        }

        // [what, the rows shown, the rows now, what's left once a key makes
        // the list afresh]: Write in Slack's dimmed row, there until Slack
        // answers, stands where it is through the search a step-in makes.
        const forgets = [
            ["a row that left is forgotten", "a b c", "c a", "a c"],
            ["a row its source dims stays where it stands", "a b c", "a b- c", "a b- c"],
            ["and is forgotten once it leaves", "a b- c", "a c", "a c"],
            ["a row that left stays forgotten", "a b c", "a", "a"]
        ];
        for (const [what, shown, now, want] of forgets) {
            const got = read(Steady.forgotten(Steady.kept(rows(shown), rows(now), id)));
            if (got !== want)
                failures.push(`${what}: "${got}", not "${want}"`);
        }

        // [what, the rows shown, the rows now, where the selection was, what's
        // shown after and where the selection goes]: a share's start takes
        // what left off the screen at once, since its copy names what the
        // share hides (Launcher.qml's untold), and the cursor keeps to its
        // row as a refresh keeps it.
        const forgetsAt = [
            ["a row that left goes, the cursor kept on its row", "a b c", "a c", 2, "a c", 1],
            ["on the row that left, it stays where it stood", "a b c", "a c", 1, "a c", 1],
            ["a row its source dims stays", "a b c", "a b- c", 2, "a b- c", 2],
            ["nothing selected stays so", "a b c", "a c", -1, "a c", -1],
            ["everything gone, it goes to the top", "a b", "", 1, "", 0]
        ];
        for (const [what, shown, now, at, want, wantAt] of forgetsAt) {
            const left = Steady.forgottenAt(Steady.kept(rows(shown), rows(now), id), at, id);
            const got = JSON.stringify([read(left.rows), left.at]);
            const wanted = JSON.stringify([want, wantAt]);
            if (got !== wanted)
                failures.push(`${what}: ${got}, not ${wanted}`);
        }

        // A share's start forgets every way back to what was shown before
        // it, since a step there may be an ask's, whose name the crumb
        // draws: Ctrl+O's and Ctrl+I's places, Ctrl+T's, and Tab's last,
        // as well as the rows that left (Launcher.qml's untold). An
        // ordinary place goes with an ask's, so a filter keeping the
        // places it thinks safe fails here: none is told apart.
        const step = { held: [{ row: { key: "ask", name: "the bank's password" } }],
            here: { typed: "bank" } };
        const plain = { held: [{ row: { key: "music", name: "Music" } }], here: { typed: "" } };
        const began = Steady.shareStarts({ rows: Steady.kept(rows("a b c"), rows("a c"), id),
            at: 2, back: [plain, step], ahead: [step, plain], tags: [plain, step], last: plain },
            id);
        const forgot = JSON.stringify([read(began.rows), began.at, began.back, began.ahead,
            began.tags, began.last]);
        const forgetting = JSON.stringify(["a c", 1, [], [], [], null]);
        if (forgot !== forgetting)
            failures.push(`a share's start forgets the ways back: ${forgot}, not ${forgetting}`);

        // While a source hides, no way back goes to a place holding a row
        // it made showing, as a step or among a step's rows, which the
        // place draws as they were: one recorded after the share began but
        // before the source's rows were remade holds the name from before
        // it, and a source that never says a share began (Copies alone)
        // forgets nothing (Launcher.qml's jump, toLastPlace and tagBack).
        // Each place is held as the launcher holds it: a collection step is
        // `from` its source, and a source's own row says it was made showing
        // (`shownBy`) wherever it's held, as For You holds an ask under Now.
        // A place whose rows were made hidden, a showing source's, and one
        // of no source stay, in order.
        const hiding = { hiding: true };
        const showing = { hiding: false };
        const asks = { row: { key: "collection:Asks", name: "Asks", from: hiding }, rows: [] };
        const copies = { row: { key: "collection:Copies", name: "Copies", from: hiding } };
        const asked = { row: { key: "ask", name: "the bank's password", shownBy: hiding },
            rows: [] };
        const place = (name, held) => ({ name, held, here: { typed: "" } });
        const inAsks = place("inAsks", [asks, asked]);
        const forYou = place("forYou", [{ row: { key: "collection:Now", from: {} }, rows: [] },
            asked]);
        const listed = place("listed", [Object.assign({ rows: [{ key: "copy", name: "hunter2",
            shownBy: hiding }] }, copies)]);
        const remade = place("remade", [Object.assign({ rows: [{ key: "copy",
            name: "A copied text" }] }, copies)]);
        const music = place("music", [{ row: { key: "song", shownBy: showing }, rows: [] }]);
        const home = place("home", [{ row: { key: "home" } }]);
        const gone = Steady.hiddenGone({ back: [music, inAsks, forYou, remade, home],
            ahead: [listed, music], tags: [home, forYou, remade], last: inAsks });
        const names = list => list.map(one => one.name).join(" ");
        const hidden = JSON.stringify([names(gone.back), names(gone.ahead), names(gone.tags),
            gone.last]);
        const hiddenWant = JSON.stringify(["music remade home", "music", "home remade", null]);
        if (hidden !== hiddenWant)
            failures.push(`a place showing what hides now can't be gone back to: ${hidden}, `
                + `not ${hiddenWant}`);
        const kept = Steady.hiddenGone({ back: [music], ahead: [], tags: [], last: remade });
        if (kept.last !== remade || kept.back[0] !== music)
            failures.push("a place showing nothing that hides now is gone back to as it was");
        // As each opening leaves it, with no last place yet.
        if (Steady.hiddenGone({ back: [], ahead: [], tags: [], last: null }).last !== null)
            failures.push("with no last place, none is made");

        // A way back a share has let go says so at its own key's first
        // press, where it would otherwise say nothing or E73, and its next
        // press says what Vim says. untold empties all four at a share's
        // start, and one press's hiddenGone can empty several, so each way
        // keeps its own mark until its key is pressed; one that never held
        // a place, still holds one or holds one again isn't let go.
        const counts = (back, ahead, tags, last) => ({ back, ahead, tags, last });
        const marked = marks => Object.keys(marks).filter(way => marks[way]).join(" ");
        const none = Steady.noneLetGo();
        const started = Steady.waysLetGo(none, counts(2, 0, 1, 1), counts(0, 0, 0, 0));
        if (marked(started) !== "back tags last")
            failures.push(`a share's start lets go of each way that held a place: `
                + `${marked(started)}`);
        const pressed = Steady.letGoTold(started, "back");
        const again = Steady.letGoTold(pressed.marks, "back");
        const tagged = Steady.letGoTold(Steady.waysLetGo(pressed.marks, counts(0, 0, 0, 0),
            counts(0, 0, 0, 0)), "tags");
        const told = [pressed.say, again.say, tagged.say, marked(tagged.marks)];
        const toldWant = JSON.stringify([true, false, true, "last"]);
        if (JSON.stringify(told) !== toldWant)
            failures.push(`each way says it was let go once, at its own key: `
                + `${JSON.stringify(told)}, not ${toldWant}`);
        const stillHeld = Steady.waysLetGo(none, counts(2, 1, 0, 1), counts(1, 0, 0, 1));
        if (marked(stillHeld) !== "ahead")
            failures.push(`only a way left empty is let go: ${marked(stillHeld)}`);
        const refilled = Steady.waysLetGo(started, counts(0, 0, 0, 0), counts(1, 0, 0, 0));
        if (marked(refilled) !== "tags last")
            failures.push(`a way holding a place again isn't let go: ${marked(refilled)}`);
        if (Steady.letGoTold(none, "ahead").say)
            failures.push("a way that never held a place says what Vim says");
        // A way written by anything but a share's letting go (a step, a
        // jump, Ctrl+], a step leaving by itself) is no longer the share's
        // to explain, though no key looked while it held a place; one not
        // let go is left as it was, so nothing that reads the marks moves.
        const written = Steady.wayWritten(started, "tags");
        if (marked(written) !== "back last")
            failures.push(`a way written since isn't the share's to explain: ${marked(written)}`);
        if (Steady.wayWritten(written, "ahead") !== written)
            failures.push("a way not let go leaves the marks as they were");

        // A row that arrives while the list is shown comes at the end, out
        // of the place a tree would give it, so it reads as it would ranked
        // flat, as a session a step in under its orchestrator does once
        // typed: at no depth, saying whom it's under (Agents.qml). One kept
        // in place, and every row of a list shown afresh, keep their own.
        const nested = key => ({ key: key, side: "", depth: 1,
            ranked: () => ({ key: key, side: "under", depth: 0 }) });
        const depths = list => list.map(row => `${row.key}${row.depth ?? 0}:${row.side}`)
            .join(" ");
        const flats = [
            ["a row arriving late reads flat", [{ key: "a", side: "" }],
                [{ key: "a", side: "" }, nested("b")], "a0: b0:under"],
            ["a row kept in place keeps its depth", [nested("b")], [nested("b")], "b1:"],
            ["a list shown afresh keeps every row's depth", [], [nested("b")], "b1:"]
        ];
        for (const [what, shown, now, want] of flats) {
            const got = depths(Steady.kept(shown, now, id));
            if (got !== want)
                failures.push(`${what}: "${got}", not "${want}"`);
        }

        // [what, the rows shown, the rows now, how they're shown after]: a
        // row that arrives comes at the end, which reads as under the last
        // heading there, so where that's another cell's, its own is said
        // again over it, as it reads above, and nowhere else (Y2YBTS,
        // 2026-11-22 [228cdeb9] and [4c5e154a]). A heading is "#" and its
        // name, with "=" and the rows it holds, by key, where it names them
        // as For You's do, one said again "+" after it, a pressing row "!",
        // and one gone "-".
        const underHeadings = said => said.split(" ").filter(one => one !== "").map(one => {
            const gone = one.endsWith("-");
            const bare = gone ? one.slice(0, -1) : one;
            const again = bare.endsWith("+");
            const named = again ? bare.slice(0, -1) : bare;
            const presses = named.endsWith("!");
            const [key, holding] = (presses ? named.slice(0, -1) : named).split("=");
            return Object.assign({ key: key, side: "" },
                key.startsWith("#") ? { heading: true } : {},
                holding === undefined ? {} : { holding: holding.split(",") },
                presses ? { pressing: true } : {},
                again ? { repeated: true } : {}, gone ? { gone: true, went: true } : {});
        });
        const readHeaded = list => list.map(row => row.key + (row.repeated ? "+" : "")
            + (row.gone ? "-" : "")).join(" ");
        const headings = [
            ["a row new to another cell's end gets its heading", "#A a1 #B b1", "#A a1 a2 #B b1",
                "#A a1 #B b1 #A+ a2"],
            ["a row new to its own cell's end gets none", "#A a1 #B b1", "#A a1 #B b1 b2",
                "#A a1 #B b1 b2"],
            ["a reopening has one heading for each cell", "", "#A a1 a2 #B b1", "#A a1 a2 #B b1"],
            ["rows new to one cell share its heading", "#A a1 #B b1", "#A a1 a2 a3 #B b1",
                "#A a1 #B b1 #A+ a2 a3"],
            ["rows new to two cells get one each", "#A a1 #B b1", "#A a1 a2 #B b1 b2",
                "#A a1 #B b1 #A+ a2 #B+ b2"],
            ["a new cell's heading comes in over its row", "#A a1", "#A a1 #C c1",
                "#A a1 #C+ c1"],
            ["a heading said again stays where it was said", "#A a1 #B b1 #A+ a2",
                "#A a1 a2 #B b1", "#A a1 #B b1 #A+ a2"],
            ["and needs no other for a row new under it", "#A a1 #B b1 #A+ a2",
                "#A a1 a2 a3 #B b1", "#A a1 #B b1 #A+ a2 a3"],
            ["and goes dim with its cell", "#A a1 #B b1 #A+ a2", "#B b1",
                "#A- a1- #B b1 #A+- a2-"],
            ["and goes dim once no row under it stands", "#A a1 #B b1 #A+ a2", "#A a1 #B b1",
                "#A a1 #B b1 #A+- a2-"],
            ["and stays dim while none comes back", "#A a1 #B b1 #A+- a2-", "#A a1 #B b1",
                "#A a1 #B b1 #A+- a2-"],
            ["and lights again with a row come back under it", "#A a1 #B b1 #A+- a2-",
                "#A a1 a2 #B b1", "#A a1 #B b1 #A+ a2"],
            ["rows handed back with their headings said again keep one of each",
                "#A a1 #B b1 #A+ a2", "#A a1 #B b1 #A+ a2", "#A a1 #B b1 #A+ a2"],
            ["a row whose cell changes stays where it was shown", "#A a1 a2 #B b1",
                "#A a1 #B b1 a2", "#A a1 a2 #B b1"],
            ["his own rows stay unheaded as a cell's come", "a1 a2", "#Y=a1,a2 a1 a2 #C=c1 c1",
                "a1 a2 #C+ c1"],
            ["a heading new to the list comes only with a new row", "a1 c1",
                "#Y=a1 a1 #C=c1 c1", "a1 c1"],
            ["kept, a row of no heading comes in, for the list shown to withhold",
                "#A=a1 a1", "#A=a1 a1 look", "#A a1 look"],
            ["a fold new to Repositories gets its heading past Projects", "#R r1 #P p1",
                "#R r1 r:more #P p1", "#R r1 #P p1 #R+ r:more"],
            ["with no heading, a new row comes in", "a1", "a1 x", "a1 x"],
            ["reopened, every row shows", "", "#A=a1 a1 look", "#A a1 look"]
        ];
        for (const [what, shown, now, want] of headings) {
            const got = readHeaded(Steady.kept(underHeadings(shown), underHeadings(now), id));
            if (got !== want)
                failures.push(`${what}: "${got}", not "${want}"`);
        }
        const saidAgain = Steady.kept(underHeadings("#A a1 #B b1 #A+ a2"),
            [{ key: "#A", side: "two", heading: true }].concat(underHeadings("a1 a2 #B b1")), id);
        if (saidAgain[4].side !== "two" || saidAgain[4].repeated !== true)
            failures.push(`a heading said again reads as it reads now: "${saidAgain[4].side}"`);
        // Handed back to itself, as For You's search hands back the rows it
        // keeps, a list stays as it is, however often.
        const handed = Steady.kept(underHeadings("#A a1 #B b1"), underHeadings("#A a1 a2 #B b1"),
            id);
        const twice = Steady.kept(Steady.kept(handed, handed, id), handed, id);
        if (readHeaded(twice) !== readHeaded(handed))
            failures.push(`a list handed back to itself: "${readHeaded(twice)}"`);
        // A row whose cell changes says what it says now where it stands.
        const moved = Steady.kept(underHeadings("#A a1 a2 #B b1"), underHeadings("#A a1 #B b1")
            .concat([{ key: "a2", side: "blocked by B" }]), id);
        if (moved[2].key !== "a2" || moved[2].side !== "blocked by B")
            failures.push(`a row whose cell changed: "${moved[2].key}:${moved[2].side}"`);
        // A key making the list afresh lets a heading go with its rows.
        const madeAfresh = readHeaded(Steady.forgotten(Steady.kept(
            underHeadings("#A a1 #B b1 #A+ a2"), underHeadings("#A a1 #B b1"), id)));
        if (madeAfresh !== "#A a1 #B b1")
            failures.push(`a heading whose rows went, made afresh: "${madeAfresh}"`);
        // [what, the rows shown, the rows now, how they're shown after, how
        // many wait]: as shown, a new row of no heading waits unseen, counted
        // in the foot, unless it's pressing.
        const shownAgain = [
            ["a row under no heading waits", "#A=a1 a1", "#A=a1 a1 look later", "#A a1", 2],
            ["and one above every heading", "#A=a1 a1", "x #A=a1 a1", "#A a1", 1],
            ["but a pressing row comes in", "#A=a1 a1", "#A=a1 a1 asked!", "#A a1 asked", 0],
            ["and one above every heading", "#A=a1 a1", "asked! #A=a1 a1", "#A a1 asked", 0],
            ["and a cell's row after it has its heading said", "#A=a1 a1",
                "asked! #A=a1,a2 a1 a2", "#A a1 asked #A+ a2", 0],
            ["a cell's new row still gets its heading past the others", "#A=a1 a1 #B=b1 b1",
                "#A=a1,a2 a1 a2 #B=b1 b1 look", "#A a1 #B b1 #A+ a2", 1],
            ["a pressing row keeps no dim heading said again lit", "#A a1 #B b1 #A+ a2",
                "asked! #A a1 #B b1", "#A a1 #B b1 #A+- a2- asked", 0],
            ["a new row past a dim cell gets its heading", "#A=a1 a1 #B=b1 b1",
                "#A=a1,a2 a1 a2", "#A a1 #B- b1- #A+ a2", 0],
            ["with headings shown and none now, a new row waits", "#A=a1 a1 #Y=y1 y1",
                "y1 look", "#A- a1- #Y- y1", 1],
            ["with no heading, a new row comes in", "a1", "a1 x", "a1 x", 0],
            ["a row of no cell waits behind a cell's first heading come with it", "a1 a2",
                "#Y=a1,a2 a1 a2 #C=c1 c1 look", "a1 a2 #C+ c1", 1],
            ["and behind his own rows' others", "a1 look1", "#Y=a1 a1 #C=c1 c1 look1 look2",
                "a1 look1 #C+ c1", 1],
            ["a pressing row shown before ends the heading above it", "#A=a1 a1 #B=b1 b1 asked!",
                "asked! #A=a1 a1 #B=b1,b2 b1 b2", "#A a1 #B b1 asked #B+ b2", 0],
            ["a row that left before it was shown is never counted", "#A=a1 a1",
                "#A=a1 a1 look-", "#A a1", 0]
        ];
        for (const [what, shown, now, want, waiting] of shownAgain) {
            const got = Steady.refreshed(underHeadings(shown), underHeadings(now), id);
            if (readHeaded(got.rows) !== want || got.waiting !== waiting)
                failures.push(`${what}: "${readHeaded(got.rows)}" with ${got.waiting} waiting, `
                    + `not "${want}" with ${waiting}`);
        }
        // A row's pressing may be what it says while asked, as Desktop's and
        // Lately's rows say it: one that says no waits, and comes in once it
        // says yes.
        const critical = { now: false };
        const asking = underHeadings("#A=a1 a1").concat([{ key: "notices", side: "",
            pressing: () => critical.now }]);
        for (const [what, now, want, waiting] of [
            ["a row whose pressing says no waits", false, "#A a1", 1],
            ["and comes in once it says yes", true, "#A a1 notices", 0]
        ]) {
            critical.now = now;
            const got = Steady.refreshed(underHeadings("#A=a1 a1"), asking, id);
            if (readHeaded(got.rows) !== want || got.waiting !== waiting)
                failures.push(`${what}: "${readHeaded(got.rows)}" with ${got.waiting} waiting, `
                    + `not "${want}" with ${waiting}`);
        }
        // A row of no cell that comes while a held step's rows are made again,
        // and leaves before it's shown, stops being counted once it's gone.
        const waitedFrom = underHeadings("#A=a1 a1");
        const cameStep = Steady.kept(waitedFrom, underHeadings("#A=a1 a1 look"), id);
        const came = Steady.refreshed(waitedFrom, cameStep, id);
        const left = Steady.refreshed(came.rows,
            Steady.kept(cameStep, underHeadings("#A=a1 a1"), id), id);
        if (came.waiting !== 1 || readHeaded(left.rows) !== "#A a1" || left.waiting !== 0)
            failures.push(`a row that came and left while waiting: ${came.waiting}, then `
                + `"${readHeaded(left.rows)}" with ${left.waiting} waiting`);
        // A held step's rows are made again by kept (Held.js's keep), and the
        // list shown is laid from them, so each row is laid twice, and again
        // at each refresh after; a row of a cell is still its cell's, and one
        // of none still counted.
        for (const [what, shown, now, want, waiting] of [
            ["a cell's new row, laid twice", "#A=a1 a1 #B=b1 b1", "#A=a1,a2 a1 a2 #B=b1 b1",
                "#A a1 #B b1 #A+ a2", 0],
            ["and a row of no cell, counted", "#A=a1 a1 #B=b1 b1", "#A=a1 a1 #B=b1 b1 look",
                "#A a1 #B b1", 1],
            ["and a pressing row, come in", "#A=a1 a1 #B=b1 b1", "asked! #A=a1 a1 #B=b1 b1",
                "#A a1 #B b1 asked", 0],
            ["and Networks' headings, which name no rows", "#R r1 #P p1", "#R r1 r2 #P p1",
                "#R r1 #P p1 #R+ r2", 0]
        ]) {
            const stepRows = Steady.kept(underHeadings(shown), underHeadings(now), id);
            const first = Steady.refreshed(underHeadings(shown), stepRows, id);
            const second = Steady.refreshed(first.rows,
                Steady.kept(stepRows, underHeadings(now), id), id);
            for (const [when, got] of [["laid", first], ["refreshed", second]]) {
                if (readHeaded(got.rows) !== want || got.waiting !== waiting)
                    failures.push(`${what}, ${when}: "${readHeaded(got.rows)}" with `
                        + `${got.waiting} waiting, not "${want}" with ${waiting}`);
            }
        }
        // A key lists a held step's rows as they stand (Steady.seen), so a
        // row of no cell kept at their end still waits there, counted, made
        // again however often, rather than reading as the last cell's; one
        // of a cell after it still has its heading said, and it comes in
        // once it's pressing.
        // [what, the rows shown, each making of the step's rows after, how a
        // key lists them, how many wait]
        for (const [what, shown, makings, want, waiting] of [
            ["a row of no cell kept at the end waits", "#A=a1 a1 #B=b1 b1",
                ["#A=a1 a1 #B=b1 b1 look"], "#A a1 #B b1", 1],
            ["and still waits made again", "#A=a1 a1 #B=b1 b1",
                ["#A=a1 a1 #B=b1 b1 look", "#A=a1 a1 #B=b1 b1 look"], "#A a1 #B b1", 1],
            ["with a cell's row come after it", "#A=a1 a1 #B=b1 b1",
                ["#A=a1 a1 #B=b1 b1 look", "#A=a1,a2 a1 a2 #B=b1 b1 look"],
                "#A a1 #B b1 #A+ a2", 1],
            ["and comes in once it's pressing", "#A=a1 a1 #B=b1 b1",
                ["#A=a1 a1 #B=b1 b1 look", "#A=a1 a1 #B=b1 b1 look!"], "#A a1 #B b1 look", 0],
            ["and leaves with nothing to dim", "#A=a1 a1 #B=b1 b1",
                ["#A=a1 a1 #B=b1 b1 look", "#A=a1 a1 #B=b1 b1"], "#A a1 #B b1", 0]
        ]) {
            const stepRows = makings.reduce((rows, now) => Steady.kept(rows, underHeadings(now),
                id), underHeadings(shown));
            const listed = Steady.seen(Steady.forgotten(stepRows));
            const shownNow = Steady.refreshed(underHeadings(shown), stepRows, id);
            if (readHeaded(listed.rows) !== want || listed.waiting !== waiting)
                failures.push(`${what}, listed by a key: "${readHeaded(listed.rows)}" with `
                    + `${listed.waiting} waiting, not "${want}" with ${waiting}`);
            if (readHeaded(shownNow.rows) !== want || shownNow.waiting !== waiting)
                failures.push(`${what}, refreshed: "${readHeaded(shownNow.rows)}" with `
                    + `${shownNow.waiting} waiting, not "${want}" with ${waiting}`);
        }
        // While the rows hold still, a heading said again or new is never
        // counted among the rows that wait.
        for (const [what, shown, now, waiting] of [
            ["a row new to another cell waits as one", "#A a1 #B b1", "#A a1 a2 #B b1", 1],
            ["and a new cell's row as one", "#A a1", "#A a1 #C c1", 1],
            ["with a heading said again shown, a row under it", "#A a1 #B b1 #A+ a2",
                "#A a1 a2 a3 #B b1", 1],
            ["and another cell's", "#A a1 #B b1 #A+ a2", "#A a1 a2 #B b1 b2", 1],
            ["and a row under no heading", "#A=a1 a1", "#A=a1,a2 a1 a2 look", 2]
        ]) {
            const got = Steady.held(underHeadings(shown), underHeadings(now), id).waiting;
            if (got !== waiting)
                failures.push(`${what}: ${got} waiting, not ${waiting}`);
        }
        // Held, a heading said again goes dim by the rows held, not by one
        // that waits.
        const heldDim = Steady.held(underHeadings("#A a1 #B b1 #A+ a2"),
            underHeadings("#A a1 a3 #B b1"), id);
        if (readHeaded(heldDim.rows) !== "#A a1 #B b1 #A+- a2-" || heldDim.waiting !== 1)
            failures.push(`held, a heading over a row gone: "${readHeaded(heldDim.rows)}" with `
                + `${heldDim.waiting} waiting`);

        // [what, the rows shown, whether an answer gave way, those that may
        // stand]: an ending is made again, and an answer given only while
        // nothing else was goes once something comes, rather than standing
        // dimmed above it with the selection on it (80ECO2, 2026-11-12).
        const ask = { key: "", side: "", answered: true };
        const ending = { key: "e", side: "", endedBy: {} };
        const standings = [
            ["an ending never stands", [ask, ending], false, ""],
            ["an answer that holds stands", [ask, { key: "a", side: "" }], false, " a"],
            ["one that gave way goes", [ask, { key: "a", side: "" }], true, "a"]
        ];
        for (const [what, shown, gaveWay, want] of standings) {
            const got = read(Steady.standing(shown, gaveWay));
            if (got !== want)
                failures.push(`${what}: "${got}", not "${want}"`);
        }

        const after = rows("a b c d");
        // [what, the row it was on, where it was, the lowest it goes, where it goes]
        const selections = [
            ["it stays on its row", "c", 1, 0, 2],
            ["on the first row too", "a", 0, 0, 0],
            ["its row gone, it stays where it was", "z", 2, 0, 2],
            ["and no further than the last row", "z", 9, 0, 3],
            ["with none, it stays where it was", "", 1, 0, 1],
            ["above the rows, an answer's headline stays", "", -1, -1, -1],
            ["but never above the lowest", "", -1, 0, 0]
        ];
        for (const [what, was, at, lowest, want] of selections) {
            const got = Steady.selection(after, { row: was, at: at, lowest: lowest }, id);
            if (got !== want)
                failures.push(`${what}: ${got}, not ${want}`);
        }
        if (Steady.selection([], { row: "a", at: 0, lowest: 0 }, id) !== 0)
            failures.push("an empty list leaves the selection at the top");
        // Two rows alike in their id, as two windows titled alike or a key
        // listed twice: [what, the rows, how many alike stood above, where it goes]
        const alike = [
            ["on the second of two alike, it stays on the second", "s a s b", 1, 2],
            ["on the first, it stays on the first", "s a s b", 0, 0],
            ["a row coming above both, it stays on the second", "n s a s b", 1, 3],
            ["with one alike left, it goes onto that one", "a s b", 1, 1]
        ];
        // [what, the rows, where the selection is, how many alike stand above it]
        const above = [
            ["the second of two alike has one above", "s a s b", 2, 1],
            ["the first has none", "s a s b", 0, 0],
            ["a row with no twin has none", "s a s b", 1, 0],
            ["nothing selected has none", "s a s b", -1, 0],
            ["nor past the last row", "s a s b", 9, 0]
        ];
        for (const [what, now, at, want] of above) {
            const got = Steady.alikeAbove(rows(now), at, id);
            if (got !== want)
                failures.push(`${what}: ${got}, not ${want}`);
        }
        for (const [what, now, nth, want] of alike) {
            const got = Steady.selection(rows(now), { row: "s", at: 2, lowest: 0, nth: nth }, id);
            if (got !== want)
                failures.push(`${what}: ${got}, not ${want}`);
        }

        // [what, what came in turn, where each landed, whether it waits after]
        const wantsB = row => row.key === "b";
        const landings = [
            ["rows holding the row it wants land on it", ["a b c"], [1], false],
            ["rows without it wait for it", ["a c"], [-1], true],
            ["and land once it comes", ["a c", "a b c"], [-1, 1], false],
            ["landed, it never lands again", ["a b c", "b a c"], [1, -1], false],
            ["a key lets it go, so the row coming after never moves the cursor",
                ["a c", "key", "a b c"], [-1, -1, -1], false]
        ];
        for (const [what, came, want, waits] of landings) {
            let waiting = wantsB;
            const got = came.map(one => {
                const next = Steady.landing(waiting,
                    one === "key" ? { key: true } : { rows: rows(one) });
                waiting = next.waiting;
                return next.at;
            });
            if (JSON.stringify(got) !== JSON.stringify(want) || (waiting !== null) !== waits)
                failures.push(`${what}: landed ${JSON.stringify(got)}, waiting ${waiting !== null}`);
        }
        if (Steady.landing(null, { rows: rows("b") }).at !== -1)
            failures.push("with no landing asked, rows land nowhere");

        // A row in the list stands its band high, with the gap above one
        // that stands apart wherever in the list it stands, and a heading its
        // line, with the groups' rung above it but for the list's first. The
        // screen's room for the list is all but endless here, but for the
        // cases that give their own.
        const rowSizes = { row: 16, heading: 10, gap: 8, rung: 4, room: 1000 };
        const headed = one => one?.heading === true;

        // The card's list is as tall as the rows it shows first stand, and
        // the headings among them, `rows` rows of them, so its last row ends
        // at its foot: every gap among them counted, and none below them. It
        // never passes the screen's room, and stops at the last row that
        // fits in it, a heading never left alone at its foot. A list with
        // fewer rows than that is sized for the room. Each case says how
        // many of the list's first it shows.
        const sizings = [
            ["rows with no gap", [{}, {}, {}], 2, 2, 32],
            ["a gap among those shown", [{}, { gap: true }, {}], 2, 2, 40],
            ["a gap on the first row", [{ gap: true }, {}, {}], 2, 2, 40],
            ["a gap just past those shown", [{}, {}, { gap: true }], 2, 2, 32],
            ["a gap past those shown", [{}, {}, {}, { gap: true }], 2, 2, 32],
            ["fewer rows than room", [{ gap: true }], 2, 1, 40],
            ["no rows at all", [], 2, 0, 32],
            ["two rows standing apart", [{ gap: true }, {}, { gap: true }, {}], 3, 3, 64],
            ["a headed list", [{ heading: true }, {}, { heading: true }, {}, {}, {}], 3, 5, 72],
            ["a heading among a card's rows", [{}, { heading: true }, {}, {}], 2, 3, 46],
            ["rows past the screen's room", [{ gap: true }, { gap: true }, { gap: true }, {}],
                3, 2, 48, 56],
            ["a heading the room leaves at the foot", [{}, { heading: true }, {}, {}], 3, 1, 16,
                40],
            ["fewer rows than room, two apart", [{ gap: true }, { gap: true }], 3, 2, 64],
            ["rows that fill the room to the pixel", [{ gap: true }, { gap: true },
                { gap: true }, {}], 3, 3, 72, 72],
            ["a short list on a small screen", [{ gap: true }], 3, 1, 40, 40]
        ];
        for (const [what, shown, fit, shows, want, room] of sizings) {
            const sizes = Object.assign({}, rowSizes, room === undefined ? {} : { room: room });
            const sized = Steady.tallFor(shown, fit, sizes, headed);
            if (sized !== want)
                failures.push(`${what}: sized ${sized}, not ${want}`);
            // Where the list runs past the card, the rows it shows stand as
            // tall as the card, so the last ends at its foot.
            const stood = shown.slice(0, shows).reduce((sum, row, at) =>
                sum + Steady.tallOf(headed(row), row.gap, at, sizes), 0);
            if (shown.length > shows && stood !== sized)
                failures.push(`${what}: its rows stand ${stood}, the card ${sized}`);
        }

        // An open card keeps its height while the screen's room holds it,
        // however much the room grows, and is sized again by tallFor's rule
        // once the room drops below it, whole rows to the foot, home or none.
        // The heights kept, 40 and 32, are ones tallFor wouldn't give in
        // those rooms (it gives 32 and 48), so a refit that always sized
        // again would fail them.
        const refits = [
            ["a room that just holds the card", 40, [{}, {}, {}], 3, 40, 40],
            ["a room grown under the card", 32, [{}, {}, {}], 3, 1000, 32],
            ["a room shrunk below the card", 48, [{}, {}, {}], 3, 40, 32],
            ["a room shrunk with home not yet found", 48, [], 3, 40, 32],
            // Sized from home's rows, a heading (10), a row (16) and a row
            // standing apart (24), to 50: whole bands would stand 48, and
            // the row apart counted as a plain row would leave 42.
            ["a room shrunk below a headed card", 64, [{ heading: true }, {}, { gap: true }, {}], 3, 50, 50],
            ["a room too small for a row", 48, [{}, {}, {}], 3, 10, 0]
        ];
        for (const [what, tall, shown, fit, room, want] of refits) {
            const sizes = Object.assign({}, rowSizes, { room: room });
            const refitted = Steady.refitted(tall, { shown, rows: fit, sizes, heading: headed });
            if (refitted !== want)
                failures.push(`${what}: stands ${refitted}, not ${want}`);
        }

        const rowHeights = [
            ["a row", false, undefined, 1, 16],
            ["a row standing apart", false, true, 1, 24],
            ["the first row standing apart", false, true, 0, 24],
            ["a heading among the rows", true, undefined, 2, 14],
            ["the list's first heading", true, undefined, 0, 10],
            ["a heading with a gap", true, true, 2, 14]
        ];
        for (const [what, heading, gap, at, want] of rowHeights) {
            const tall = Steady.tallOf(heading, gap, at, rowSizes);
            if (tall !== want)
                failures.push(`${what}: stands ${tall}, not ${want}`);
        }

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
