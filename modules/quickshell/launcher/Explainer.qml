// Explain It's machinery, for any row a session is behind: it asks the
// realm's node what the session is (Explaining.js's command), and hands
// what Joel would know it by to a pi of its own on the machine's quick
// model, whose answer then stands in the act's place, its first sentence
// as its name and the whole of it read, as On the Selection's answers do
// (Explaining.js's shown).
//
// Its pi is pi-ask with no tools at all (`--no-launcher`), never Ask's own:
// a session's title and plan are an agent's words, and Ask's pi can run
// what the launcher offers. It's made the first time a row is explained,
// as the launcher makes each source, and kept warm from then on as Ask's
// is. Each read of the node is a process of its own that knows its row,
// so no answer is ever taken for another row's, and a row asked while
// another is read makes that one the act again, its answer let go when it
// comes.
pragma ComponentBehavior: Bound

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
    // gives one standing in. Its own is pi-ask started with no tools.
    property var asker: null
    readonly property var pi: ["pi-ask", "--no-launcher"]

    // The row explained last, by its act's key, and how each stands, as
    // Explaining.js's shown reads it: { stage, message, at, beside }, where
    // stage is "reading" while the node is asked, "asking" once the pi is,
    // and "unknown" or "failed" when the node can't say, and beside names
    // the acts on its row it was asked with (Explaining.fits). Replaced
    // whole, so bindings hear it.
    property string explaining: ""
    property var explained: ({})

    // Told whenever what an act shows changes, so its place is said again.
    signal changed

    // How often, at most, in milliseconds, a streaming answer says its act again: the pi tells
    // each word as it comes, and each change makes all of Now's rows read as changed, many
    // times a second through an answer (Y2YBTS on #5357). A tenth of a second still reads as
    // the words coming.
    property int pace: 100

    // Explains the session at `address` on `key`'s row, whose acts Joel
    // could answer it by are named `beside` (Explaining.facts), kept with
    // it so its answer goes once they change (Explaining.fits). They're
    // read as Explaining.names reads them, never screened here, as a list
    // Qt hands through a property is no array of JavaScript's.
    function explain(key, address, beside) {
        const command = Explaining.command(realm, address);
        const before = explaining;
        explaining = key;
        const left = Object.assign({}, explained);
        // A row whose read is overtaken is the act again, never left
        // saying it reads.
        if (before !== key && left[before]?.stage === "reading")
            delete left[before];
        left[key] = {
            stage: command === null ? Explaining.unasked(realm) : "reading",
            at: Date.now(),
            beside: beside ?? []
        };
        explained = left;
        changed();
        if (command === null)
            return;
        if (asker === null)
            asker = own();
        reading.createObject(explainer, { key: key, command: command, patience: patience,
            beside: beside ?? [] });
    }

    // Its own Ask, made as the launcher makes each source, or null where it
    // can't be made.
    function own() {
        const file = Qt.createComponent("Ask.qml");
        if (file.status !== Component.Ready) {
            console.error(`Explain It's pi couldn't load: ${file.errorString()}`);
            return null;
        }
        return file.createObject(explainer, {
            settings: { name: "explain", command: pi },
            launcher: launcher
        });
    }

    // How `key`'s row stands now, `how`, its acts kept as explain was given
    // them.
    function stands(key, how) {
        explained = Object.assign({}, explained, { [key]: Object.assign({ at: Date.now(),
            beside: explained[key]?.beside ?? [] }, how) });
        changed();
    }

    // What the node said of the session at `address`, for `key`'s row,
    // `out` and `errors`, or null when it gave no answer in time: it's this
    // row's only while the row is the one explained and still reads.
    // `beside` names the row's acts, as explain was given them, and the
    // act's `acts` words ask the answer to end with one only while it names
    // any (Explaining.message).
    function heard(key, address, out, errors, beside) {
        if (key !== explaining || explained[key]?.stage !== "reading")
            return;
        const ended = out === null ? { stage: "failed" }
            : Explaining.outcome(out, errors, address);
        if (ended.cell === undefined || asker === null) {
            if (ended.stage !== "unknown")
                console.warn(`Explain It: no answer from ${realm}'s node: ${errors ?? "none in time"}`);
            stands(key, { stage: ended.stage ?? "failed" });
            return;
        }
        const message = Explaining.message(ended.cell, said.asks, beside, said.acts);
        stands(key, { stage: "asking", message: message });
        asker.pose(message);
    }

    // What the act at `key` shows now, on a row whose acts are `beside`, or
    // null where it's the act as it was, as it is once they change, since
    // its answer may name one (Explaining.fits).
    function shown(key, beside) {
        const one = explained[key] ?? null;
        return one !== null && !Explaining.fits(one.beside, beside) ? null
            : Explaining.shown(one, asker, said, Date.now());
    }

    // The pi's answer, a word a moment as Ask tells it, said again for
    // whichever row it answers, at most once each `pace`: the words told
    // meanwhile are said together as it ends, whether a row still asks
    // judged then, so the last word is never left unsaid.
    Connections {
        target: explainer.asker
        ignoreUnknownSignals: true

        function onAnswered() {
            if (!pacing.running)
                pacing.start();
        }
    }

    Timer {
        id: pacing

        interval: explainer.pace
        onTriggered: {
            const posed = explainer.asker?.posed;
            if (Object.values(explainer.explained).some(one => one.stage === "asking"
                && one.message === posed))
                explainer.changed();
        }
    }

    // One read of the node, for one row: it says what it heard once both
    // its streams and its run have ended, in whichever order Quickshell
    // tells them, or nothing once it's given up on, and then goes.
    Component {
        id: reading

        Process {
            id: one

            required property string key
            required property int patience
            required property var beside
            property var out: undefined
            property var errors: undefined
            property bool done: false

            function settle() {
                if (one.done || one.running || one.out === undefined
                    || one.errors === undefined)
                    return;
                one.done = true;
                // The address is the fifth word asked (Explaining.command).
                explainer.heard(one.key, one.command[4], one.out, one.errors, one.beside);
                one.destroy();
            }

            running: true
            stdout: StdioCollector {
                onStreamFinished: {
                    one.out = text;
                    one.settle();
                }
            }
            stderr: StdioCollector {
                onStreamFinished: {
                    one.errors = text;
                    one.settle();
                }
            }
            onRunningChanged: one.settle()

            // A node that never answers is given up on, as is a read that
            // couldn't start, which ends neither stream.
            property Timer waiting: Timer {
                interval: one.patience
                running: true
                onTriggered: {
                    if (one.done)
                        return;
                    one.done = true;
                    one.running = false;
                    explainer.heard(one.key, "", null, null);
                    one.destroy();
                }
            }
        }
    }
}
