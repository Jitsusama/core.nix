pragma Singleton
// What spend-watch (modules/spend-watch) tells the launcher: its
// collection, the collection on Super+U and its answers (the launcher's
// Spend) and what each pi session spent today (its Agents); and its
// budgets.json, how each budget stands, every fact of it in words for a
// budget's drawer (SpendDrawer.js), and which budget each session drew on
// last. Every figure and word is the daemon's, so this only holds them.
//
// The collection comes over the daemon's socket as records
// (SpendWatch.js), each kind watched only while it's drawn, and after
// each pass only the records it changed (its src/interface.rs). Each
// property here is assigned only when a record it's made from changed, so
// a pass that moves one session's figure wakes Agents' sessions and
// nothing of Spend's. The whole file was read every 37 s before, about
// 228 KB, and each of Spend's rows made again while it was shut
// (2026-11-10). Agents' figures and today's are watched always, the
// collection's rows while the launcher is open, and the reports and
// their words while Spend's rows are wanted.
//
// While the socket isn't there, as before the daemon starts or in a check
// that writes the file by hand, the file is read instead, when the watch
// on its folder hears it replaced (nixos.nix's patch), never polled. It
// is cut into the same records, so what reads them can't tell which told
// them. A file from a daemon of another shape draws nothing rather than
// something wrong.
import Quickshell
import Quickshell.Io
import QtQuick
import "SpendWatch.js" as Records

