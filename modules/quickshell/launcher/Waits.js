// What Lag says beside its name, worked out apart from Waits.qml so
// tests/answers/waits.js holds it in Qt's own engine. Joel asked Machine
// why the laptop was slow and gave up after ten seconds with Lag's name
// alone (VKCF3Z, 2026-11-12), so while anything waited it says the cause
// in his words and the program most behind it, which the machine daemon
// adds to lag-watch's window as `top` (using.rs): "slowed by the
// processor · cargo · 11 of 16 cores".
.pragma library
.import "Numbers.js" as Numbers

// Bytes in SI units, to three digits: "6.24 GB".
const UNITS = ["B", "kB", "MB", "GB", "TB"];
const KILO = 1000;

// Lag's words, by id, with each {name} filled, or "" where they say none.
function filled(words, id, fill) {
    let said = (words ?? {})[id];
    if (typeof said !== "string")
        return "";
    for (const key of Object.keys(fill ?? {}))
        said = said.replace(`{${key}}`, fill[key]);
    return said;
}

function sized(bytes, settings) {
    let scaled = Number(bytes);
    let unit = 0;
    while (scaled >= KILO && unit < UNITS.length - 1) {
        scaled /= KILO;
        unit += 1;
    }
    return `${Numbers.significant(scaled, settings?.group, settings?.point)} ${UNITS[unit]}`;
}

// The program behind a wait and its share, as the daemon told it: the
// processor's in cores of all it has, at least one, since the program
// most behind a wait used some; memory's by what it holds.
function behind(top, words, settings) {
    if (!top || !top.name)
        return "";
    if (top.of === "cpu" && Number(top.cores) > 0) {
        const all = Number(top.cores);
        const used = Math.min(all, Math.max(1, Math.round(Number(top.share_ratio) * all)));
        return filled(words, "cores", { name: top.name, used: used, all: all });
    }
    if (top.of === "memory" && Number(top.resident_b) > 0)
        return filled(words, "holds", { name: top.name, held: sized(top.resident_b, settings) });
    return "";
}

// Lag's headline, its side in Machine: that nothing's slowing the
// desktop; or what is, by its cause, then the program behind it; or, for a
// cause it has no words for, lag-watch's own sentence, or how many.
function headline(window, words, settings) {
    const trouble = Array.isArray(window?.trouble) ? window.trouble : [];
    if (trouble.length === 0)
        return filled(words, "calm");
    return trouble.length === 1 ? trouble[0]
        : filled(words, "several", { count: trouble.length });
}
