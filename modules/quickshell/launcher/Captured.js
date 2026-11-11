// What Capture's Recent says of each thing taken (launcher/Capture.qml): the
// mark its row wears, a picture's or a video's as files wear them, and the
// drawer under it, where it is and what kind of file above, the picture
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
const PICTURE = /\.(png|jpe?g|webp|avif|gif)$/i;
const VIDEO = /\.(mp4|mkv|webm|mov)$/i;

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
// and nothing today: "04:39", "yesterday 23:05", "Thursday 12:00", "28
// Oct 08:01", one moment read one way everywhere, with nothing between
// its day and its time (VKCF3Z, 2026-11-08). `days` gives the launcher's
// word for yesterday and `date(date, format)`, Qt's formatDate in the
// shell; a thing taken is never tomorrow, so it needs no word for that.
function taken(at, now, days) {
    const moment = new Date(at);
    const time = `${twoFigures(moment.getHours())}:${twoFigures(moment.getMinutes())}`;
    const day = When.day(at, now, days);
    return day === "" ? time : `${day.replace(/ /g, KEPT)} ${time}`;
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

// What file-preview said of one thing, from the line it answered with:
// null where it couldn't say, or said anything but an object of facts.
function shownFrom(line) {
    try {
        const said = JSON.parse(line);
        return typeof said === "object" && said !== null && !Array.isArray(said) && !said.error
            ? said : null;
    } catch (_malformed) {
        return null;
    }
}

// The drawer for one thing taken: `file` its { path, name, where, at, size },
// where its folder from ~; `shown` what file-preview said of it, or null;
// and `told` the moment it's drawn (now), the label of each fact by its id
// (named), the vocabulary's words (said) and how days are said (days,
// FileFacts.js's). Its eyebrow is the one Recent's pictures wear
// (Recent.takenEyebrow), where, then the kind of file it is, a picture or
// a video, as the folder already says what took it: "in
// ~/Pictures/Screenshots \u00b7 picture" (VKCF3Z, 2026-11-08). Short of
// room the kind gives way first, then the folder from its middle.
function drawer(file, shown, told) {
    const { now, named, said, days } = told;
    const kind = said[PICTURE.test(file.path) ? "picture" : VIDEO.test(file.path) ? "video"
        : ""] ?? "";
    return Object.assign(Recent.takenEyebrow(file.where, kind, said), {
        title: file.name,
        preview: preview(file, shown),
        facts: facts(file, shown, now, days)
            .map(fact => ({ label: named(fact.id), value: fact.value }))
    });
}
