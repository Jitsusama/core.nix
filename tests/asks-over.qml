// Drives the launcher's Asks (modules/secret-asks/Asks.qml) as it runs where
// the passwords daemon answers secret asks: over Passwords' own connection,
// stood in here so each ask Asks sends is read, with the real ReadyRows.qml
// and a stand-in launcher whose frames are presented when a case says. A
// waiting ask's row arms on a first Return, which the launcher's Arming.js
// says does nothing but arm, so nothing is sent. Every opening is asked
// from outside, so no line is timed until one of Joel's keys moves over the
// rows, never on the find, where a terminal's Up and Down would be his too;
// typing starts the time again, Return never arms, and a place stepped into
// is no move. A yes says how long since the first frame presented its line
// selected after that move, in that opening. A yes for a line that changed
// says no time until a frame presents it as it is now, and a frame asked
// for a line left before it came counts for nothing. Release It from the
// right arrow's verbs counts from its ask's own line. A yes walked from
// outside the launcher is never sent, and a no goes as served. While the
// screen is shared, or before niri has said its casts, an ask's row says
// only that it's hidden, with no acts and no frame timed, and shown again it
// counts nothing until his next move (Niri and Recorder stood in by the
// check). It exits 0 once every case holds, and otherwise 1, saying which
// didn't.
import qs
import Quickshell
import QtQuick
import KeyTimes
import "launcher" as Launcher
import "Arming.js" as Arming
import "Expect.js" as Expect

