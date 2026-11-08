// What Capture's Recent says of each thing taken (launcher/Capture.qml): the
// mark its row wears, a picture's or a video's as files wear them, and the
// drawer under it, where it is and what kind of capture above, the picture
// itself, then when it was taken, how big it is, and a picture's dimensions
// or a recording's length (VKCF3Z, 2026-11-08). A fact nothing has said yet
// is left out rather than said as unknown. Plain JavaScript, held to it by
// tests/captured.qml in Qt's own engine.
.pragma library
.import "../FileFacts.js" as FileFacts
.import "../When.js" as When

// A number keeps to its unit, and a dimension's two sides to the ×.
const KEPT = "\u00a0";
const PICTURE = /\.(png|jpe?g|webp|avif|gif)$/i;
const VIDEO = /\.(mp4|mkv|webm|mov)$/i;
const FIELD = /\{(\w+)\}/g;
const YESTERDAY = "\u0000yesterday";

// "{n}" in a sentence the vocabulary gives, filled.
function filled(template, fields) {
    return String(template ?? "").replace(FIELD, (whole, name) => String(fields[name] ?? whole));
}

function twoFigures(number) {
    return String(number).padStart(2, "0");
}

// The theme's mark for what it is, by its ending: a picture's, a video's,
// or "" for anything else, which keeps a file's own.
function mark(path) {
    if (PICTURE.test(path)) {
        return "picture";
    }
    return VIDEO.test(path) ? "film" : "";
}

// When it was taken, to the second, since two screenshots a moment apart
// are told apart by it: "04:39:15 today", "04:39:15 yesterday", "04:39:15
// on Thursday", "04:39:15 on 28 Oct". `said` holds the vocabulary's words
// and `date(date, format)`, Qt's formatDate in the shell.
function taken(at, now, said) {
    const moment = new Date(at);
    const time = [moment.getHours(), moment.getMinutes(), moment.getSeconds()]
        .map(twoFigures).join(":");
    // Yesterday is a sentence of its own, so When.js is asked for it by a
    // word no day is named.
    const day = When.day(at, now, { yesterday: YESTERDAY, date: said.date });
    if (day === "") {
        return filled(said.today, { time });
    }
    return filled(day === YESTERDAY ? said.yesterday : said.on, { time, day });
}

// Its facts, each { id, value }, in order: when it was taken, its size,
// then a picture's dimensions, or a recording's length, once file-preview
// has said them (`shown`, null until then).
function facts(file, shown, now, said) {
    const size = shown?.size ?? file.size;
    const own = shown?.picture === file.path && shown?.across > 0;
    return [
        { id: "taken", value: typeof file.at === "number" ? taken(file.at, now, said) : "" },
        { id: "size", value: typeof size === "number" ? FileFacts.weighed(size) : "" },
        {
            id: "dimensions",
            value: own ? `${shown.across}${KEPT}\u00d7${KEPT}${shown.down}` : ""
        },
        {
            id: "length",
            value: typeof shown?.seconds === "number" ? FileFacts.length(shown.seconds) : ""
        }
    ].filter(fact => fact.value !== "");
}

// What the drawer previews: file-preview's picture of it, a recording's
// frame or the picture itself, or, before it's said, a picture as it is.
function preview(file, shown) {
    if (shown?.picture) {
        return { picture: shown.picture, across: shown.across ?? 0, down: shown.down ?? 0 };
    }
    return PICTURE.test(file.path) ? { picture: file.path, across: 0, down: 0 } : null;
}

// The drawer for one thing taken: `file` its { path, name, where, kind, at,
// size }, where its folder from ~ and kind what Capture calls what lands
// there; `shown` what file-preview said of it, or null; and `told` the
// moment it's drawn (now), the label of each fact by its id (named) and
// the vocabulary's words (said).
function drawer(file, shown, told) {
    const { now, named, said } = told;
    const kind = said.kinds?.[file.kind] ?? "";
    return {
        eyebrow: filled(kind === "" ? said.within : said.eyebrow, { where: file.where, kind }),
        title: file.name,
        preview: preview(file, shown),
        facts: facts(file, shown, now, said)
            .map(fact => ({ label: named(fact.id), value: fact.value }))
    };
}
