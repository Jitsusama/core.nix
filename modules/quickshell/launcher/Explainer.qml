// Explain It's machinery, for any row a session is behind: it asks the
// realm's node what the session is (Explaining.js's command), and hands
// what Joel would know it by to a pi of its own on the machine's quick
// model, whose answer then stands in the act's place, its first sentence
// as its name and the whole of it read, as On the Selection's answers do
// (Explaining.js's shown).
//
// Its pi is pi-ask with the launcher's tool left out, never Ask's own: a
// session's title and plan are an agent's words, and Ask's pi can run what
// the launcher offers. It's started the first time a row is explained, and
// kept warm from then on as Ask's is. One session is read at a time: one
// asked while another is read waits for that read to end, and the one it
// replaced is the act again.
import Quickshell
import Quickshell.Io
import QtQuick
import "Explaining.js" as Explaining

Scope {
    id: explainer

    // The realm whose node is asked, the launcher, whose closing stops an
    // answer as it stops Ask's, and the act's words: its question, with
    // {facts}, and what each stage says.
    property string realm: ""
    property var launcher: null
    property var said: ({})
    // How long the node has to answer, in milliseconds: it says a cell at
    // once, so ten seconds without one is no answer.
    property int patience: 10000
    // The pi that answers, its own once a row is first explained; a test
    // gives one standing in.
    property var asker: null

    // The row explained last, by its act's key, and how it stands, as
    // Explaining.js's shown reads it: { stage, message, at }, where stage is
    // "reading" while the node is asked, "asking" once the pi is, and
    // "unknown" or "failed" when the node can't say. Replaced whole, so
    // bindings hear it.
    property string explaining: ""
    property var explained: ({})
    // The words the node is asked with next, while a read before them ends.
    property var next: null

    // Told whenever what an act shows changes, so its place is said again.
    signal changed

    function explain(key, address) {
        const command = Explaining.command(realm, address);
        const before = explaining;
        explaining = key;
        const left = Object.assign({}, explained);
        // A row whose read is overtaken is the act again, never left
        // saying it reads.
        if (before !== key && left[before]?.stage === "reading")
            delete left[before];
        left[key] = { stage: command === null ? "unknown" : "reading", at: Date.now() };
        explained = left;
        changed();
        if (command === null)
            return;
        if (asker === null)
            asker = own();
        if (reader.running || reader.settling) {
            next = command;
            return;
        }
        read(command);
    }

    // Its own Ask, made as the launcher makes each source, with the
    // launcher's tool left out, or null where it can't be made.
    function own() {
        const file = Qt.createComponent("Ask.qml");
        if (file.status !== Component.Ready) {
            console.error(`Explain It's pi couldn't load: ${file.errorString()}`);
            return null;
        }
        return file.createObject(explainer, {
            settings: { name: "explain", command: ["pi-ask", "--exclude-tools", "launcher"] },
            launcher: launcher
        });
    }

    function read(command) {
        reader.heard = {};
        reader.settling = true;
        reader.command = command;
        reader.running = true;
        waiting.interval = patience;
        waiting.restart();
    }

    function stands(key, how) {
        explained = Object.assign({}, explained,
            { [key]: Object.assign({ at: Date.now() }, how) });
        changed();
    }

    // A read is settled once both its streams and its run have ended, in
    // whichever order Quickshell tells them; the next waiting is read then.
    function hear(stream, text) {
        reader.heard = Object.assign({}, reader.heard, { [stream]: text });
        settle();
    }

    function settle() {
        const heard = reader.heard;
        if (!reader.settling || reader.running || heard.out === undefined
            || heard.errors === undefined)
            return;
        waiting.stop();
        if (next !== null) {
            // Read a moment later, never from inside the reader's own
            // telling that it ended; it stays settling until then, so a row
            // asked meanwhile waits its turn too.
            const command = next;
            next = null;
            // What the last read said is let go now, as Quickshell may tell
            // its ending again before the next starts, and it isn't the
            // waiting row's.
            reader.heard = {};
            Qt.callLater(() => explainer.read(command));
            return;
        }
        reader.settling = false;
        if (explained[explaining]?.stage !== "reading")
            return;
        const ended = Explaining.outcome(heard.out, heard.errors);
        if (ended.cell === undefined || asker === null) {
            stands(explaining, { stage: ended.stage ?? "failed" });
            return;
        }
        const message = Explaining.message(ended.cell, said.asks);
        stands(explaining, { stage: "asking", message: message });
        asker.pose(message);
    }

    // What the act at `key` shows now, or null where it's the act as it was.
    function shown(key) {
        return Explaining.shown(explained[key] ?? null, asker, said, Date.now());
    }

    // The pi's answer, a word a moment as Ask tells it, said again while
    // it's this act's.
    Connections {
        target: explainer.asker
        ignoreUnknownSignals: true

        function onAnswered() {
            const one = explainer.explained[explainer.explaining] ?? null;
            if (one?.stage === "asking" && explainer.asker?.posed === one.message)
                explainer.changed();
        }
    }

    Process {
        id: reader

        property var heard: ({})
        property bool settling: false

        stdout: StdioCollector {
            onStreamFinished: explainer.hear("out", text)
        }
        stderr: StdioCollector {
            onStreamFinished: explainer.hear("errors", text)
        }
        onRunningChanged: explainer.settle()
    }

    // A node that never answers is given up on, as is a reader that
    // couldn't start, which ends neither stream.
    Timer {
        id: waiting

        onTriggered: {
            reader.running = false;
            reader.settling = false;
            explainer.next = null;
            if (explainer.explained[explainer.explaining]?.stage === "reading")
                explainer.stands(explainer.explaining, { stage: "failed" });
        }
    }
}
