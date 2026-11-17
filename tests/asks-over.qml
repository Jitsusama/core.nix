// Drives the launcher's Asks (modules/secret-asks/Asks.qml) as it runs where
// the passwords daemon answers secret asks: over Passwords' own connection,
// stood in here so each ask Asks sends is read, with the real ReadyRows.qml
// and a stand-in launcher whose frames are presented when a case says. A
// waiting ask's row arms on a first Return, which the launcher's Arming.js
// says does nothing but arm, so nothing is sent. A yes says how long since
// the first frame presented its line selected, in that opening; a line
// still selected as the launcher opens again is noted from that opening's
// first frame, so Joel's yes then says it too. A yes for a line that
// changed says no time until a frame presents it as it is now, and a frame
// asked for a line left before it came counts for nothing. Release It from
// the right arrow's verbs counts from its ask's own line. A yes walked
// from outside the launcher is never sent, and a no goes as served. A
// launcher opened from outside, or stepped into Asks from outside, times
// no line until one of Joel's keys moves through what's shown; typing
// starts the time again, and Return never arms. A place he walks into by
// his own key times its line from the first frame. It exits 0 once every
// case holds, and otherwise 1, saying which didn't.
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
        // and the asks to open from outside, as Launcher.qml counts them.
        property int keysTaken: 0
        property string keyTaken: ""
        property bool keyMoved: false
        property int toldFromOutside: 0
        readonly property var sources: [passwords]
        property var awaiting: []
        property var toldNots: []
        property var toldWhys: []

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

        // An opening, asked from outside as every one is.
        function opened() {
            toldFromOutside += 1;
            openingsDrawn += 1;
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

        function becomesAll(source, holds, how) {
        }

        function becameNot(key, why) {
        }

        function afterFrame(then) {
            then();
        }

        function toldNot(name, act, why) {
            toldNots = toldNots.concat([`${act} ${why}`]);
            toldWhys = toldWhys.concat([why ?? ""]);
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

    function last() {
        return link.sent[link.sent.length - 1] ?? null;
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
        steps([arming, untimed, untimedThenAnswered, typedThenAnswered, moving, shown, entryTold,
            reopened,
            reopenedThenMoved, left, moved, changed, stepping, stepped, outside, refused, drawn,
            walkedInto, walkedThenShown, switchedFromOutside, switchedThenAnswered, typing,
            typedAfter, steppedAside, steppedAsideUntimed, switchedMidKey]);
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
        row().run();
        same("an Asks opened from outside times no line, however long it's shown",
            last().shown, undefined);
        stage.press("Return", false);
    }

    function untimedThenAnswered() {
        stage.present(600);
        stage.press("Return", false);
        row().run();
        same("nor do Returns pressed on it", last().shown, undefined);
        stage.press("l", false);
    }

    function typedThenAnswered() {
        stage.press("s", false);
        stage.present(600);
        row().run();
        same("nor letters typed on its find", last().shown, undefined);
    }

    function moving() {
        stage.press("Down", true);
    }

    function shown() {
        stage.present(800);
        row().run();
        same("a key of his moving through the rows times the line from its next frame",
            last().shown >= 800 && last().shown < 5000, true);
        same("and says the opening it was shown in", last().opening, 1);
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
        row().run();
        same("a line still selected as it opens again from outside isn't timed",
            last().shown, undefined);
        stage.press("Escape", false);
    }

    function reopenedThenMoved() {
        stage.present(700);
        row().run();
        same("until a key of his moves, even where nothing can move",
            last().shown >= 700 && last().shown < 5000, true);
        same("in that opening", last().opening, 2);
    }

    function left() {
        stage.selected = null;
        stage.selected = row();
        stage.present(100);
        row().run();
        same("a line selected again counts from coming back, not from when it was first shown",
            last().shown >= 100 && last().shown < 700, true);
    }

    function moved() {
        stage.selected = null;
        stage.selected = row();
        stage.selected = null;
        stage.present(900);
        stage.selected = row();
        row().run();
        same("a frame asked for a line left before it came counts for nothing",
            last().shown, undefined);
        stage.present(100);
    }

    function changed() {
        link.tell("d2");
        stage.selected = row();
        row().run();
        same("a yes for a line that changed says no time before a frame presents it",
            [last().digest, last().shown], ["d2", undefined]);
        stage.present(600);
        row().run();
        same("and counts from the frame that does",
            last().shown >= 600 && last().shown < 700, true);
    }

    function stepping() {
        stage.press("Right", true, () => {
            stage.held = [{ row: root.row() }];
            stage.selected = root.row().verbs()[0];
        });
    }

    function stepped() {
        row().verbs()[0].run();
        same("Release It from the right arrow's verbs counts from its ask's line",
            last().shown >= 600 && last().shown < 5000, true);
        stage.held = [];
        stage.selected = row();
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
        row().verbs()[1].run();
        same("a no goes as served, saying no time", [last().ask, last().shown, last().digest],
            ["refuse", undefined, "d2"]);
    }

    // A frame drawing the selected row as it was says nothing of the ask as
    // it's served now, before the row drawn is made again.
    function drawn() {
        link.tell("d3");
        stage.present(400);
        row().run();
        same("a frame of the row as it was drawn times no yes for the ask as it is now",
            [last().digest, last().shown], ["d3", undefined]);
    }

    // Opened from outside on home, then Passwords by his own Return.
    function walkedInto() {
        stage.open = false;
        stage.held = [];
        stage.selected = null;
        stage.opened();
        stage.press("Return", false, stage.intoPasswords);
    }

    function walkedThenShown() {
        stage.present(600);
        row().run();
        same("a place he walks into by his own key times its line from its first frame",
            last().shown >= 600 && last().shown < 5000, true);
    }

    // Opened from outside on home, a letter of his landing there, then
    // stepped into Passwords from outside too.
    function switchedFromOutside() {
        stage.open = false;
        stage.held = [];
        stage.selected = null;
        stage.opened();
        stage.press("l", false);
    }

    function switchedThenAnswered() {
        stage.toldFromOutside += 1;
        stage.intoPasswords();
        stage.present(600);
        stage.press("Return", false);
        row().run();
        same("a step into Asks from outside is no walk of his, whatever he typed before it",
            last().shown, undefined);
        stage.press("k", true);
    }

    function typing() {
        stage.present(600);
        stage.press("x", false);
    }

    function typedAfter() {
        stage.present(600);
        row().run();
        same("typing after a move starts the line's time again", last().shown, undefined);
    }

    // Opened from outside on home, a letter of his landing there, then
    // Passwords stepped into by neither his key nor an ask from outside.
    function steppedAside() {
        stage.open = false;
        stage.held = [];
        stage.selected = null;
        stage.opened();
        stage.press("l", false);
    }

    function steppedAsideUntimed() {
        stage.intoPasswords();
        stage.present(600);
        row().run();
        same("a step no key of his made is no walk of his, whatever he typed before it",
            last().shown, undefined);
    }

    // Opened from outside on home, then stepped into Passwords from
    // outside while a key of his is being taken.
    function switchedMidKey() {
        stage.open = false;
        stage.held = [];
        stage.selected = null;
        stage.opened();
        stage.press("Return", false, () => {
            stage.toldFromOutside += 1;
            stage.intoPasswords();
        });
        stage.present(600);
        row().run();
        same("nor is a step from outside that lands while a key of his is taken",
            last().shown, undefined);
    }
}
