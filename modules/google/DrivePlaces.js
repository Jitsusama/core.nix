// Drive's own places in the launcher, Programs › Google Drive
// (DriveProgram.qml): which of Starred, Shared with Me and Shared Drives
// it lists and the count each says, and everything a Drive file's drawer
// says beyond its owner and when it changed (Drive.js's details), so the
// one row every surface draws a Drive file with (Drive.qml's row) shows
// what the google program reads of it (src/drive.rs says which, and why
// the rest are left out). An eyebrow says no state (docs/design.md's
// Type), so whether he starred it is said by Starred's holding it, that
// it was shared with him by its Shared and Shared By, and whether his own
// is shared by Drive's Share, one Open away, as src/drive.rs leaves who
// may see it to Drive. It's kept apart from Drive.js, which the
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

// Its kind, as the drawer's eyebrow says it after where it is, each part
// an item the eyebrow gives way by, with what a picture or a video
// measures: a file by its ending, as a mail's file is ("PDF"), a picture
// with its size ("PNG", "2400 × 1600") and a video with its length
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
    return [kind, measured, file.lasts ? lasting(file.lasts) : ""].filter(part => part !== "");
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

// Where it is, as the drawer's eyebrow says it before its kind, in
// `said`'s words: on the shared drive it's on, by its name among `drives`
// (the program's places' shared drives), "in Design (shared drive)", or
// in Shared Drives while that drive's name isn't read; in Shared with Me
// where someone shared it with him, since it's in their Drive, not his;
// else in My Drive. The folder it's in isn't said: the program doesn't
// read a file's folders, since naming each would take an ask of Drive a
// file (src/drive.rs, `parents`), and the drive it's in is what the
// record already holds. A shared drive's own is nothing, its title
// naming it and its kind saying it's one.
function where(file, drives, said) {
    if (file.kind === "drive") {
        return "";
    }
    return said.in.replace("{place}", placeOf(file, drives, said));
}

function placeOf(file, drives, said) {
    if (file.drive) {
        const drive = (drives ?? []).find(each => each.id === file.drive);
        return drive ? said.onDrive.replace("{drive}", drive.title) : said.drives;
    }
    return file.sharedWithMe ? said.sharedWithMe : said.mine;
}

// The order its facts are said in, what changes most first, then what
// describes it, then what identifies it, as every drawer's are (VKCF3Z,
// 2026-11-08): who changed it and when, then whose it is, then the rest.
const ORDER = ["Changed", "Changed By", "Owner", "You Changed", "Created", "Shared", "Shared By",
    "Size", "ID"];

// Its drawer, as Drive.js's details give it, with the rest of what the
// program read: where it is and its kind, as its eyebrow, in Note as
// written, since My Drive and a drive's name are Drive's words, not the
// shell's; who changed it last, when Joel himself did, when it was made,
// when and by whom it was shared with him and its size, in ORDER; its id
// last, marked as an identifier (`identifies`), which docs/design.md's
// drawer sets in the Name face; and what its owner wrote of
// it as its words. `around` gives what it's said among: { drives, kinds,
// said, now, dates }, the shared drives, the words for its kinds and
// where it is, and the time, said as every Drive file's is.
function drawn(details, file, around) {
    const when = at => (at ? Mail.moment(at, around.now, around.dates) : "");
    const more = [
        { label: "Changed By", value: file.changedBy ?? "" },
        { label: "You Changed", value: when(file.changedByMe) },
        { label: "Created", value: when(file.created) },
        { label: "Shared", value: when(file.sharedWithMe) },
        { label: "Shared By", value: file.sharedBy ?? "" },
        { label: "Size", value: typeof file.size === "number" ? Mail.weighed(file.size) : "" },
        { label: "ID", value: file.id ?? "", identifies: true }
    ];
    const rank = fact => ORDER.indexOf(fact.label);
    const facts = details.facts.concat(more).filter(fact => fact.value !== "")
        .sort((one, other) => rank(one) - rank(other));
    // Short of room, its eyebrow gives way by whole items from its end, as
    // every eyebrow does (Reading.wholeItems): a video's length goes first,
    // then what it measures, then its kind, and where it is never.
    const said = [where(file, around.drives, around.said)]
        .concat(kindOf(file, around.kinds)).filter(part => part !== "");
    const eyebrowItems = said.map((part, at) => ({ said: part, keep: said.length - at }));
    return Object.assign({}, details, {
        eyebrow: said.join(" \u00b7 "),
        eyebrowItems,
        eyebrowNames: true,
        facts,
        body: file.about ?? ""
    });
}
