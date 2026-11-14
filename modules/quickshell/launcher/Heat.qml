// How hot the machine runs, answered in the field by Heat's words
// ("temperature", "heat", "hot", "thermal", "sensors" or "fans", as
// words.json lists them). The headline is the hottest part and the fans;
// the rows are each part by its name first, then what it reads: the
// Processor, how often it's Throttled, its NPU, Image Processor, Media
// Engine and Display Engine, the Disk, the Memory, the Wi-Fi card, the
// Battery, the Air inside and each fan, each holding more a step in.
//
// The machine daemon (modules/machine, heat_rows.rs) makes every row and
// the headline, worded, ordered and graphed, from its sensor, zone,
// cooling, processor and telemetry records, reading them once a second
// only while Heat's rows are watched, and serves them on the ready-row
// form (ReadyRows.qml). Nothing here gathers, sorts or words a part (Joel,
// 2026-11-10: "the daemons need to be the data layer, and they need to
// serve the launcher"): where Heat.qml made every part's rows from the
// records on the shell's thread, each key typed while Heat shows now
// draws the rows the daemon last served, and a change remakes only the
// rows it names.
import qs
import Quickshell
import QtQuick
import "Seen.js" as Seen

Provider {
    id: provider

    examples: ["temperature", "fans"]

    // The words it answers whole, and by their start (Provider.qml).
    questions: Words.answers("heat")

    // A part asked for alone, whole, by its own words ("throttling"), as
    // the vocabulary's id for it. Only whole, so the start of one never
    // takes another answer's start.
    readonly property var partsAsked: {
        const out = {};
        for (const id of ["heat:throttled"])
            for (const question of Words.answers(id))
                out[question] = id;
        return out;
    }

    function answer(typed) {
        const part = partsAsked[plainly(typed)];
        if (!part && asks(typed) === "")
            return null;
        return part ? alone(part) : told();
    }

    // Whether Heat's rows can be seen: its answer, or words typed that
    // ask it or a part, a row of it, or Machine's place holding its row, so
    // the daemon reads the sensors only then.
    readonly property bool seen: Seen.seen(launcher, provider, ["heat"],
        typed => Seen.asks(questions.concat(Object.keys(partsAsked)), plainly(typed)))

    // Whether the daemon hasn't yet answered this watch whole, so a
    // terminal's show asks again for the figures rather than printing
    // what it held before (Provider.stillReading).
    // A row held before its watch began counts too, so the show that
    // begins it waits for what the daemon serves.
    stillReading: provider.seen && ready.state !== "not answering"
        && !(ready.link?.answeredThisWatch ?? false)

    // Heat's rows and headline as the machine daemon serves them, watched
    // only while they can be seen, since the daemon reads Heat's sensors
    // each second while they're watched; its rows are held meanwhile.
    ReadyRows {
        id: ready

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
        place: "heat"
        beside: ["head"]
        live: provider.seen
        maker: provider
        // Each row in Heat's own ink where it doesn't warn.
        make: (row, key) => ready.thing(row, key, { hue: row.hue || "machine" })
        drawn: provider.launcher?.open ?? false
    }
    // What the daemon serves, for a reader told of each change it hears.
    readonly property alias served: ready
    readonly property string headKey: "head:heat"

    // Heat's answer: its headline and its own rows, a step in each holding
    // what's inside it; held in its place until the daemon has served a
    // row (unheld).
    function told() {
        const rows = ready.rows();
        if (rows.length === 0)
            return unheld();
        const head = ready.held[headKey]?.head ?? {};
        return {
            headline: head.said ?? "",
            graph: head.graph,
            glyph: Words.glyph("heat"),
            rows: rows,
            live: provider
        };
    }

    // Heat with no row served: while its daemon is starting or not yet
    // watched, held in its place with the fixed placeholder, as Machine's
    // own are (Machine.qml's unheld), so a terminal's show finds it and
    // its watch begins; and while the daemon isn't answering, saying so.
    // Never gone, since Machine's head names Heat and a show must find
    // the row it names (VKCF3Z, 2026-11-12 [058bd4b0]). It says it has
    // parts, so a place lists it from the start and says it again in
    // place as its rows come.
    function unheld() {
        const down = ready.state === "not answering";
        return {
            headline: down ? "" : "\u2026",
            glyph: Words.glyph("heat"),
            rows: [],
            note: down ? (Words.of("heat").said?.down ?? "") : "",
            asking: !down,
            parted: true,
            live: provider
        };
    }

    // One part as an answer, by its id, "heat:throttled" its row
    // row:heat:throttled: its figure as the headline and its own parts a
    // step in, as Machine's do.
    function alone(id) {
        const key = `row:${id}`;
        const part = ready.served(key) ? ready.rowAt(key) : null;
        if (!part)
            return null;
        return {
            headline: part.side,
            graph: part.graph,
            glyph: Words.glyph("heat"),
            rows: ready.inside(key),
            note: part.note,
            live: provider
        };
    }

    // While one of its answers is shown, what the daemon changes is
    // answered at once, so its figures and sparks move.
    readonly property bool shown: launcher !== null && launcher.open && launcher.held.length === 0
        && launcher.answer?.live === provider

    // Whether its answer is stepped into: held as a step rather than
    // being the launcher's answer, as Machine.qml's are.
    readonly property bool steppedIn: launcher !== null && launcher.open
        && launcher.held.some(step => step.row?.live === provider)

    // Whether its answer is a row listed in the place held last, as
    // Machine's Heat is: said again, only that row is asked again
    // (Launcher.qml's relive), so its headline moves with each change
    // for little. Said only while shown or stepped into, Machine's Heat
    // row kept the heat it first heard for as long as Machine was open.
    readonly property bool listed: launcher !== null && launcher.open
        && launcher.held.length > 0
        && (launcher.held[launcher.held.length - 1].rows ?? []).some(row => row.asker === provider)

    // Whether the daemon has said anything since Heat's rows came into
    // sight: its first rows are answered at once, shown or not, so
    // Machine's Heat row is there as soon as they are; after that, a
    // shown or listed answer moves with each, and one stepped into as its
    // rows come or go, so the processor's own parts are listed once its
    // telemetry first reads, however soon the answer was stepped into.
    // Stepped into, its rows are things that move in place, and saying
    // each change would make the step again each second.
    property bool heardSinceOpen: false
    property string placedAnswered: ""
    onSeenChanged: {
        if (!seen)
            heardSinceOpen = false;
    }

    Connections {
        target: ready

        // Its daemon gone or back, as its hello says it (Daemon.qml):
        // Heat's row says so where it's held. A daemon's state changes
        // only as it connects or drops, never with its figures.
        function onStateChanged() {
            provider.answered();
            provider.launcher?.placesRead?.();
        }

        function onHeard(changed, gone) {
            const placed = ready.keys.join("\n");
            const moved = placed !== provider.placedAnswered;
            provider.placedAnswered = placed;
            if (!provider.heardSinceOpen) {
                provider.heardSinceOpen = true;
                provider.answered();
                // Machine's Heat row is Heat's answer, so the places held
                // are said again with it.
                provider.launcher?.placesRead?.();
            } else if (provider.shown || provider.listed || (moved && provider.steppedIn)) {
                provider.answered();
            }
        }
    }
}
