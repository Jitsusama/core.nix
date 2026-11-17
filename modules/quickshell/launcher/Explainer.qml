// Explain It's machinery, for any row a session is behind: it asks the
// realm's node what the session is (Explaining.js's command), and hands
// what Joel would know it by to Ask's warm pi on the machine's quick model,
// whose answer then stands in the act's place, its first sentence as its
// name and the whole of it read, as On the Selection's answers do. One
// session is explained at a time; asking another stops the first.
import Quickshell
import Quickshell.Io
import QtQuick
import "Explaining.js" as Explaining
import "OnSelection.js" as OnSelection

Scope {
    id: explainer

    // The realm whose node is asked, Ask's place, which answers, and the
    // act's words: its question, with {facts}, and what each stage says.
    property string realm: ""
    property var asker: null
    property var said: ({})

    // The row explained last, by its act's key, and how it stands:
    // "reading" while the node is asked, "asking" once Ask is, "unknown"
    // when the node knows no such session, "failed" when it couldn't say,
    // with the question Ask was given. Replaced whole, so bindings hear it.
    property string explaining: ""
    property var explained: ({})
    // The words the node is asked with while it answers, and the row asked
    // for meanwhile, { key, command }. A reader is never stopped for the
    // next, since what a stopped one ends with would be taken for the
    // next's answer; the node answers in a moment, so the next waits.
    property var next: null

    // Told whenever what an act shows changes, so its place is said again.
    signal changed

    function explain(key, address) {
        const command = Explaining.command(realm, address);
        explaining = key;
        if (command === null) {
            stands(key, { stage: "unknown" });
            return;
        }
        stands(key, { stage: "reading" });
        if (reader.running) {
            next = { key, command };
            return;
        }
        read(command);
    }

    function read(command) {
        next = null;
        reader.command = command;
        reader.running = true;
        patience.restart();
    }

    function stands(key, how) {
        explained = Object.assign({}, explained, { [key]: how });
        changed();
    }

    function heard(text) {
        patience.stop();
        if (next !== null) {
            read(next.command);
            return;
        }
        if (explaining === "" || explained[explaining]?.stage !== "reading")
            return;
        const cell = Explaining.cellOf(text);
        if (cell === null || asker === null) {
            stands(explaining, { stage: cell === null ? "unknown" : "failed" });
            return;
        }
        const message = Explaining.message(cell, said.asks);
        stands(explaining, { stage: "asking", message: message });
        asker.pose(message);
    }

    // What the act at `key` shows now, or null while it was never asked:
    // its name, the whole answer to read, its note and its hue.
    function shown(key) {
        const one = explained[key] ?? null;
        if (one === null)
            return null;
        const ours = one.stage === "asking" && asker?.posed === one.message;
        if (!ours) {
            return one.stage === "asking" ? null : {
                name: String(said[one.stage] ?? ""),
                reads: "",
                note: "",
                hue: one.stage === "reading" ? "voice" : "alert"
            };
        }
        const whole = String(asker.said ?? "").trim();
        const first = OnSelection.firstSentence(whole);
        return {
            name: whole === "" ? "\u2026" : first,
            reads: first === whole ? "" : whole,
            note: asker.stage === "answered" ? String(said.answered ?? "")
                : asker.stage === "failed" ? String(said.failed ?? "") : String(said.asking ?? ""),
            hue: asker.stage === "failed" ? "alert" : "voice"
        };
    }

    // Ask's answer, as it streams and once it ends, said again while it's
    // ours.
    Connections {
        target: explainer.asker
        ignoreUnknownSignals: true

        function onSaidChanged() {
            explainer.told();
        }

        function onStageChanged() {
            explainer.told();
        }
    }

    function told() {
        const one = explained[explaining] ?? null;
        if (one?.stage === "asking" && asker?.posed === one.message)
            changed();
    }

    Process {
        id: reader

        stdout: StdioCollector {
            onStreamFinished: explainer.heard(text)
        }
    }

    // The node says a cell at once; ten seconds without it is no answer,
    // as is a reader that couldn't start, which ends no stream.
    Timer {
        id: patience

        interval: 10000
        onTriggered: {
            reader.running = false;
            explainer.next = null;
            if (explainer.explained[explainer.explaining]?.stage === "reading")
                explainer.stands(explainer.explaining, { stage: "failed" });
        }
    }
}
