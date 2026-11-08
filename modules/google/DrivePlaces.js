// Drive's own places in the launcher, Programs › Google Drive
// (DriveProgram.qml): which of Starred, Shared with Me and Shared Drives
// it lists and the count each says, and everything a Drive file's drawer
// says beyond its owner and when (Drive.js's details), so the one row
// every surface draws a Drive file with (Drive.qml's row) shows every
// figure the google program reads of it (src/drive.rs says which, and why
// the rest are left out). It's kept apart from Drive.js, which the
// picker's rows share, so each changes on its own. The
// the-shell-reads-the-documents check runs it in Qt's own engine
// (tests/drive.qml).
.pragma library
.import "Mail.js" as Mail

// The places Google Drive holds after its own acts, in Drive's own order,
// each by its words and the part of the program's places it shows. Its
// Recent is the program's Recent, given apart (DriveProgram.qml's
// recentOf), since every program's Recent is one row of its own.
const PLACES = [
    { key: "drive:place:starred", from: "starred" },
    { key: "drive:place:shared", from: "shared" },
    { key: "drive:place:drives", from: "drives" }
];

// Each place that holds anything, as { key, files, count }: one holding
// nothing isn't listed, as an empty Recent isn't. Its count is how many
// it holds, or its words for more, `over(key)`, said of them where Drive
// holds more than it told.
function listed(places, over) {
    return PLACES.map(place => {
        const placed = places?.[place.from] ?? { files: [], more: false };
        const files = placed.files ?? [];
        const count = placed.more ? over(place.key).replace("{count}", String(files.length))
            : String(files.length);
        return { key: place.key, files, count };
    }).filter(place => place.files.length > 0);
}

// Its kind, as the drawer says it, with what a picture or a video
// measures: a file by its ending, as a mail's file is ("PDF"), a picture
// with its size ("PNG · 2400 × 1600") and a video with its length
// too; one of Google's other kinds by the name Drive's type gives it
// ("Form"); the rest by `named`'s word for each, and a file with no
// ending as `named.file`.
const GOOGLES = "application/vnd.google-apps.";

function kindOf(file, named) {
    const other = (file.mime ?? "").startsWith(GOOGLES) ? file.mime.slice(GOOGLES.length) : "";
    const kind = file.kind === "file" ? Mail.kind({ name: file.title ?? "" }, named.file)
        : file.kind === "other" && other !== "" ? other[0].toUpperCase() + other.slice(1)
        : named[file.kind] ?? named.file;
    const measured = file.across && file.down ? `${file.across} \u00d7 ${file.down}` : "";
    return [kind, measured, file.lasts ? lasting(file.lasts) : ""]
        .filter(part => part !== "").join(" \u00b7 ");
}

// A length in milliseconds as a player says one: 3:41, or 1:02:03.
const SECOND = 1000;
const MINUTE = 60;
const HOUR = 3600;

function lasting(milliseconds) {
    const seconds = Math.round(milliseconds / SECOND);
    const two = number => String(number).padStart(2, "0");
    const hours = Math.floor(seconds / HOUR);
    const minutes = Math.floor((seconds % HOUR) / MINUTE);
    const rest = two(seconds % MINUTE);
    return hours > 0 ? `${hours}:${two(minutes)}:${rest}` : `${minutes}:${rest}`;
}

// Where it is, as the drawer's eyebrow says it: the shared drive it's on,
// by its name among `drives` (the program's places' shared drives), then
// Starred where Joel starred it, and Shared with Me where someone did, or
// shared where he shares his own, each in `said`'s words. A shared
// drive's own eyebrow needs no name, its title being it.
function where(file, drives, said) {
    const drive = file.kind === "drive" ? null
        : (drives ?? []).find(each => each.id === file.drive)?.title ?? null;
    const shared = file.sharedWithMe ? said.sharedWithMe
        : file.shared && !file.drive ? said.shared : "";
    return [drive ?? "", file.starred ? said.starred : "", shared]
        .filter(part => part !== "");
}

// Its drawer, as Drive.js's details give it, with the rest of what the
// program read: where it is, as its eyebrow, who changed it last, when
// Joel himself did, when it was made, when and by whom it was shared with
// him, its kind and size, and what its owner wrote of it as its words.
// `around` gives what it's said among: { drives, kinds, said, now,
// dates }, the shared drives, the words for its kinds and where it is,
// and the time, said as every Drive file's is.
function drawn(details, file, around) {
    const when = at => (at ? Mail.dated(at, around.now, around.dates) : "");
    const more = [
        { label: "Changed By", value: file.changedBy ?? "" },
        { label: "You Changed", value: when(file.changedByMe) },
        { label: "Created", value: when(file.created) },
        { label: "Shared", value: when(file.sharedWithMe) },
        { label: "Shared By", value: file.sharedBy ?? "" },
        { label: "Kind", value: kindOf(file, around.kinds) },
        { label: "Size", value: typeof file.size === "number" ? Mail.weighed(file.size) : "" }
    ];
    const eyebrowItems = where(file, around.drives, around.said);
    return Object.assign({}, details, {
        eyebrow: eyebrowItems.join(" \u00b7 "),
        eyebrowItems,
        facts: details.facts.concat(more.filter(fact => fact.value !== "")),
        body: file.about ?? ""
    });
}