Singleton {
    id: root

    // Every record held, by key: plain, so holding one notifies nothing.
    readonly property var held: ({})
    // Whether a collection is held, so Spend has one to show.
    property bool known: false

    // What each pi session spent today, by its whole id.
    property var sessions: ({})
    // How each one's day went, by the same id: { graph, rebuilt }, its
    // hours as Graph's spark, carrying no dollars, and what rebuilding its
    // context cost, "" under a cent.
    property var sessionsToday: ({})
    // What each one's councils, judges and fleets cost today, costliest
    // first: [{ on, spent }].
    property var helpers: ({})
    // What each quest spent today with every quest under it, by its id:
    // { all, own, under }, so an orchestrator's row says its whole
    // orchestration's without the shell adding anything up.
    property var families: ({})
    // How today stands, "$412 today", and what every session together
    // spent today, each "" until it's said.
    property string today: ""
    property string agentsToday: ""
    // The collection's rows, each list in the daemon's order: { now,
    // findings, more, quiet }, each row { key, line }, its record's key and
    // the row as the daemon wrote it.
    property var rows: ({ now: [], findings: [], more: [], quiet: [] })
    // Each report by its id, and the words that name a period, a cut, and
    // the examples the field offers.
    property var reports: ({})
    property var periods: []
    property var cuts: []
    property var examples: []

    // Said once records change, before any property made from them is, so
    // what keeps something made of a record lets it go first: the keys
    // that changed and those that went.
    signal heard(var changed, var gone)

    // What the launcher sets, so only what it draws is watched: whether
    // it's open, whether Spend's rows are wanted, and itself, so a watch
    // begun as it opens waits for its frame (Daemon.qml's paced). With no
    // launcher, as in a check, everything is watched.
    property bool listing: true
    property bool reporting: true
    property var paced: null

    // Whether the socket isn't answering, so the file is read instead.
    readonly property bool fromFile: spent.state === "not answering"
    // Whether the rows, or the reports, are still being read for this
    // opening, drawn from what was held meanwhile (Provider.stillReading).
    readonly property bool rowsReading: !fromFile && listed.live && !listed.answeredThisWatch
    readonly property bool reportsReading: !fromFile && reported.live
        && !reported.answeredThisWatch

    // Takes records said anew and keys let go, wherever from. One that
    // says what the one held says is left as it was, so a watch told whole
    // again, as at each opening, changes nothing it didn't change.
    function took(records, gone) {
        const touched = {};
        const changed = [];
        const dropped = [];
        for (const record of records) {
            if (typeof record?.key !== "string")
                continue;
            if (record.key in held && JSON.stringify(held[record.key]) === JSON.stringify(record))
                continue;
            for (const property of Records.touched(held[record.key] ?? null, record))
                touched[property] = true;
            held[record.key] = record;
            changed.push(record.key);
        }
        for (const key of gone) {
            if (!(key in held))
                continue;
            for (const property of Records.touched(held[key], null))
                touched[property] = true;
            delete held[key];
            dropped.push(key);
        }
        if (changed.length === 0 && dropped.length === 0)
            return;
        heard(changed, dropped);
        for (const property in touched)
            root[property] = Records.made(held, property);
        known = Object.keys(held).length > 0;
    }

    // What a daemon said, taken with every key of its kinds it doesn't
    // hold let go, as one held from the file is once the socket answers.
    function heardFrom(daemon, changed, gone) {
        const kinds = [].concat(daemon.kind);
        const unheld = Object.keys(held).filter(key => kinds.includes(Records.kindOf(key))
            && !(key in daemon.held));
        took(changed.map(key => daemon.held[key]).filter(record => record !== undefined),
            gone.concat(unheld));
    }

    // A collection as read from the file, or null for none.
    function readCollection(collection) {
        const now = collection === null ? {} : Records.records(collection);
        took(Object.values(now), Object.keys(held).filter(key => !(key in now)));
    }

    // budgets.json as read, or null.
    property var standing: null
    // Each budget, nearest its allowance first, by its name, as `spend-watch
    // budgets --json` says it.
    readonly property var budgets: {
        const named = {};
        for (const budget of standing?.budgets ?? [])
            named[budget.name] = budget;
        return named;
    }
    // Which budget each session that spent today drew on last, by its
    // whole id.
    readonly property var budgetOf: standing?.sessions ?? {}

    function parsed(text, version) {
        try {
            const value = JSON.parse(text);
            return typeof value === "object" && value !== null && value.v === version
                ? value : null;
        } catch (error) {
            return null;
        }
    }

    readonly property string socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/spend-watch/socket`

    // Agents' figures and today's, watched always.
    Daemon {
        id: spent

        socket: root.socket
        kind: ["session", "quest", "day"]
        answersWhole: true
        onHeard: (changed, gone) => root.heardFrom(spent, changed, gone)
    }

    // The collection's rows, while the launcher is open.
    Daemon {
        id: listed

        socket: root.socket
        kind: "row"
        live: root.listing
        paced: root.paced
        answersWhole: true
        onHeard: (changed, gone) => root.heardFrom(listed, changed, gone)
    }

    // The reports and their words, while Spend's rows are wanted.
    Daemon {
        id: reported

        socket: root.socket
        kind: ["report", "words"]
        live: root.reporting
        paced: root.paced
        answersWhole: true
        onHeard: (changed, gone) => root.heardFrom(reported, changed, gone)
    }

    onFromFileChanged: {
        if (fromFile)
            readFile();
    }

    // The file read again. A FileView that doesn't preload only forgets
    // its text on reload, and reads once that text is asked for, so it's
    // asked for here, its answer coming as loaded or loadFailed.
    function readFile() {
        file.reload();
        file.text();
    }

    FileView {
        id: file
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/spend-watch/collection.json`
        // Watched always, which costs nothing, since a watch begun once
        // the socket fails never hears the file made; read only then.
        watchChanges: true
        preload: false
        printErrors: false
        onFileChanged: {
            if (root.fromFile)
                root.readFile();
        }
        // What was read last, so a reading that changed nothing says nothing.
        property string read: ""
        onLoaded: {
            if (!root.fromFile || file.text() === file.read)
                return;
            file.read = file.text();
            root.readCollection(root.parsed(file.read, 2));
        }
        onLoadFailed: {
            file.read = "";
            if (root.fromFile)
                root.readCollection(null);
        }
    }

    FileView {
        id: budgetsFile
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/spend-watch/budgets.json`
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        property string read: ""
        onLoaded: {
            if (budgetsFile.text() === budgetsFile.read)
                return;
            budgetsFile.read = budgetsFile.text();
            root.standing = root.parsed(budgetsFile.read, 1);
        }
        onLoadFailed: {
            budgetsFile.read = "";
            root.standing = null;
        }
    }
}
