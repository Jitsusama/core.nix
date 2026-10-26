// Whether what Joel uses waited, answered in the field: "lag", Desktop's
// one word for it. lag-watch (modules/lag-watch) writes its last ten
// seconds to a file as each ends; it's read when asked, so nothing is
// watched or spawned. A machine without lag-watch has no file, and so no
// answer.
//
// Joel, 2026-10-25: "computer lag is confusing as well, i don't really
// understand what it is trying to say or what i can do about it." So it
// says, in plain words, what's slowing the desktop and what to do about it:
// the headline is the worst of what waited, or that nothing's slowing it;
// the rows are each thing that waited, in lag-watch's words, then one act
// that goes to what's behind the worst (the processor's or the memory's
// programs, or the disk), then how fast the desktop answers, and how hard
// builds and jobs are running, which goes to the queue. Nothing is named
// by a program's name, and no figure needs knowing what a percentile is.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "Numbers.js" as Numbers

Provider {
    id: provider

    // The word it answers whole, and by its start (Provider.qml).
    questions: ["lag"]

    // How old a window may be and still be the last ten seconds: lag-watch
    // writes one every ten.
    readonly property int fresh: 25

    // Where each of lag-watch's causes is looked into: the answer whose
    // parts name what's using it, typed at the top.
    readonly property var lookedAt: ({
        cpu: "processor",
        memory: "memory",
        disk: "disk"
    })

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

    function said(id, fill) {
        let words = (Words.of("waits").said ?? {})[id] ?? id;
        for (const key of Object.keys(fill ?? {}))
            words = words.replace(`{${key}}`, fill[key]);
        return words;
    }

    // An act that goes to a spot by its word, from the top, the launcher
    // kept open there.
    function goes(key, word, fields) {
        return thing(Object.assign({
            key: key,
            verb: Words.verb(key),
            name: Words.name(key),
            glyph: Words.glyph(key),
            hue: "action",
            stays: true,
            run: () => {
                provider.launcher.held = [];
                provider.launcher.restart(word);
            }
        }, fields));
    }

    function told() {
        const window = parsed(read(status));
        if (!window || !Array.isArray(window.trouble))
            return null;
        const waited = Theme.glyph.lag;
        const fine = Theme.glyph.fine;
        const rows = window.trouble.map(line => copyRow({ name: line, side: "", glyph: waited }));
        const where = lookedAt[window.worst ?? ""];
        if (where !== undefined)
            rows.push(goes(`waits:see:${window.worst}`, where));
        const niri = window.niri ?? {};
        rows.push(copyRow({
            name: Words.name("waits:typical"),
            side: said("answers", { typical: ms(niri.p50_ms ?? 0), worst: ms(niri.p99_ms ?? 0) }),
            note: said("answersNote"),
            glyph: fine
        }));
        rows.push(goes("waits:work", "queue", {
            side: String(window.band ?? ""),
            note: said("workNote")
        }));
        const count = window.trouble.length;
        let headline = said("calm");
        if (count === 1)
            headline = window.trouble[0];
        else if (count > 1)
            headline = said("several", { count: count });
        const age = Date.now() / 1000 - Number(window.ended ?? 0);
        const note = age <= fresh ? said("fresh") : said("stale", { ago: ago(age) });
        return { headline: headline, rows: rows, note: note };
    }

    FileView {
        id: status
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/lag-watch/status.json`
        blockLoading: true
        printErrors: false
    }
}
