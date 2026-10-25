// Whether what Joel uses waited, answered in the field: "lag", "laggy",
// "slow", "jank", "stutter" or "waits". lag-watch (modules/lag-watch) writes
// its last ten seconds to a file as each ends; it's read when asked, so
// nothing is watched or spawned. The headline is what waited, or that
// nothing did; the rows are each thing that waited, in lag-watch's words,
// then how fast the desktop (niri) answered and how hard the work behind
// was going, each named first and never by the program's name. A machine
// without lag-watch has no file, and so no answer.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "Numbers.js" as Numbers

Provider {
    id: provider

    // The words it answers whole, and by their start (Provider.qml). Each
    // start of three letters is one nothing else is typed by: "sta" and
    // "res" start Joel's own words for other things.
    questions: ["lag", "laggy", "lagging", "slow", "jank", "janky", "stutter", "waits",
        "why slow", "is it slow"]

    // How old a window may be and still be the last ten seconds: lag-watch
    // writes one every ten.
    readonly property int fresh: 25

    function answer(typed) {
        if (asks(typed) === "")
            return null;
        return told();
    }

    function ms(value) {
        return `${Numbers.format(value, value < 10 ? 1 : 0, settings.group, settings.point)} ms`;
    }

    // How long ago, said as a person would.
    function ago(seconds) {
        if (seconds < 120)
            return `${Math.round(seconds)} s ago`;
        if (seconds < 7200)
            return `${Math.round(seconds / 60)} min ago`;
        return `${Math.round(seconds / 3600)} h ago`;
    }

    function told() {
        const window = parsed(read(status));
        if (!window || !Array.isArray(window.trouble))
            return null;
        const waited = Theme.glyph.lag;
        const fine = Theme.glyph.fine;
        const trouble = window.trouble.map(line => copyRow({ name: line, side: "", glyph: waited }));
        const niri = window.niri ?? {};
        const part = (id, side, note) => copyRow({ name: Words.name(`waits:${id}`), side: side,
            note: note, glyph: fine });
        const rows = trouble.concat([
            part("typical", ms(niri.p50_ms ?? 0), "as a rule"),
            part("slowest", ms(niri.p99_ms ?? 0), "the slowest 1 in 100"),
            part("work", String(window.band ?? ""), "how hard jobs and builds ran")
        ]);
        const count = window.trouble.length;
        let headline = "nothing waited";
        if (count === 1)
            headline = window.trouble[0];
        else if (count > 1)
            headline = `${count} things waited`;
        const age = Date.now() / 1000 - Number(window.ended ?? 0);
        const note = age <= fresh ? "the last ten seconds"
            : `ten seconds that ended ${ago(age)}`;
        return { headline: headline, rows: rows, note: note };
    }

    FileView {
        id: status
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/lag-watch/status.json`
        blockLoading: true
        printErrors: false
    }
}
