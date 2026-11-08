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
.import "Recent.js" as Recent

// A number keeps to its unit, and a dimension's two sides to the ×.
const KEPT = "\u00a0";
const BETWEEN = " \u00b7 ";
const PICTURE = /\.(png|jpe?g|webp|avif|gif)$/i;
const VIDEO = /\.(mp4|mkv|webm|mov)$/i;
// What Capture calls what lands in each of its folders, each a word the
// vocabulary says.
const KINDS = ["screenshot", "recording", "photo", "video"];

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

// When it was taken, to the minute, its day as When.js says every day
// and nothing today: "04:39", "yesterday \u00b7 23:05", "Thursday \u00b7
// 12:00", "28 Oct \u00b7 08:01" (VKCF3Z, 2026-11-08). `days` gives the
// launcher's word for yesterday and `date(date, format)`, Qt's formatDate
// in the shell.
function taken(at, now, days) {
    const moment = new Date(at);
    const time = `${twoFigures(moment.getHours())}:${twoFigures(moment.getMinutes())}`;
    const day = When.day(at, now, days);
    return day === "" ? time : `${day.replace(/ /g, KEPT)}${BETWEEN}${time}`;
}

// Its facts, each { id, value }, in order: when it was taken, its size,
// then a picture's dimensions, or a recording's length, once file-preview
// has said them (`shown`, null until then).
function facts(file, shown, now, days) {
    const size = shown?.size ?? file.size;
    const own = shown?.picture === file.path && shown?.across > 0;
    return [
        { id: "taken", value: typeof file.at === "number" ? taken(file.at, now, days) : "" },
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
// moment it's drawn (now), the label of each fact by its id (named), the
// vocabulary's words (said) and how days are said (days, FileFacts.js's).
// Its eyebrow is the one Recent's pictures wear (Recent.takenEyebrow),
// giving way from the folder's middle, never two of them.
function drawer(file, shown, told) {
    const { now, named, said, days } = told;
    const kind = KINDS.includes(file.kind) ? said[file.kind] ?? "" : "";
    return Object.assign(Recent.takenEyebrow(file.where, kind, said), {
        title: file.name,
        preview: preview(file, shown),
        facts: facts(file, shown, now, days)
            .map(fact => ({ label: named(fact.id), value: fact.value }))
    });
}
