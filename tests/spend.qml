// Drives the AI spend piece (modules/quickshell/pieces/Spend.qml) and the
// tokens piece (pieces/Tokens.qml) through Quickshell itself, over the
// status spend-watch wrote from a small history beside this, and over
// states written by hand, with niri, do not disturb, the sessions and the
// shortcuts stood in for. With nothing written there's no piece; what the
// watch wrote is drawn as it says, with a way to see where it went; a cap
// near its end wants Joel, in the alert colour, with none of the watch's
// actions while its socket isn't there to do them (tests/spend-told.qml
// asks them of it); while the screen is shared the amounts go; a watch
// that stopped writing is a warning with a way to start it; a file of
// another version is no piece, and nor is one taken away again. The
// tokens piece is there only while the watch's `tokens` warns: as the
// watch words it, wanting Joel, said as it arrives and again as its level
// steps down, with See What Spends and Pause for a session that's running
// and nothing else; urgent in the alert colour once out; quiet while
// shared; saying what can't be read; and gone while the watch has stopped.
// The launcher's collection is tests/spend-served.qml's. It exits 0 once
// every case holds, and otherwise 1, saying which didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick

ShellRoot {
    id: root

    readonly property string runtime: Quickshell.env("XDG_RUNTIME_DIR")
    readonly property real now: Date.now() / 1000
    readonly property string written: real.text()

    function said(value, badge, hint, context) {
        return { value: value, badge: badge, hint: hint, context: context };
    }

    function status(fields) {
        return JSON.stringify(Object.assign({
            v: 1,
            heartbeat: root.now,
            active: false,
            role: "text",
            wants: false,
            title: "AI spend",
            open: said("$148 today · usual", "", "$148 today · usual", "at list prices"),
            quiet: said("today · usual", "", "today · usual", "at list prices"),
            actions: []
        }, fields));
    }

    readonly property string alert: status({
        active: true,
        role: "alert",
        wants: true,
        open: said("$3,642 of $4,000 this week", "91%", "The weekly cap is 91% spent",
            "a 60% chance of the cap by Thu 14:00"),
        quiet: said("the weekly cap is close", "91%", "The weekly cap is close", ""),
        actions: [{
            kind: "go",
            name: "Go to the session",
            run: ["spend-watch", "go", "s1"]
        }, {
            kind: "mark",
            name: "It's expected",
            run: ["spend-watch", "findings", "expect", "k"]
        }]
    })

    readonly property var running: ({
        level: "hour",
        now: false,
        title: "Tokens Run Out in About 40 Min",
        value: "resets 14:00",
        context: "at this hour's rate \u00b7 62% used",
        hint: "Tokens run out in about 40 min, at 14:00 they reset",
        quiet: {
            title: "Tokens Run Out Soon",
            value: "resets 14:00",
            context: "at this hour's rate",
            hint: "Tokens run out soon"
        },
        actions: [
            { kind: "spends", name: "See What Spends" },
            { kind: "pause", name: "Pause 15J7N3", session: "s-running", quest: "Q" },
            { kind: "pause", name: "Pause Gone", session: "s-gone", quest: "Q" },
            { kind: "stop", name: "Stop" }
        ]
    })

    // Each case: what the watch wrote (null: nothing), whether the screen
    // is shared, and what the piece makes of it, and the tokens piece where
    // it says.
    readonly property var cases: [{
        what: "a watch that never wrote",
        status: null,
        holds: piece => !piece.present && !piece.active,
        tokens: piece => !piece.present && !piece.active
    }, {
        what: "tokens running out",
        status: status({ tokens: root.running }),
        holds: piece => piece.present,
        tokens: piece => piece.present && piece.active && piece.wants && !piece.now
            && piece.role === "warning" && piece.arrives && piece.withdraws
            && piece.telling === "hour" && piece.title === "Tokens Run Out in About 40 Min"
            && piece.value === "resets 14:00" && piece.context === "at this hour's rate \u00b7 62% used"
            && piece.hint === piece.title
            && piece.message === "resets 14:00 \u00b7 at this hour's rate \u00b7 62% used"
            && piece.actions.map(action => action.name).join(", ")
                === "See What Spends, Pause The Launcher's Shape"
    }, {
        what: "pausing the session that spends most",
        status: status({ tokens: root.running }),
        act: (piece, tokens) => tokens.actions[1].run(),
        holds: piece => Sessions.steered === "ADDR1 pause"
    }, {
        what: "tokens running out, while sharing",
        status: status({ tokens: root.running }),
        sharing: true,
        holds: piece => piece.present,
        tokens: piece => piece.wants && piece.title === "Tokens Run Out Soon"
            && piece.hint === "Tokens Run Out Soon" && !piece.message.includes("%")
    }, {
        what: "tokens that can't be read",
        status: status({ tokens: Object.assign({}, root.running, { unread: "can't be read since 10:12" }) }),
        holds: piece => piece.present,
        tokens: piece => piece.wants && piece.context === "can't be read since 10:12"
    }, {
        what: "tokens out",
        status: status({
            tokens: Object.assign({}, root.running, { level: "out", now: true, title: "Tokens Ran Out" })
        }),
        holds: piece => piece.present,
        tokens: piece => piece.now && piece.role === "alert" && piece.telling === "out"
            && piece.title === "Tokens Ran Out" && piece.message.startsWith("resets 14:00")
    }, {
        what: "tokens while the watch has stopped",
        status: status({ heartbeat: root.now - 600, tokens: root.running }),
        holds: piece => piece.value === "not watching",
        tokens: piece => !piece.present && !piece.active
    }, {
        // A day of logs and no feed: list prices are all there is, and a
        // day has no usual or forecast yet, so there's nothing to add.
        what: "what the watch wrote",
        status: root.written,
        holds: piece => piece.present && !piece.silent && /today/.test(piece.value)
            && piece.title === "AI spend" && piece.context === ""
            && piece.actions.some(action => action.name === "Show where it went"
                && action.chord.map(key => key.text).join("+") === "Super+U")
    }, {
        what: "a cap close to its end",
        status: root.alert,
        holds: piece => piece.active && piece.role === "alert" && piece.wants
            && piece.value === "$3,642 of $4,000 this week"
            && piece.hint === "The weekly cap is 91% spent"
            && piece.actions.map(action => action.name).join(", ") === "Show where it went"
    }, {
        what: "a cap close to its end, while sharing",
        status: root.alert,
        sharing: true,
        holds: piece => piece.wants && piece.value === "the weekly cap is close"
            && !piece.value.includes("$") && !piece.hint.includes("$") && piece.context === ""
    }, {
        what: "a watch that stopped",
        status: status({ heartbeat: root.now - 600 }),
        holds: piece => piece.active && piece.role === "warning" && !piece.wants
            && piece.value === "not watching"
            && piece.actions.some(action => action.name === "Start the watch")
    }, {
        what: "a file of another version",
        status: JSON.stringify({ v: 2, open: {}, quiet: {} }),
        holds: piece => !piece.present
    }, {
        what: "what the watch wrote, again",
        status: root.written,
        holds: piece => piece.present && /today/.test(piece.value)
    }, {
        what: "the file taken away",
        status: null,
        holds: piece => !piece.present && !piece.active
    }]

    property int at: 0
    property int left: 0
    property bool acted: false

    Spend {
        id: piece
        settings: ({ status: `${root.runtime}/status.json` })
    }

    Tokens {
        id: tokensPiece
        settings: ({ status: `${root.runtime}/status.json` })
    }

    FileView {
        id: real
        path: `${root.runtime}/written/status.json`
    }

    FileView {
        id: file
        path: `${root.runtime}/status.json`
        printErrors: false
    }

    // Takes the status file away, as nothing having written one leaves it.
    Process {
        id: removing
        command: ["rm", "-f", `${root.runtime}/status.json`]
    }

    function begin() {
        const step = cases[at];
        Quiet.sharing = step.sharing ?? false;
        if (step.status === null)
            removing.running = true;
        else
            file.setText(step.status);
        left = 100;
        acted = false;
        waiting.start();
    }

    // Quickshell reads the files in the background, so each case waits for
    // the piece to hold, for five seconds at most.
    Timer {
        id: waiting
        interval: 50
        repeat: true
        onTriggered: {
            const step = root.cases[root.at];
            if (step.act && !root.acted && piece.present) {
                root.acted = true;
                step.act(piece, tokensPiece);
            }
            if (step.holds(piece) && (step.tokens === undefined || step.tokens(tokensPiece))) {
                stop();
                if (++root.at === root.cases.length)
                    Qt.exit(0);
                else
                    root.begin();
            } else if (--root.left === 0) {
                stop();
                console.warn(`FAIL ${step.what}: present ${piece.present}, active ${piece.active},`
                    + ` role ${piece.role}, wants ${piece.wants}, value "${piece.value}",`
                    + ` hint "${piece.hint}", context "${piece.context}",`
                    + ` actions ${piece.actions.map(action => action.name).join(", ")};`
                    + ` tokens present ${tokensPiece.present}, now ${tokensPiece.now},`
                    + ` title "${tokensPiece.title}", context "${tokensPiece.context}",`
                    + ` actions ${tokensPiece.actions.map(action => action.name).join(", ")}`);
                Qt.exit(1);
            }
        }
    }

    Component.onCompleted: begin()
}
