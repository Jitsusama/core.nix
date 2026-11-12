// What Capture's Recent says of each thing taken (launcher/Capture.qml): the
// mark its row wears, a picture's or a video's as files wear them, and the
// drawer under it, where it is and what kind of file above, the picture
// itself, then when it was taken, how big it is, and a picture's dimensions
// or a recording's length (VKCF3Z, 2026-11-08); and the drawer under each
// of its acts (`act`). A fact nothing has said yet
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

// The drawer for one thing taken: `file` its { path, name, where, at, size },
// where its folder from ~; `shown` what file-preview said of it, or null;
// and `told` the moment it's drawn (now), the label of each fact by its id
// (named), the vocabulary's words (said) and how days are said (days,
// FileFacts.js's). Its eyebrow is the one Recent's pictures wear
// (Recent.takenEyebrow), where, then the kind of file it is, a picture or
// a video, as the folder already says what took it: "in
// ~/Pictures/Screenshots \u00b7 picture" (VKCF3Z, 2026-11-08). Short of
// room the kind gives way first, then the folder from its middle. Where
// file-preview isn't answering (told.silent), the fact it would have said
// says so, rather than being left out as one not said yet.
function drawer(file, shown, told) {
    const { now, named, said, days } = told;
    const kind = said[PICTURE.test(file.path) ? "picture" : VIDEO.test(file.path) ? "video"
        : ""] ?? "";
    return Object.assign(Recent.takenEyebrow(file.where, kind, said), {
        title: file.name,
        preview: preview(file, shown),
        facts: facts(file, shown, now, days).concat(unanswered(file, shown, told))
            .map(fact => ({ label: named(fact.id), value: fact.value }))
    });
}

// The newest thing taken straight in `folder`, of Taken's files, each
// { path, at }, or null where none is: the Last an act's drawer says.
function newest(files, folder) {
    const within = `${folder}/`;
    return files.filter(file => typeof file.at === "number" && file.path.startsWith(within)
        && !file.path.slice(within.length).includes("/"))
        .reduce((last, file) => last === null || file.at > last.at ? file : last, null);
}

// Where one of Capture's acts lands, as its drawer's Lands In says it:
// the folder by its own name, as drawers.md's Act example has it
// (Y2YBTS, #3579's review), then `also`, the clipboard beside it where
// it's copied, or "" for nothing more. "Screenshots, and on the
// clipboard" for a screenshot.
function landsIn(folder, also) {
    const name = folder.split("/").filter(part => part !== "").pop() ?? "";
    return [name, also].filter(said => said !== "").join(", ");
}

// The drawer under one of Capture's acts, Screen, Window, Region, Record
// and Stop among them (drawers.md's Act kind, VKCF3Z, 37e49991): what it
// does, in one sentence, then its Key, where it Lands In, and the Last one
// taken there, with when; where it lands is said once, as a fact
// (VKCF3Z, bc8cda12). Folded under za are what a terminal types to do the
// same, the folder's whole path and the format of what it takes, as the
// Act kind's fold has them, never among its facts. Its acts are named here,
// Return's first, rather than made from its row's verbs, which made rows
// each time an arrow rested on one (KP0L6A's ee796b07). `act` is its {
// title, does, lands, command, path, format, acts }, `acts` their names;
// `key` its chord as words, "" for none, and `last` the newest thing
// where it lands, or null; `told` as `drawer`'s, its `eyebrow` the
// place's name. A fact or a bit it can't give, as a first screenshot's
// Last, is left out.
function act(row, key, last, told) {
    const { now, named, eyebrow, days } = told;
    const said = fact => ({ label: named(fact.id), value: fact.value });
    return {
        eyebrow,
        title: row.title,
        body: row.does,
        facts: [
            { id: "key", value: key },
            { id: "lands", value: row.lands },
            { id: "last", value: last === null ? "" : taken(last.at, now, days) }
        ].filter(fact => fact.value !== "").map(said),
        nerd: ["command", "path", "format"].map(id => ({ id, value: row[id] ?? "" }))
            .filter(bit => bit.value !== "").map(said),
        acts: (row.acts ?? []).map((name, at) => ({ name, key: at === 0 ? "\u21b5" : "" }))
    };
}

// The fact file-preview would have said, where it's silent and hasn't.
function unanswered(file, shown, told) {
    const id = PICTURE.test(file.path) ? "dimensions" : VIDEO.test(file.path) ? "length" : "";
    return told.silent && !shown && id !== "" ? [{ id, value: told.said.silent ?? "" }] : [];
}
