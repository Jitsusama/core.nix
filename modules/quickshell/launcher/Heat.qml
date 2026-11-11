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
    stillReading: ready.link !== null && !ready.link.answeredThisWatch
        && ready.state !== "not answering"

    // Heat's rows and headline as the machine daemon serves them, watched
    // only while they can be seen.
    ReadyRows {
        id: ready

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
        place: "heat"
        beside: ["head"]
        active: provider.seen && !!Quickshell.env("XDG_RUNTIME_DIR")
        maker: provider
        make: (row, key) => ready.thing(row, key, provider.drawnAs(row))
        drawn: provider.launcher?.open ?? false
    }
    // What the daemon serves, for a reader told of each change it hears.
    readonly property alias served: ready
    readonly property string headKey: "head:heat"

    // What a row is drawn with beyond what every place's are: Heat's own
    // ink where the row doesn't warn. The rest is a stopgap until
    // ReadyRows.qml's thing carries it for every place (Y2YBTS, #3136, by
    // 2026-11-11 05:30): the row's graph, its side's items with what each
    // keeps and warns in, and a row whose Return copies made as
    // Provider.copyRow makes one, closing once it has.
    function drawnAs(row) {
        const fields = {
            hue: row.hue || "machine",
            graph: row.graph,
            sideItems: row.side ?? []
        };
        const copy = row.acts?.[0]?.copy;
        if (typeof copy === "string")
            Object.assign(fields, {
                figure: true,
                plain: true,
                now: true,
                stays: false,
                copies: () => copy
            });
        return fields;
    }

    // Heat's answer: its headline and its own rows, a step in each holding
    // what's inside it; none until the daemon has served a row.
    function told() {
        const rows = ready.rows();
        if (rows.length === 0)
            return null;
        const head = ready.held[headKey]?.head ?? {};
        return {
            headline: head.said ?? "",
            graph: head.graph,
            glyph: Words.glyph("heat"),
            rows: rows,
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

    // Whether the daemon has said anything since Heat's rows came into
    // sight: its first rows are answered at once, shown or not, so
    // Machine's Heat row is there as soon as they are; after that, only a
    // shown answer moves with each.
    property bool heardSinceOpen: false
    onSeenChanged: {
        if (!seen)
            heardSinceOpen = false;
    }

    Connections {
        target: ready

        function onHeard(changed, gone) {
            if (!provider.heardSinceOpen) {
                provider.heardSinceOpen = true;
                provider.answered();
                // Machine's Heat row is Heat's answer, so the places held
                // are said again with it.
                provider.launcher?.placesRead?.();
            } else if (provider.shown) {
                provider.answered();
            }
        }
    }
}
