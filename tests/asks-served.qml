// Drives the launcher's Asks (modules/secret-asks/Asks.qml) over the rows
// the secret-asks daemon itself serves, through Quickshell itself, with
// stand-ins for 1Password's op and logind's loginctl, as the daemon's own
// tests have (modules/secret-asks/tests/daemon.rs), and two asks put up
// before it starts, as an agent puts one up. Asks lists both as they're
// served, says how many wait in the daemon's words, and gives For You each
// as a thing to do that blocks its agent. Return on one is Release It, the
// right arrow's verbs Release It and Refuse It: while the screen is locked
// the yes is refused, said in the daemon's words, and op never runs;
// unlocked, it releases the item to the checker once, and For You lets
// that ask go. Refuse It on the other refuses it, op still having run
// once, and For You holds none. It exits 0 once every case holds, and
// otherwise 1, saying which didn't.
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher
import "Expect.js" as Expect

ShellRoot {
    id: root

    readonly property string runtime: Quickshell.env("XDG_RUNTIME_DIR")
    readonly property string folder: Quickshell.env("FOLDER")
    readonly property string item: Quickshell.env("ITEM")
    // The two asks, as `secret-asks ask` named them.
    readonly property string yes: Quickshell.env("ASK_YES")
    readonly property string no: Quickshell.env("ASK_NO")

    readonly property var expect: Expect.expect
    readonly property var same: Expect.same

    // The launcher as Asks asks it: open, and each refusal said, as its
    // row's key and why (Provider.qml's refused).
    QtObject {
        id: asking

        property bool open: true
        property var refusals: []

        function wants(source) {
            return true;
        }

        function becomesAll(source, holds, how) {
        }

        function becameNot(key, why) {
            refusals = refusals.concat([`${key} ${why}`]);
        }

        function afterFrame(then) {
            then();
        }
    }

    Launcher.Asks {
        id: asks
        settings: ({ name: "asks" })
        launcher: asking
    }

    // The daemon, serving the asks folder over its socket, as its unit
    // runs it; what it says of itself goes on the check's log.
    Process {
        running: true
        command: [Quickshell.env("SECRET_ASKS"), "serve",
            "--socket", `${root.runtime}/daemons/secret-asks.sock`,
            "--asks", `${root.runtime}/secret-asks`,
            "--vocabulary", Quickshell.env("WORDS"), "--item", root.item,
            "--op", `${root.folder}/op`, "--git", Quickshell.env("GIT"),
            "--loginctl", `${root.folder}/loginctl`]
        stderr: SplitParser {
            onRead: line => console.warn(`secret-asks serve: ${line}`)
        }
    }

    // What the stand-in loginctl says of the session's lock.
    // Written before the yes that reads it is sent.
    FileView {
        id: locked
        path: `${root.folder}/locked`
        blockWrites: true
    }

    // Each time op ran, a line.
    FileView {
        id: opRan
        path: `${root.folder}/op-ran`
        blockLoading: true
        printErrors: false
    }

    // What the daemon answered the agent, once it has.
    FileView {
        id: answered
        path: `${root.runtime}/secret-asks/${root.no}.answer.json`
        blockLoading: true
        printErrors: false
    }

    FileView {
        id: ran
        path: `${root.runtime}/secret-asks/${root.yes}.answer.json`
        blockLoading: true
        printErrors: false
    }

    function lines(view) {
        view.reload();
        return view.text().split("\n").filter(line => line !== "").length;
    }

    function answerOf(view) {
        view.reload();
        try {
            return JSON.parse(view.text());
        } catch (_unread) {
            return null;
        }
    }

    function rowOf(id) {
        return asks.everything().find(row => row.key === `ask:${id}`) ?? null;
    }

    // Waits for `holds`, ten seconds at most, then goes on to `then`, or
    // fails saying `what`; `then` failing fails too.
    function until(what, holds, then) {
        waits.what = what;
        waits.holds = holds;
        waits.then = then;
        waits.left = 200;
        waits.start();
    }

    Timer {
        id: waits

        property string what: ""
        property var holds: null
        property var then: null
        property int left: 0

        interval: 50
        repeat: true
        onTriggered: {
            try {
                if (holds()) {
                    stop();
                    then();
                } else if (--left === 0) {
                    stop();
                    throw new Error(`${what} never held`);
                }
            } catch (error) {
                stop();
                console.warn(`FAIL ${error.message}\n${error.stack ?? ""}`);
                Qt.callLater(() => Qt.exit(1));
            }
        }
    }

    Component.onCompleted: until("both asks served",
        () => rowOf(root.yes) !== null && rowOf(root.no) !== null, listed)

    function listed() {
        const row = rowOf(root.yes);
        same("an ask's row names its item", [row.name, rowOf(root.no).name],
            [root.item, root.item]);
        same("Asks says how many wait as the daemon does", asks.counted(), "2 waiting");
        same("For You holds each waiting ask as a thing to do that blocks its agent",
            asks.forYou.map(one => [one.row.key, one.row.forYou.source, one.thing.kind,
                one.thing.blocks, typeof one.thing.since]).sort(),
            [[`ask:${root.no}`, "asks", "todo", 1, "number"],
                [`ask:${root.yes}`, "asks", "todo", 1, "number"]].sort());
        same("Return releases, and the verbs are Release It and Refuse It",
            [row.verb, row.verbs().map(one => one.name)],
            ["release", ["Release It", "Refuse It"]]);
        locked.setText("yes");
        row.run();
        until("a locked yes refused", () => asking.refusals.length > 0, lockedOut);
    }

    function lockedOut() {
        same("a yes while locked is refused in the daemon's words", asking.refusals,
            [`ask:${root.yes} not while the screen is locked`]);
        same("op never ran for it", lines(opRan), 0);
        locked.setText("no");
        rowOf(root.yes).run();
        until("the yes answered", () => answerOf(ran)?.state === "ran"
            && asks.forYou.length === 1, released);
    }

    function released() {
        same("the yes ran the checker", answerOf(ran).exit, 0);
        same("op ran once", lines(opRan), 1);
        same("For You holds only the ask still waiting",
            asks.forYou.map(one => one.row.key), [`ask:${root.no}`]);
        rowOf(root.no).verbs()[1].run();
        until("the no answered", () => answerOf(answered)?.why === "refused by Joel"
            && asks.forYou.length === 0, refused);
    }

    function refused() {
        same("op didn't run for the no", lines(opRan), 1);
        until("none said waiting", () => asks.counted() === "none waiting", () => {
            console.info("every case held");
            Qt.callLater(() => Qt.exit(0));
        });
    }
}
