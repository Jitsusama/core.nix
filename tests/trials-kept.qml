// Keeps the launcher's openings (modules/quickshell/Trials.qml) through
// Quickshell itself, offscreen, as the `the-launcher-keeps-its-trials-one-by-one`
// check runs it: `qs -p shell.qml` beside Trials.qml, launcher/Trials.js and
// Expect.js.
// The list the shell kept before lines is carried into them as it starts,
// the oldest past `remember` let go; a close adds its own opening's line
// and touches none before it, so what it writes is the same however many
// are kept; and a start with too many lines cuts them, the next close
// adding to the file cut rather than the one it replaced. It exits 0 once
// every case holds, and otherwise 1, saying which didn't.
import Quickshell
import Quickshell.Io
import QtQuick
import "Expect.js" as Expect

ShellRoot {
    id: root

    property string stage: "listing"
    // The file's lines as last read, and those it held before a close.
    property var lines: []
    property var before: []
    // The Trials the loader made, as itself.
    readonly property Trials made: trials.item as Trials

    function read() {
        kept.reload();
        return kept.text().split("\n").filter(line => line !== "");
    }

    // An opening with a letter typed and nothing picked, closed.
    function openAndClose() {
        before = read();
        root.made.opened();
        root.made.pressed("letters");
        root.made.closed();
    }

    // Five openings in the list the shell kept before lines, of which the
    // newest three are kept.
    FileView {
        id: listed
        path: Quickshell.statePath("launcher-trials.json")
        blockLoading: true
        printErrors: false
        onSaved: {
            root.stage = "carrying";
            trials.active = true;
        }
    }

    FileView {
        id: kept
        path: Quickshell.statePath("launcher-trials.jsonl")
        blockLoading: true
        printErrors: false
    }

    Loader {
        id: trials
        active: false
        sourceComponent: Component {
            Trials {
                remember: 3
            }
        }
    }

    // What the file holds is read until it says what the stage waits for:
    // the pipe writes on its own time.
    Timer {
        interval: 50
        repeat: true
        running: root.stage !== "listing" && root.stage !== "done"
        onTriggered: {
            root.lines = root.read();
            // The openings listed before are 1 to 5; one closed here is new.
            const ats = root.lines.map(line => JSON.parse(line).at)
                .map(at => at <= 5 ? at : "new").join(" ");
            if (root.stage === "carrying" && ats === "3 4 5") {
                // Asked with `in`, since Trials has no such list for the lint to
                // find, while its own members are seen.
                if (!("remember" in root.made) || "trials" in root.made)
                    return Expect.fail("the shell holds the openings");
                root.stage = "closing";
                root.openAndClose();
            } else if (root.stage === "closing" && root.lines.length === 4) {
                if (root.lines.slice(0, 3).join("\n") !== root.before.join("\n"))
                    return Expect.fail(`a close rewrote what came before it: ${root.lines}`);
                if (JSON.parse(root.lines[3]).keys.letters !== 1)
                    return Expect.fail(`the close's line isn't its opening: ${root.lines[3]}`);
                root.stage = "closing again";
                root.openAndClose();
            } else if (root.stage === "closing again" && root.lines.length === 5) {
                // With one more kept, the second close still adds its line
                // alone.
                if (root.lines.slice(0, 4).join("\n") !== root.before.join("\n"))
                    return Expect.fail(`a second close rewrote what came before it: ${root.lines}`);
                if (Array.isArray(JSON.parse(root.lines[4])))
                    return Expect.fail(`a close wrote a list: ${root.lines[4]}`);
                // Started again with five lines, the shell keeps three, the
                // list from before left unread now there are lines.
                root.stage = "starting again";
                trials.active = false;
                trials.active = true;
            } else if (root.stage === "starting again" && root.lines.length === 3
                && ats === "5 new new") {
                root.stage = "closing after the cut";
                root.openAndClose();
            } else if (root.stage === "closing after the cut" && root.lines.length === 4) {
                root.stage = "done";
                console.info("every case holds");
                Qt.callLater(() => Qt.exit(0));
            }
        }
    }

    Timer {
        interval: 20000
        running: true
        onTriggered: Expect.fail(`stuck at ${root.stage}, the file holding ${root.lines}`)
    }

    Component.onCompleted: listed.setText(JSON.stringify([1, 2, 3, 4, 5].map(at => ({
        at, home: "today", keys: { letters: 0 }
    }))))
}