ShellRoot {
    id: root

    readonly property string id: "0123456789abcdef0123456789abcdef"
    readonly property string key: `row:asks:${root.id}`
    readonly property var same: Expect.same
    // The row as it was made before the screen was shared, as a step the
    // launcher holds keeps it.
    property var madeBefore: null

    // A waiting ask's record as the passwords daemon serves it (secret-asks'
    // src/row.rs), its line at `digest`.
    function record(digest) {
        const thing = `ask:${root.id}`;
        const act = verb => ({ key: `verb:${verb}`, ask: { ask: verb, key: thing, digest } });
        return { key: root.key, kind: "row", row: {
            key: root.key, thing, place: "asks", kind: "ask", state: "waiting",
            name: "op://Private/Kiln/password", glyph: "passwords", hue: "action",
            note: "S1", side: [], attention: 1000, digest, order: 1024,
            acts: [act("release"), act("refuse")],
            drawer: { eyebrow: "S1", title: "op://Private/Kiln/password",
                body: "To fire the kiln.", facts: [] }
        } };
    }

    // Passwords' connection to its daemon, as Asks is handed it: what it
    // holds, told as it changes, and each ask sent through it.
    QtObject {
        id: link

        property var held: ({})
        property string state: "ready"
        property var sent: []
        signal heard(var changed, var gone)

        function ask(ask, then) {
            sent = sent.concat([ask]);
            then({});
        }

        function tell(digest) {
            const next = Object.assign({}, held);
            next[root.key] = root.record(digest);
            held = next;
            heard([root.key], []);
        }

        // An entry, as a manager's list is read in batches over the same connection.
        function tellEntry() {
            const entry = "entry:onepassword:kiln";
            const next = Object.assign({}, held);
            next[entry] = { key: entry, kind: "entry", entry: { title: "Kiln" } };
            held = next;
            heard([entry], []);
        }
    }

    // Passwords, as Asks finds it among the launcher's places.
    QtObject {
        id: passwords

        readonly property var asksLink: link
    }

    // The launcher as Asks asks it, its frames presented as each case says.
    QtObject {
        id: stage

        property bool open: false
        property var selected: null
        // The rows stepped into, the last the one whose verbs are listed.
        property var held: []
        property bool cataloguing: false
        property int openingsDrawn: 0
        // The keys the line took, the last by name and whether it moved,
        // whether they land on the rows rather than the find, and the asks
        // to open from outside, as Launcher.qml counts them.
        property int keysTaken: 0
        property string keyTaken: ""
        property bool keyMoved: false
        property bool keysOverRows: false
        property int toldFromOutside: 0
        readonly property var sources: [passwords]
        property var awaiting: []
        property var toldNots: []
        property var toldWhys: []
        property var toldNames: []
        property var becameNames: []

        function presentedAwait(at, then) {
            awaiting = awaiting.concat([then]);
        }

        // A frame reaching the screen `ago` ms before now, on the frames' clock.
        function present(ago) {
            const then = awaiting;
            awaiting = [];
            for (const one of then)
                one(KeyTimes.now() - ago);
        }

        // An opening, asked from outside as every one is, on the find.
        function opened() {
            toldFromOutside += 1;
            openingsDrawn += 1;
            keysOverRows = false;
            open = true;
        }

        // A key of Joel's, taken as Prompt.qml takes one: named, counted,
        // then said to have moved or not, and what it does done with it,
        // as a Return stepping into a place.
        function press(name, moved, does) {
            keyTaken = name;
            keysTaken += 1;
            keyMoved = moved;
            if (does)
                does();
        }

        // Escape, which leaves the find for the rows and moves nothing.
        function toRows() {
            press("Escape", false, () => {
                stage.keysOverRows = true;
            });
        }

        // A step into Passwords, the asks drawn there with the ask's line
        // selected.
        function intoPasswords() {
            held = [{ row: { key: "place:passwords" } }];
            selected = root.row();
        }

        function placeOf(steps) {
            return steps.map(step => step.row?.key ?? "").join("\u0001");
        }

        function wants(source) {
            return true;
        }

        // Each row held as it becomes, by its key, as the launcher's own
        // becoming holds it (Launcher.qml's becomesAll).
        property var becoming: ({})

        function becomesAll(source, holds, how) {
            becameNames = becameNames.concat([how?.name ?? ""]);
            const now = Object.assign({}, becoming);
            for (const one of holds) {
                now[one.key] = Object.assign({ key: one.key, from: source, row: one.row },
                    how);
            }
            becoming = now;
        }

        function becameNot(key, why) {
        }

        // How many times what was told and not yet read was forgotten.
        property int untolds: 0
        function untold() {
            untolds += 1;
        }

        function afterFrame(then) {
            then();
        }

        function toldNot(name, act, why) {
            toldNots = toldNots.concat([`${act} ${why}`]);
            toldWhys = toldWhys.concat([why ?? ""]);
            toldNames = toldNames.concat([name]);
        }
    }

    Launcher.Asks {
        id: asks

        settings: ({ name: "asks", over: "passwords" })
        launcher: stage
    }

    function row() {
        return asks.everything().find(one => one.shownKey === root.key) ?? null;
    }

    // The ask a verb of the row sends, the first verb, Release It, unless
    // `verb` says, held to have been sent by this case, never an earlier one.
    function sent(verb) {
        const before = link.sent.length;
        if (verb === undefined)
            row().run();
        else
            verb.run();
        same("the verb sent its ask", link.sent.length, before + 1);
        return link.sent[before];
    }

    // How long the yes sent now says its line was shown.
    function shownNow() {
        return root.sent().shown;
    }

    // Whether `ms` is a frame's time `ago` ms back, give or take the run.
    function since(ms, ago) {
        return ms >= ago && ms < ago + 4000;
    }

    // Each step runs after the last, past what its keys left for later, as
    // Asks reads a key once the launcher has done what it does.
    function steps(left) {
        if (left.length === 0) {
            console.info("every case held");
            Qt.exit(0);
            return;
        }
        try {
            left[0]();
        } catch (error) {
            Expect.fail(`${error}`);
            return;
        }
        Qt.callLater(() => root.steps(left.slice(1)));
    }

    Component.onCompleted: {
        link.tell("d1");
        steps([arming, untimed, returned, returnedThenAnswered, typed, typedThenAnswered,
            upOnTheFind, upOnTheFindAnswered, escaped, escapedThenAnswered, moving, shown,
            entryTold, reopened, reopenedThenEscaped, reopenedThenMoved, reopenedAndShown, left,
            moved, changed, stepping, stepped, steppedBack, movedBack, drawn, outside, refused,
            returnedInto, returnedIntoShown, steeredOut, walkedInto, walkedIntoShown,
            walkedThenEscaped, walkedThenMoved, walkedThenMovedShown, toldInPlace, typing,
            typedAfter, movedAgain, typedBeforeItsFrame, typedThenShown, movedOnceMore,
            typedWithReturn, typedWithReturnShown, movedThenOpened, movedThenOpenedShown,
            overRowsThenTold, toldOverRows, toldOverRowsShown, landed, landedThenMoved,
            landedThenShown, commanded, commandedThenRun, commandedThenMoved,
            commandedThenShown, timedBeforeSharing, timedMove, sharing, sharedYes,
            sharingMoved, sharingMovedAsked, unshared, unsharedShown, unsharedMoved,
            unsharedMovedShown, unheard, ownRecording, sharedNo, heldThenShared]);
    }

    function arming() {
        stage.opened();
        same("a waiting ask's row arms, saying a second Return releases", row()?.arms,
            "releases");
        same("and a first Return on it only arms", Arming.pressed(Arming.rest(), root.key,
            { keysTaken: 1, counted: 1, now: 0, wait: 300 }).act, "armed");
        same("so nothing is sent", link.sent, []);
    }

    // Opened from outside, as any process running as Joel may open it.
    function untimed() {
        stage.selected = row();
        stage.present(800);
        same("an Asks opened from outside times no line, however long it's shown",
            shownNow(), undefined);
    }

    function returned() {
        stage.press("Return", false);
    }

    function returnedThenAnswered() {
        stage.present(600);
        same("nor do Returns pressed on it", shownNow(), undefined);
    }

    function typed() {
        stage.press("s", false);
    }

    function typedThenAnswered() {
        stage.present(600);
        same("nor letters typed on its find", shownNow(), undefined);
    }

    // Up on the find moves through the rows, but is a terminal's commonest
    // key, as its history's, before the Return that runs what it brought back.
    function upOnTheFind() {
        stage.press("Up", true);
    }

    function upOnTheFindAnswered() {
        stage.present(600);
        same("nor a move on the find, which a terminal's Up would be", shownNow(), undefined);
    }

    function escaped() {
        stage.toRows();
    }

    function escapedThenAnswered() {
        stage.present(600);
        same("nor the Escape that leaves the find for the rows", shownNow(), undefined);
    }

    function moving() {
        stage.press("j", true);
    }

    function shown() {
        stage.present(800);
        const said = root.sent();
        same("a key of his moving over the rows times the line from its next frame",
            since(said.shown, 800), true);
        same("and says the opening it was shown in", said.opening, 1);
    }

    // Entries come in batches as a manager is read, which would push back the frame a line
    // waits on each time, were Asks told of them.
    function entryTold() {
        const before = asks.everything();
        const waiting = stage.awaiting.length;
        link.tellEntry();
        same("an entry told over the connection leaves Asks' rows as they were",
            asks.everything() === before, true);
        same("and asks no frame for its line again", stage.awaiting.length, waiting);
    }

    function reopened() {
        stage.open = false;
        same("closing the launcher forgets every line's frame", asks.seen, {});
        stage.opened();
        stage.present(700);
        same("a line still selected as it opens again from outside isn't timed",
            shownNow(), undefined);
        stage.toRows();
    }

    // Escape is a terminal's key as much as the launcher's, and moves nothing here.
    function reopenedThenEscaped() {
        stage.present(700);
        same("nor after an Escape the launcher takes for no move", shownNow(), undefined);
    }

    function reopenedThenMoved() {
        stage.press("Down", true);
    }

    function reopenedAndShown() {
        stage.present(700);
        const said = root.sent();
        same("until a key of his moves over the rows, even where nothing can move",
            since(said.shown, 700), true);
        same("in that opening", said.opening, 2);
    }

    function left() {
        stage.selected = null;
        stage.selected = row();
        stage.present(100);
        same("a line selected again counts from coming back, not from when it was first shown",
            since(shownNow(), 100) && shownNow() < 700, true);
    }

    function moved() {
        stage.selected = null;
        stage.selected = row();
        stage.selected = null;
        stage.present(900);
        stage.selected = row();
        same("a frame asked for a line left before it came counts for nothing",
            shownNow(), undefined);
        stage.present(100);
    }

    function changed() {
        link.tell("d2");
        stage.selected = row();
        const said = root.sent();
        same("a yes for a line that changed says no time before a frame presents it",
            [said.digest, said.shown], ["d2", undefined]);
        stage.present(600);
        same("and counts from the frame that does", since(shownNow(), 600), true);
    }

    // The right arrow lists the ask's verbs, which the launcher takes as no move.
    function stepping() {
        stage.press("Right", false, () => {
            stage.held = [{ row: root.row() }];
            stage.selected = root.row().verbs()[0];
        });
    }

    function stepped() {
        same("Release It from the right arrow's verbs counts from its ask's line",
            since(root.sent(row().verbs()[0]).shown, 600), true);
    }

    // Back from the verbs by Left, then a move over the rows timing the line again.
    function steppedBack() {
        stage.press("Left", false, () => {
            stage.held = [];
            stage.selected = root.row();
        });
    }

    function movedBack() {
        stage.press("j", true);
    }

    // A frame drawing the selected row as it was says nothing of the ask as
    // it's served now, before the row drawn is made again.
    function drawn() {
        stage.present(800);
        link.tell("d3");
        stage.present(400);
        const said = root.sent();
        same("a frame of the row as it was drawn times no yes for the ask as it is now",
            [said.digest, said.shown], ["d3", undefined]);
    }

    function outside() {
        const before = link.sent.length;
        stage.cataloguing = true;
        row().run();
        stage.cataloguing = false;
        same("a yes walked from outside the launcher is never sent", link.sent.length, before);
        same("and is told as not done", stage.toldNots.length, 1);
        // A walk answers done where nothing told says why not (Reached.js), so the agent that
        // asked would wait out its ask were the launcher's word for it ever empty.
        same("saying why, in the launcher's words", stage.toldWhys[0] !== "", true);
    }

    function refused() {
        const said = root.sent(row().verbs()[1]);
        same("a no goes as served, saying no time", [said.ask, said.shown, said.digest],
            ["refuse", undefined, "d3"]);
    }

    // Opened from outside on home, Passwords selected there, then stepped into by a Return of
    // his and nothing before it, as his Returns typed for a terminal it took the keys from.
    function returnedInto() {
        stage.open = false;
        stage.held = [];
        stage.selected = null;
        stage.opened();
        stage.press("Return", false, stage.intoPasswords);
    }

    function returnedIntoShown() {
        stage.present(600);
        same("a place a lone Return steps into times nothing", shownNow(), undefined);
    }

    // Back on home, where a letter of his finds Passwords and his Return steps
    // into it, as a terminal's command typed and run would.
    function steeredOut() {
        stage.held = [];
        stage.selected = null;
        stage.press("p", false);
    }

    function walkedInto() {
        stage.press("Return", false, stage.intoPasswords);
    }

    function walkedIntoShown() {
        stage.present(600);
        same("nor a place letters and a Return step into", shownNow(), undefined);
    }

    function walkedThenEscaped() {
        stage.toRows();
    }

    function walkedThenMoved() {
        stage.press("k", true);
    }

    function walkedThenMovedShown() {
        stage.present(600);
        same("until a key of his moves over its rows", since(shownNow(), 600), true);
    }

    // An ask from outside in the place he moved in, which may select another line.
    function toldInPlace() {
        stage.toldFromOutside += 1;
        stage.selected = null;
        stage.selected = row();
        stage.present(600);
        same("a line an ask from outside selects is no move of his", shownNow(), undefined);
        stage.press("k", true);
    }

    function typing() {
        stage.present(600);
        stage.press("x", false);
    }

    function typedAfter() {
        stage.present(600);
        same("typing after a move starts the line's time again", shownNow(), undefined);
    }

    // A move, then a letter before the frame the move asked for reaches the screen.
    function movedAgain() {
        stage.press("j", true);
    }

    function typedBeforeItsFrame() {
        stage.press("x", false);
    }

    function typedThenShown() {
        stage.present(600);
        same("a frame a move asked for before a letter times nothing", shownNow(), undefined);
    }

    // A letter and a Return read at once, as keys typed fast are.
    function movedOnceMore() {
        stage.press("j", true);
    }

    function typedWithReturn() {
        stage.present(600);
        stage.press("x", false);
        stage.press("Return", false);
    }

    function typedWithReturnShown() {
        same("a letter read with the Return after it starts the time again", shownNow(),
            undefined);
    }

    // A move, and an ask from outside before Asks reads the move.
    function movedThenOpened() {
        stage.press("j", true, () => {
            stage.toldFromOutside += 1;
        });
    }

    function movedThenOpenedShown() {
        stage.present(600);
        same("a move an ask from outside follows arms nothing", shownNow(), undefined);
    }

    // His Escape, then an ask from outside that leaves the keys over the rows.
    function overRowsThenTold() {
        stage.toRows();
    }

    function toldOverRows() {
        stage.toldFromOutside += 1;
        stage.press("k", true);
    }

    function toldOverRowsShown() {
        stage.present(600);
        same("a move after an ask from outside times nothing until his Escape", shownNow(),
            undefined);
    }

    // An opening, then a Return that lands, which puts the keys over the rows unescaped.
    function landed() {
        stage.open = false;
        stage.opened();
        stage.press("Return", false, () => {
            stage.keysOverRows = true;
        });
    }

    function landedThenMoved() {
        stage.press("Up", true);
    }

    function landedThenShown() {
        stage.present(600);
        same("a move over rows a landing Return reached times nothing", shownNow(), undefined);
    }

    // An opening, then the command line, which leaves the keys over the rows once run.
    function commanded() {
        stage.open = false;
        stage.opened();
        stage.press(":", false);
    }

    function commandedThenRun() {
        stage.press("Return", false, () => {
            stage.keysOverRows = true;
        });
    }

    function commandedThenMoved() {
        stage.press("j", true);
    }

    function commandedThenShown() {
        stage.present(600);
        same("a move over rows the command line left times nothing", shownNow(), undefined);
    }

    // A line timed by his Escape and move, then the screen shared to a call.
    function timedBeforeSharing() {
        stage.open = false;
        stage.opened();
        stage.selected = row();
        stage.toRows();
    }

    function timedMove() {
        stage.press("j", true);
    }

    function sharing() {
        stage.present(800);
        same("the line is timed before the screen is shared", Object.keys(asks.seen),
            [root.key]);
        root.madeBefore = row();
        Niri.casts = { "1": { kind: "PipeWire", session_id: 1 } };
        stage.selected = row();
        const hidden = "Ask, Hidden While You Share";
        same("while the screen is shared an ask's row names only that it's hidden",
            row()?.name, hidden);
        same("and says nothing of where it's from", row()?.note, "");
        // A ready row's drawer is made as it's drawn (ReadyRows.qml's drawing).
        const drawer = row()?.drawer?.() ?? {};
        same("its drawer says neither what it asks for, where from, nor why",
            [drawer.title, drawer.eyebrow, drawer.body, drawer.facts?.length],
            [hidden, "", "", 0]);
        same("it has no act, so no key answers it unseen",
            [row()?.arms, typeof row()?.run, row()?.verbs ?? null], ["", "undefined", null]);
        same("it still waits for him in For You, hidden there too",
            asks.forYou.map(one => one.row.name), [hidden]);
        same("and its line's time starts again", asks.seen, {});
    }

    // A yes from the row as it was before the share, as a step held into
    // its verbs still offers, is never sent, and what's told of it names
    // only that it's hidden.
    function sharedYes() {
        const before = link.sent.length;
        const told = stage.toldNames.length;
        root.madeBefore.run();
        same("a yes from a row made before the share is never sent while it's shared",
            link.sent.length, before);
        same("and what's told of it names only that it's hidden", stage.toldNames.slice(told),
            ["Ask, Hidden While You Share"]);
        same("saying it's the share that withholds it, not a missing key press",
            stage.toldWhys.slice(told), ["not while your screen is shared"]);
        // Only what releases nothing goes while it's shared, so an act a
        // daemon adds later that grants something is withheld too.
        same("nor is any ask but a no or a revoke",
            asks.stamped(root.key, { ask: "grant", thing: "op://Private/Kiln/password" }), null);
        same("while a revoke, taking a standing yes away, still goes",
            asks.stamped(root.key, { ask: "revoke", thing: "op://Private/Kiln/password" })?.ask,
            "revoke");
    }

    function sharingMoved() {
        stage.awaiting = [];
        stage.press("k", true);
    }

    function sharingMovedAsked() {
        same("a move over a hidden line asks no frame of it", stage.awaiting.length, 0);
    }

    function unshared() {
        Niri.casts = {};
        stage.selected = row();
        same("the share over, the ask's row says what it asks for again", row()?.name,
            "op://Private/Kiln/password");
    }

    function unsharedShown() {
        stage.present(600);
        same("a line shown again after a share counts nothing before his next move",
            shownNow(), undefined);
    }

    function unsharedMoved() {
        stage.press("j", true);
    }

    function unsharedMovedShown() {
        stage.present(700);
        same("and his next move times it from its next frame", since(shownNow(), 700), true);
    }

    function unheard() {
        Niri.castsHeard = false;
        same("before niri says its casts, an ask's row is hidden", row()?.name,
            "Ask, Hidden While You Share");
        const waiting = "not until it's known whether your screen is shared";
        same("saying on the row that it waits on knowing, not on a share",
            [row()?.note, row()?.drawer?.()?.body], [waiting, waiting]);
        const told = stage.toldWhys.length;
        root.madeBefore.run();
        same("and a yes then is told it waits on knowing, not on a share",
            stage.toldWhys.slice(told), ["not until it's known whether your screen is shared"]);
        // A no from the row as it was is held as the row is now, saying it
        // waits on knowing too.
        stage.becoming = {};
        root.sent(root.madeBefore.verbs()[1]);
        const held = () => Object.values(stage.becoming).find(one => one.from !== stage) ?? {};
        same("a no while it's unknown is held as a row waiting on knowing", held().row?.note,
            waiting);
        Niri.casts = { "1": { kind: "PipeWire", session_id: 1 } };
        Niri.castsHeard = true;
        same("niri then saying there's a share, the row no longer says it waits on knowing",
            [row()?.name, row()?.note], ["Ask, Hidden While You Share", ""]);
        same("nor does the row it's held as", [held().row?.name, held().row?.note],
            ["Ask, Hidden While You Share", ""]);
        Niri.casts = {};
        stage.becoming = {};
    }

    // The shell's own recording of one window shows nothing of the launcher.
    function ownRecording() {
        const before = Recorder.windowSession;
        Recorder.windowSession = 7;
        Niri.casts = { "2": { kind: "PipeWire", session_id: 7 } };
        same("the shell's own recording of a window hides no ask", row()?.name,
            "op://Private/Kiln/password");
        Niri.casts = {};
        Recorder.windowSession = before;
    }

    // A no from the row as it was before the share goes, since it releases
    // nothing, but the row it's held as names only that it's hidden.
    function sharedNo() {
        Niri.casts = { "1": { kind: "PipeWire", session_id: 1 } };
        const said = root.sent(root.madeBefore.verbs()[1]);
        same("a no from a row made before the share still goes", said.ask, "refuse");
        same("and the row it's held as names only that it's hidden",
            stage.becameNames.slice(-1), ["Ask, Hidden While You Share"]);
        Niri.casts = {};
    }

    // A no from the line just before the share is held as the row it
    // becomes until the daemon answers; the share hides that row too, and
    // the name told if the no is refused or comes late.
    function heldThenShared() {
        // Another source's row keyed as the ask's, so only whose hold it
        // is keeps it apart.
        const other = { key: "wifi:kiln", from: stage, row: { name: "Kiln", shownKey: root.key },
            name: "Kiln", act: "join" };
        stage.becoming = { "wifi:kiln": other };
        root.sent(row().verbs()[1]);
        const held = () => Object.values(stage.becoming).find(one => one.from !== stage) ?? {};
        const real = "op://Private/Kiln/password";
        same("a no before the share is held under the ask's own name", held().name, real);
        const act = held().act;
        const done = held().done;
        const untolds = stage.untolds;
        Niri.casts = { "1": { kind: "PipeWire", session_id: 1 } };
        const hidden = "Ask, Hidden While You Share";
        same("the share holds it again named only as hidden, its row hidden too",
            [held().name, held().row?.name, held().act], [hidden, hidden, act]);
        same("still saying when it's done, as it did", held().done === done, true);
        same("what was told before the share is forgotten", stage.untolds, untolds + 1);
        same("another source's hold is left as it was", stage.becoming["wifi:kiln"] === other,
            true);
        Niri.casts = {};
        same("the share ended holds it again as itself", [held().name, held().row?.name],
            [real, real]);
        same("still saying when it's done after, as it did", held().done === done, true);
    }
}
