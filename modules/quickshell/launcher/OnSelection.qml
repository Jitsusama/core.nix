// On the Selection, on Super+X: what Joel has selected in any window, as
// the place's head, and what an agent can do with it, as its rows (VKCF3Z,
// e684d7ae). The rows are pi-verbs' records as it serves them, in its
// order and its words, with Ask last for a question of his own: the
// daemon words them, and nothing here sorts or names one.
//
// Both daemons are reached over their sockets only while the place is
// open, and let go as it closes. pi-verbs rests after five minutes with
// nothing connected, so it's never held awake (harness #1436). Its last
// list is kept, in the shell and in a file written only when it changes,
// so the place draws at once from it, even the first time after a start
// while pi-verbs is still waking (WCJQSG, c0f83eb9). The selected text
// comes from the clipboard daemon as the place opens, read from the
// selection then and never kept, here or there (93XPJV, 2df16224).
//
// Return on a row asks pi-verbs for its prompt, then Ask's warm pi for
// the answer, which is written in place of the row's name, its whole in
// the drawer. One that replaces the selection is copied as it's
// answered, since nothing types into his window, and Return copies any
// answer. Handing it to an agent comes with the talk line's own form.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "OnSelection.js" as OnSelection

Provider {
    id: provider
    what: "action"
    collection: Words.collection("selection", { searched: false })

    readonly property var said: Words.of("selection").said ?? ({})

    // Whether the place is open now: the launcher shown, standing in it.
    readonly property bool here: (launcher?.open ?? false)
        && (launcher?.held ?? []).some(step => step.row.from === provider)

    // The selected text, "" with none, and whether the daemon has said.
    property string selected: ""
    property bool selectionSaid: false
    // The head's longest, in characters, so the place's path keeps its room.
    readonly property int headMost: 60
    heading: !selectionSaid ? "" : selected === "" ? said.nothing ?? ""
        : OnSelection.headOf(selected, headMost)

    onHereChanged: {
        if (here)
            return;
        selected = "";
        selectionSaid = false;
        asked = {};
    }

    Daemon {
        id: verbs
        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/pi-verbs.sock`
        kind: "verb"
        connects: provider.here
        // Told after what's held takes the line, so the order and the
        // records it lists are read together.
        onTold: answer => {
            if (answer?.id === "all" && Array.isArray(answer.records))
                provider.order = answer.records.map(record => record?.key);
            if (answer?.records !== undefined || answer?.changed !== undefined)
                provider.took();
        }
    }

    Daemon {
        id: clipboard
        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/clipboard-history.sock`
        kind: "selection"
        live: false
        connects: provider.here
        onWireChanged: {
            if (wire !== null)
                provider.readSelection();
        }
    }

    // Asked as the place opens, its answer given only to this and never
    // held, so the text goes once the place closes.
    function readSelection() {
        clipboard.find({ ask: "get", keys: ["selection:primary"] }, answer => {
            if (!here)
                return;
            const record = (answer?.records ?? []).find(one => one?.key === "selection:primary");
            selected = String(record?.text ?? "");
            selectionSaid = true;
            changed();
        });
    }

    // The records' keys as the daemon's whole answer listed them.
    property var order: []
    // The last list, as served, and as kept for a start.
    property var listed: OnSelection.served(order, verbs.held)
    property var kept: []

    function took() {
        listed = OnSelection.served(order, verbs.held);
        keep();
        changed();
    }

    // What each row's Return asked, by its key: the message Ask was given
    // and the record's result, so its answer is found as Ask gives it.
    property var asked: ({})
    readonly property var ask: (launcher?.sources ?? []).find(one => one.settings.name === "ask")
        ?? null

    Connections {
        target: provider.ask
        ignoreUnknownSignals: true
        function onAnswered() {
            provider.answerHeard();
        }
    }

    // An answer that replaces the selection is copied once it's whole.
    function answerHeard() {
        for (const key of Object.keys(asked)) {
            const one = asked[key];
            if (ask?.posed === one.message && ask.stage === "answered" && one.result === "replace"
                && !one.copied) {
                one.copied = true;
                Quickshell.clipboardText = ask.said.trim();
                verbs.tell({ id: `told-${key}`, ask: "told", use: one.use, outcome: "accepted" });
            }
        }
        changed();
    }

    function act(record) {
        if (selected === "")
            return;
        verbs.ask({ ask: "verb", verb: record.id, args: {}, selection: selected }, answer => {
            if (!here || answer === null)
                return;
            if (answer.ok !== true) {
                asked[record.key] = { refused: String(answer.refused ?? "") };
                changed();
                return;
            }
            const message = `${answer.system ?? ""}\n\n${answer.prompt ?? ""}`.trim();
            asked[record.key] = { message, result: record.result, use: answer.use, copied: false };
            ask?.pose(message);
            changed();
        });
    }

    function asking(question) {
        const message = (said.asks ?? "{question}\n\n{selection}").replace("{question}", question)
            .replace("{selection}", selected);
        asked["verb:selection:ask"] = { message, result: "say", use: "", copied: false };
        ask?.pose(message);
        changed();
    }

    // A record's row: its served words, or, once asked, its answer.
    function rowOf(record) {
        const one = asked[record.key] ?? null;
        const answering = one !== null && one.message !== undefined && ask?.posed === one.message;
        return answering ? answered(record.key, one) : member({
            key: record.key,
            name: String(record.words ?? record.id ?? ""),
            note: one?.refused ? one.refused : String(record.says ?? ""),
            glyph: Words.glyph("verb:selection:act"),
            hue: one?.refused ? "alert" : "action",
            verb: Words.verb("verb:selection:act"),
            stays: true,
            changes: false,
            run: () => provider.act(record)
        });
    }

    // An answer in its row's place: its first sentence as the name, the
    // whole in the drawer, and Return copying it.
    function answered(key, one) {
        const whole = String(ask.said ?? "").trim();
        const first = OnSelection.firstSentence(whole);
        const done = ask.stage === "answered";
        return member({
            key: key,
            name: whole === "" ? "\u2026" : first,
            reads: first === whole ? "" : whole,
            note: OnSelection.noted(ask.stage, one.result, said),
            glyph: Words.glyph("verb:selection:act"),
            hue: ask.stage === "failed" ? "alert" : "voice",
            verb: done ? "copy" : Words.verb("verb:selection:act"),
            stays: !done,
            changes: false,
            run: () => {
                if (!done)
                    return;
                Quickshell.clipboardText = whole;
                if (one.use)
                    verbs.tell({ id: `told-${key}`, ask: "told", use: one.use,
                        outcome: "accepted" });
            }
        });
    }

    // Ask, last: what's typed asked about the selection.
    function askRow(typed) {
        const one = asked["verb:selection:ask"] ?? null;
        if (one !== null && ask?.posed === one.message && typed === "")
            return answered("verb:selection:ask", one);
        return member({
            key: "verb:selection:ask",
            name: Words.name("verb:selection:ask"),
            note: typed === "" ? said.typing ?? "" : typed,
            glyph: Words.glyph("verb:selection:ask"),
            hue: "action",
            verb: Words.verb("verb:selection:ask"),
            stays: true,
            changes: false,
            run: () => {
                if (typed !== "")
                    provider.asking(typed);
            }
        });
    }

    // Handing it to an agent asks a note in the talk line's form, which
    // comes after it, so until then a record handed on isn't listed
    // rather than asked of Ask as if it answered.
    readonly property var records: (listed.length > 0 ? listed : kept)
        .filter(record => record?.result !== "hand")

    function everything() {
        return records.map(record => rowOf(record)).concat([askRow("")]);
    }

    function search(typed) {
        const words = Fuzzy.words(typed);
        const rows = records.map(record => rowOf(record))
            .filter(row => Fuzzy.match(row.prepared ?? Fuzzy.prepare(row.name), words, false)
                .score >= 0);
        return { rows: rows.concat([askRow(typed.trim())]), grade: 0 };
    }

    // The last list, written only when it differs from what's kept.
    function keep() {
        const text = JSON.stringify(listed);
        if (listed.length === 0 || text === file.text())
            return;
        kept = listed;
        file.setText(text);
    }

    FileView {
        id: file
        path: Quickshell.statePath("launcher-selection-actions.json")
        blockLoading: true
        printErrors: false
    }

    Component.onCompleted: {
        try {
            const read = JSON.parse(file.text() || "[]");
            kept = Array.isArray(read) ? read : [];
        } catch (error) {
            kept = [];
        }
    }

    // Super+X again puts it away, with nothing asked lost but the answer.
    function pressedOver(inside) {
        if (!inside)
            return false;
        launcher.open = false;
        return true;
    }
}
