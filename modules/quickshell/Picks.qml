// What was picked for what was typed, so the launcher learns: whatever you
// pick for "sl" becomes what "sl" means, with no alias to set up. It is
// LaunchBar's way, and Firefox's address bar's, rather than counting how
// often a thing is opened whatever was typed.
//
// Each pick is kept against the query it answered, as a count that halves
// every halfLife days, so habits that change are followed and old ones
// fade. It lives in a small file in the shell's state folder, written when
// a pick is made, and nothing else is spawned or stored.
import Quickshell
import Quickshell.Io
import QtQuick

Scope {
    id: root

    // Whether it learns at all; off, nothing is kept or leaned on.
    property bool learns: true
    // Days until a pick counts half as much.
    property real halfLife: 14
    // How many queries to remember; the least recently used go first.
    property int remember: 500

    // query -> { key -> { n: the fading count when last picked, c: every
    // pick, t: when, in milliseconds } }
    property var picks: ({})

    readonly property real day: 24 * 60 * 60 * 1000

    function fade(entry, now) {
        return entry.n * Math.pow(0.5, (now - entry.t) / (halfLife * day));
    }

    // A query as it is kept: in lower case, its spaces collapsed.
    function key(typed) {
        return typed.trim().toLowerCase().replace(/\s+/g, " ");
    }

    // How strongly each thing answers what's typed, by what was picked
    // before: fully for picks made for exactly this, and half for picks
    // made for something this starts, since you'd typed more to mean it.
    // Each answer is { weight, times }, times being how often it was picked
    // for exactly this.
    function weights(typed) {
        const query = key(typed);
        const now = Date.now();
        const out = {};
        if (!learns || query === "")
            return out;
        for (const asked in picks) {
            const exact = asked === query;
            if (!exact && !asked.startsWith(query))
                continue;
            const answers = picks[asked];
            for (const thing in answers) {
                const found = out[thing] ?? (out[thing] = { weight: 0, times: 0 });
                found.weight += fade(answers[thing], now) * (exact ? 1 : 0.5);
                if (exact)
                    found.times += answers[thing].c;
            }
        }
        return out;
    }

    // Remember that this thing was picked for what was typed.
    function picked(typed, thing) {
        const query = key(typed);
        if (!learns || query === "" || !thing)
            return;
        const now = Date.now();
        const answers = picks[query] ?? {};
        const entry = answers[thing];
        answers[thing] = entry
            ? { n: fade(entry, now) + 1, c: entry.c + 1, t: now }
            : { n: 1, c: 1, t: now };
        picks[query] = answers;

        // Forget the queries least recently answered once there are too many.
        const queries = Object.keys(picks);
        if (queries.length > remember) {
            const latest = asked => Math.max(...Object.values(picks[asked]).map(e => e.t));
            queries.sort((a, b) => latest(a) - latest(b));
            for (const asked of queries.slice(0, queries.length - remember))
                delete picks[asked];
        }
        file.setText(JSON.stringify(picks));
    }

    FileView {
        id: file
        path: Quickshell.statePath("launcher-picks.json")
        blockLoading: true
        printErrors: false
    }

    Component.onCompleted: {
        try {
            const kept = JSON.parse(file.text() || "{}");
            picks = kept && typeof kept === "object" ? kept : {};
        } catch (e) {
            console.warn(`The launcher's picks couldn't be read, so it starts afresh: ${e}`);
            picks = {};
        }
    }
}
