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
        // Said again "+", gone "-", and waiting unseen "?".
        const readHeaded = list => list.map(row => row.key + (row.repeated ? "+" : "")
            + (row.gone ? "-" : "") + (row.unseen === true ? "?" : "")).join(" ");
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
                "#A=a1 a1", "#A=a1 a1 look", "#A a1 look?"],
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
        // in the foot, unless it's pressing, when it comes in first, above
        // every heading, as For You gathers Agents Asking there
        // (ForYouCells.js's laidOut), since at the end it would read as the
        // last cell's.
        const shownAgain = [
            ["a row under no heading waits", "#A=a1 a1", "#A=a1 a1 look later", "#A a1", 2],
            ["and one above every heading", "#A=a1 a1", "x #A=a1 a1", "#A a1", 1],
            ["but a pressing row comes in first", "#A=a1 a1", "#A=a1 a1 asked!", "asked #A a1",
                0],
            ["and a pressing one above every heading", "#A=a1 a1", "asked! #A=a1 a1",
                "asked #A a1", 0],
            ["and a cell's row after it comes in under its own heading", "#A=a1 a1",
                "asked! #A=a1,a2 a1 a2", "asked #A a1 a2", 0],
            ["and a row of no cell beside it still waits", "#A=a1 a1",
                "asked! #A=a1 a1 look", "asked #A a1", 1],
            ["where the end reads as under none, a pressing row comes in at the end",
                "#A=a1 a1 look", "#A=a1 a1 look asked!", "#A a1 look asked", 0],
            ["a cell's new row still gets its heading past the others", "#A=a1 a1 #B=b1 b1",
                "#A=a1,a2 a1 a2 #B=b1 b1 look", "#A a1 #B b1 #A+ a2", 1],
            ["a pressing row keeps no dim heading said again lit", "#A a1 #B b1 #A+ a2",
                "asked! #A a1 #B b1", "asked #A a1 #B b1 #A+- a2-", 0],
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
                "#A=a1 a1 look-", "#A a1", 0],
            ["a row of no cell after one of none comes in", "#A=a1 a1 look",
                "#A=a1 a1 look try", "#A a1 look try", 0],
            ["a cell's new row after To Look At gets its heading", "#A=a1 a1 #B=b1 b1 look",
                "#A=a1 a1 #B=b1,b2 b1 b2 look", "#A a1 #B b1 look #B+ b2", 0],
            ["a row of no cell after a dim one of none comes in", "#A=a1 a1 look",
                "#A=a1 a1 try", "#A a1 look- try", 0],
            ["a row of an object's own name is its cell's", "#A=a1 a1 #B=b1 b1",
                "#A=a1,__proto__ a1 __proto__ #B=b1 b1", "#A a1 #B b1 #A+ __proto__", 0]
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
            ["and comes in once it says yes", true, "notices #A a1", 0]
        ]) {
            critical.now = now;
            const got = Steady.refreshed(underHeadings("#A=a1 a1"), asking, id);
            if (readHeaded(got.rows) !== want || got.waiting !== waiting)
                failures.push(`${what}: "${readHeaded(got.rows)}" with ${got.waiting} waiting, `
                    + `not "${want}" with ${waiting}`);
        }
        // Kept unseen at a held step's end while it says no, as Held.js's
        // keep lays it, and handed back once it says yes, it comes in as
        // the row it was, all else it holds kept and no longer marked, and
        // the step's own row keeps its mark; with nothing shown, as a list
        // laid afresh from the step's rows is, it comes in unmarked too.
        critical.now = false;
        const keptUnseen = Steady.kept(underHeadings("#A=a1 a1"), asking, id);
        critical.now = true;
        const cameIn = Steady.refreshed(underHeadings("#A=a1 a1"), keptUnseen, id).rows
            .find(row => row.key === "notices");
        const stepOwn = keptUnseen.find(row => row.key === "notices");
        const asSeenRead = row => row === undefined ? "none" : JSON.stringify([
            "unseen" in row, row.side, row.pressing === asking[2].pressing,
            Steady.seen([row], id).rows.length]);
        if (asSeenRead(cameIn) !== JSON.stringify([false, "", true, 1])
                || stepOwn?.unseen !== true)
            failures.push(`a row kept unseen comes in as it was: ${asSeenRead(cameIn)}, `
                + `the step's own ${stepOwn?.unseen}`);
        const laidAfresh = Steady.refreshed([], keptUnseen, id).rows
            .find(row => row.key === "notices");
        if (asSeenRead(laidAfresh) !== JSON.stringify([false, "", true, 1]))
            failures.push(`a row kept unseen, laid afresh: ${asSeenRead(laidAfresh)}`);
        // Listed by a key as it stands (Steady.seen), one kept unseen that
        // has become pressing since comes in first, as a refresh lays it,
        // not counted new and hidden.
        const pressedListed = Steady.seen(keptUnseen, id);
        if (readHeaded(pressedListed.rows) !== "notices #A a1" || pressedListed.waiting !== 0)
            failures.push(`a row kept unseen, pressing, listed: `
                + `"${readHeaded(pressedListed.rows)}" with ${pressedListed.waiting} waiting`);
        // Once the end reads as under none, what waited comes in at the end,
        // after the row that ends the heading, not where it stands right
        // after a cell's rows, where it would read as that cell's.
        const waitedMid = underHeadings("#A=a1 a1 look try");
        waitedMid[2].unseen = true;
        const letIn = Steady.seen(waitedMid, id);
        if (readHeaded(letIn.rows) !== "#A a1 try look" || letIn.waiting !== 0)
            failures.push(`rows waiting let in once the end reads as under none: `
                + `"${readHeaded(letIn.rows)}" with ${letIn.waiting} waiting`);
        // A row marked unseen, then seen, hands on a getter it holds unread,
        // as Member.js's copied does, so it's neither read at its cost nor
        // frozen as it read then.
        let reads = 0;
        let preparing = "a";
        const lazy = Object.defineProperty({ key: "lazy" }, "prepared", {
            get: () => { reads += 1; return preparing; }, enumerable: true
        });
        const lazyKept = Steady.kept(underHeadings("#A=a1 a1"),
            underHeadings("#A=a1 a1").concat([lazy]), id).filter(row => row.key === "lazy");
        const lazySeen = Steady.searched(lazyKept);
        const readsCopying = reads;
        preparing = "b";
        if (lazyKept[0]?.unseen !== true || lazySeen.length !== 1 || "unseen" in lazySeen[0]
                || readsCopying !== 0
                || lazySeen[0].prepared !== "b")
            failures.push(`a getter seen: ${readsCopying} reads copying, `
                + `reading "${lazySeen[0]?.prepared}"`);
        // Found by what's typed in the step, by rank and with no heading, a
        // row kept unseen is shown where it was found, and stays there as
        // the list is said again and again, as it is while rows are held
        // still, though the step hands it back marked each time, as a row
        // that stops matching what's typed comes back from its rows.
        const unheaded = rows => rows.filter(row => row.heading !== true && !row.repeated);
        const lateStep = Steady.kept(underHeadings("#A=a1 a1"),
            underHeadings("#A=a1 a1 look"), id);
        const typedFound = Steady.searched([lateStep[2], lateStep[1]]);
        const typedAgain = Steady.refreshed(typedFound, unheaded(lateStep), id);
        const typedHeld = Steady.held(typedFound, unheaded(lateStep), id);
        const typedThen = Steady.refreshed(typedAgain.rows, unheaded(lateStep), id);
        const typedHeldThen = Steady.held(typedHeld.rows, unheaded(lateStep), id);
        if (readHeaded(typedFound) !== "look a1" || readHeaded(typedAgain.rows) !== "look a1"
                || typedAgain.waiting !== 0 || readHeaded(typedHeld.rows) !== "look a1"
                || readHeaded(typedThen.rows) !== "look a1" || typedThen.waiting !== 0
                || readHeaded(typedHeldThen.rows) !== "look a1"
                || lateStep[2].unseen !== true)
            failures.push(`a row kept unseen, found by what's typed: "${readHeaded(typedFound)}",`
                + ` then "${readHeaded(typedAgain.rows)}" with ${typedAgain.waiting} waiting,`
                + ` then "${readHeaded(typedThen.rows)}" with ${typedThen.waiting},`
                + ` held "${readHeaded(typedHeld.rows)}", then`
                + ` "${readHeaded(typedHeldThen.rows)}"`);
        // What's typed taken away, the row a search found has been seen, so
        // the step's rows as they stand, headings and all, show it and count
        // nothing new; one the search didn't find still waits.
        const clearedSeen = Steady.seen(Steady.seenAmong(lateStep, typedFound.map(id), id), id);
        const clearedUnfound = Steady.seen(Steady.seenAmong(lateStep, [id(lateStep[1])], id),
            id);
        if (readHeaded(clearedSeen.rows) !== "#A a1 look" || clearedSeen.waiting !== 0
                || readHeaded(clearedUnfound.rows) !== "#A a1"
                || clearedUnfound.waiting !== 1)
            failures.push(`a row found, then what's typed taken away: `
                + `"${readHeaded(clearedSeen.rows)}" with ${clearedSeen.waiting} waiting, unfound `
                + `"${readHeaded(clearedUnfound.rows)}" with ${clearedUnfound.waiting}`);
        // Found once, it stays seen as the step is made again, hands it back
        // marked and lists it after other lists were shown, as when Joel
        // searches again or steps into a row and back out: Joel found it by
        // name, so it isn't hidden again. It stands where the step keeps it,
        // at the end, after every row shown, as each row that waited does.
        const madeAgain = Steady.kept(lateStep, underHeadings("#A=a1 a1 look"), id);
        const clearedLater = Steady.seen(Steady.seenAmong(madeAgain, typedFound.map(id), id),
            id);
        if (madeAgain[2]?.unseen !== true || readHeaded(clearedLater.rows) !== "#A a1 look"
                || clearedLater.rows.some(row => row.unseen === true)
                || clearedLater.waiting !== 0)
            failures.push(`a row found, then the step made again and listed: `
                + `"${readHeaded(clearedLater.rows)}" with ${clearedLater.waiting} waiting`);
        // A key that forgets the rows gone may leave no heading above the
        // end, so a row that waited there comes in where it stands, as the
        // refresh after would let it, rather than at a moment no key says.
        const headingsGo = Steady.kept(underHeadings("#A=a1 a1 #Y=y1 y1"),
            underHeadings("y1 look"), id);
        const keptHeaded = Steady.seen(headingsGo, id);
        const headingsForgot = Steady.seen(Steady.forgotten(headingsGo), id);
        if (readHeaded(keptHeaded.rows) !== "#A- a1- #Y- y1" || keptHeaded.waiting !== 1
                || readHeaded(headingsForgot.rows) !== "y1 look"
                || headingsForgot.waiting !== 0)
            failures.push(`a row waiting, its headings forgotten: `
                + `"${readHeaded(keptHeaded.rows)}" with ${keptHeaded.waiting} waiting, then `
                + `"${readHeaded(headingsForgot.rows)}" with ${headingsForgot.waiting}`);
        // A step lists its rows with nothing typed (Launcher.qml's listings):
        // steadied, every row, its count the refresh's; else as they stand,
        // what waits counted, but for one a search found, though its id is
        // an object's own.
        const protoStep = Steady.kept(underHeadings("#A=a1 a1"),
            underHeadings("#A=a1 a1 __proto__"), id);
        const steadiedList = Steady.listed(protoStep, [], id, true);
        const standingList = Steady.listed(protoStep, [], id, false);
        const foundList = Steady.listed(protoStep, ["__proto__"], id, false);
        if (readHeaded(steadiedList.rows) !== "#A a1 __proto__?" || steadiedList.waiting !== null
                || readHeaded(standingList.rows) !== "#A a1" || standingList.waiting !== 1
                || readHeaded(foundList.rows) !== "#A a1 __proto__" || foundList.waiting !== 0)
            failures.push(`a step listed: steadied "${readHeaded(steadiedList.rows)}" with `
                + `${steadiedList.waiting}, standing "${readHeaded(standingList.rows)}" with `
                + `${standingList.waiting}, found "${readHeaded(foundList.rows)}" with `
                + `${foundList.waiting}`);
        // Held still with a row shown that waits unseen, as one Return acted
        // on may be, the rows held are those the rows shown put back, so a
        // cell's new row waits too, rather than slip in among them.
        const heldUnseen = Steady.held(lateStep, underHeadings("#A=a1,a2 a1 a2 look"), id);
        if (readHeaded(heldUnseen.rows) !== "#A a1" || heldUnseen.waiting !== 2)
            failures.push(`held, a row shown that waits unseen: `
                + `"${readHeaded(heldUnseen.rows)}" with ${heldUnseen.waiting} waiting`);
        // A row's id may name what every object has, and is kept as any.
        const ownNames = Steady.refreshed(underHeadings("a1 constructor"),
            underHeadings("a1 constructor __proto__ toString"), id);
        if (readHeaded(ownNames.rows) !== "a1 constructor __proto__ toString")
            failures.push(`rows named as an object's own: "${readHeaded(ownNames.rows)}"`);
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
                "asked #A a1 #B b1", 0],
            ["a new cell's row, its heading said again alone in the step's rows", "#A=a1 a1",
                "#A=a1 a1 #C=c1 c1", "#A a1 #C+ c1", 0],
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
                ["#A=a1 a1 #B=b1 b1 look", "#A=a1 a1 #B=b1 b1 look!"], "look #A a1 #B b1", 0],
            ["and leaves with nothing to dim", "#A=a1 a1 #B=b1 b1",
                ["#A=a1 a1 #B=b1 b1 look", "#A=a1 a1 #B=b1 b1"], "#A a1 #B b1", 0]
        ]) {
            const stepRows = makings.reduce((rows, now) => Steady.kept(rows, underHeadings(now),
                id), underHeadings(shown));
            const listed = Steady.seen(Steady.forgotten(stepRows), id);
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
            ["and a row under no heading", "#A=a1 a1", "#A=a1,a2 a1 a2 look", 2],
            ["and a pressing one, though it would come in first", "#A=a1 a1",
                "asked! #A=a1 a1", 1]
        ]) {
            const got = Steady.held(underHeadings(shown), underHeadings(now), id).waiting;
            if (got !== waiting)
                failures.push(`${what}: ${got} waiting, not ${waiting}`);
        }
        // Held, the rows held are those shown, with nothing come in first.
        const heldRaised = Steady.held(underHeadings("#A=a1 a1 #B=b1 b1"),
            underHeadings("asked! #A=a1 a1 #B=b1 b1"), id);
        if (readHeaded(heldRaised.rows) !== "#A a1 #B b1" || heldRaised.waiting !== 1)
            failures.push(`held, a pressing row come: "${readHeaded(heldRaised.rows)}" with `
                + `${heldRaised.waiting} waiting`);
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
