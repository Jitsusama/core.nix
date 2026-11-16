// Drives the launcher's Asks (modules/secret-asks/Asks.qml) as it runs where
// the passwords daemon answers secret asks: over Passwords' own connection,
// stood in here so each ask Asks sends is read, with the real ReadyRows.qml
// and a stand-in launcher whose frames are presented when a case says. A
// waiting ask's row arms on a first Return, which the launcher's Arming.js
// says does nothing but arm, so nothing is sent. A yes says how long since
// the first frame presented its line selected, in that opening; a line
// still selected as the launcher opens again is noted from that opening's
// first frame, so Joel's yes then says it too. A yes for a line that
// changed says no time until a frame presents it as it is now. A yes walked
// from outside the launcher is never sent, and a no goes as served. It
// exits 0 once every case holds, and otherwise 1, saying which didn't.
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
        property bool cataloguing: false
        property int openingsDrawn: 0
        readonly property var sources: [passwords]
        property var awaiting: []
        property var toldNots: []

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

        function opened() {
            openingsDrawn += 1;
            open = true;
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

    Component.onCompleted: {
        try {
            link.tell("d1");
            arming();
            shown();
            reopened();
            left();
            changed();
            outside();
            refused();
            console.info("every case held");
            Qt.callLater(() => Qt.exit(0));
        } catch (error) {
            Expect.fail(`${error}`);
        }
    }

    function arming() {
        stage.opened();
        same("a waiting ask's row arms, saying a second Return releases", row()?.arms,
            "releases");
        same("and a first Return on it only arms", Arming.pressed(Arming.rest(), root.key,
            { keysTaken: 1, counted: 1, now: 0, wait: 300 }).act, "armed");
        same("so nothing is sent", link.sent, []);
    }

    function shown() {
        stage.selected = row();
        stage.present(800);
        row().run();
        same("a yes says how long since its line was presented selected",
            last().shown >= 800 && last().shown < 5000, true);
        same("and the opening it was shown in", last().opening, 1);
    }

    function reopened() {
        stage.open = false;
        stage.opened();
        stage.present(700);
        row().run();
        same("a line still selected as the launcher opens again is noted from its first frame",
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

    function outside() {
        const before = link.sent.length;
        stage.cataloguing = true;
        row().run();
        stage.cataloguing = false;
        same("a yes walked from outside the launcher is never sent", link.sent.length, before);
        same("and is told as not done", stage.toldNots.length, 1);
    }

    function refused() {
        row().verbs()[1].run();
        same("a no goes as served, saying no time", [last().ask, last().shown, last().digest],
            ["refuse", undefined, "d2"]);
    }
}
