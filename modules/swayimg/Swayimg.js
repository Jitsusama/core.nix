// What swayimg's provider in the launcher works out (Screenshots.qml), kept
// apart from Quickshell's types so tests/swayimg-galleries.qml runs it in
// Qt's own engine: the folders its page offers to look through, from the
// pictures in swayimg's Recent, newest first, each once, and which picture
// each opens on; the command that reads how many pictures each holds, and
// what it printed; what each gallery's row says beside it; which row leads
// its page; the command that opens a gallery; and what its drawer says of
// the screenshots. A count and a time are said as the launcher says them of
// any file (FileFacts.js).
.pragma library
.import "../FileFacts.js" as FileFacts

// A file swayimg draws, by its ending: the formats it reads, so a folder's
// count is of what its gallery shows, never its other files.
const PICTURE = /\.(?:a?png|jpe?g|gif|webp|avif|hei[cf]|jxl|bmp|tiff?|svgz?|exr|qoi|p[bgnp]m)$/i;

// The Nix store, whose every entry is something built: a picture kept
// straight in it has no folder to look through, and reading the store
// whole would list every build on the machine.
const STORE = "/nix/store";

// Between items said in one place, as every side joins them.
const BETWEEN = " \u00b7 ";

// Said while a folder's pictures aren't counted yet, as every row still
// reading says it.
const WAITING = "\u2026";

// Whether a file is one swayimg draws, by its name.
function isPicture(path) {
    return PICTURE.test(String(path ?? ""));
}

// The folder a path is in: "/a/b.png" is in /a, and "/b.png" in /.
function folderOf(path) {
    const cut = path.lastIndexOf("/");
    return cut <= 0 ? "/" : path.slice(0, cut);
}

// The folders the pictures in swayimg's Recent are in, newest first as
// Recent lists them, each once, with its pictures there, newest first:
// [{ folder, pictures }]. A folder in `skipped`, as the screenshots' is
// while its own gallery leads, isn't listed again, nor the Nix store, nor
// anything that isn't a whole path.
function folders(recent, skipped) {
    const each = [];
    const at = {};
    for (const path of recent ?? []) {
        if (!String(path).startsWith("/")) {
            continue;
        }
        const folder = folderOf(path);
        if (folder === STORE || (skipped ?? []).includes(folder)) {
            continue;
        }
        if (!(folder in at)) {
            at[folder] = each.length;
            each.push({ folder, pictures: [] });
        }
        each[at[folder]].pictures.push(path);
    }
    return each;
}

// What reads the files in these folders, a whole path a line, without
// going into the folders inside them or listing what's hidden, as
// swayimg's gallery leaves those out. The launcher runs it without a
// shell.
function listing(within) {
    return ["find"].concat(within, ["-maxdepth", "1", "-type", "f", "-not", "-name", ".*",
        "-printf", "%p\\n"]);
}

// What the listing printed, of the folders it read (`within`): how many
// pictures each holds and which pictures are there, { within, counts,
// there }, `counts` by folder and `there` by path. A folder it read that
// holds none, or is gone, counts none.
function read(lines, within) {
    const counts = {};
    const there = {};
    for (const folder of within) {
        counts[folder] = 0;
    }
    for (const path of lines) {
        const folder = folderOf(path);
        if (folder in counts && isPicture(path)) {
            counts[folder] += 1;
            there[path] = true;
        }
    }
    return { within: within.slice(), counts, there };
}

// Each folder's gallery, as its page offers them, newest first: { folder,
// count, target }, `count` null until `listed` (read's) has counted it,
// and `target` what swayimg opens: the newest picture of Recent's still
// there, so its gallery opens on it, or the folder alone, opening on its
// newest, where none is. A folder counted holding no picture isn't
// offered, since its gallery would open on nothing.
function galleries(recent, listed, skipped) {
    const offered = [];
    for (const one of folders(recent, skipped)) {
        if (!(listed?.within ?? []).includes(one.folder)) {
            offered.push({ folder: one.folder, count: null, target: one.pictures[0] });
            continue;
        }
        const count = listed.counts[one.folder] ?? 0;
        if (count === 0) {
            continue;
        }
        const kept = one.pictures.find(path => listed.there[path] === true);
        offered.push({ folder: one.folder, count, target: kept ?? one.folder });
    }
    return offered;
}

// What a gallery's row says beside its name: how many pictures it holds
// and then where its folder is, "42 pictures · ~", as a folder's row says
// where it is, so a side cut short loses where and never the count; an
// ellipsis while it's being counted. `said` gives a count's words, its
// `one` and `many`.
function side(count, within, said) {
    const counted = count === null ? WAITING : FileFacts.counted(count, said, false);
    return [counted, within].filter(part => part !== "").join(BETWEEN);
}

// A gallery's name, its words' with the folder's own name in it, as it's
// said wherever a folder is: "Look Through Downloads".
function named(words, name) {
    return String(words ?? "").replace("{folder}", name);
}

// Which of its galleries leads the page, so Return on swayimg lands on
// it: the screenshots' while it holds any, as `screenshots` counts them,
// "screenshots"; else the first of `offered`, by its folder; else "",
// leaving nothing to lead.
function lead(screenshots, offered) {
    if (screenshots > 0) {
        return "screenshots";
    }
    return offered.length > 0 ? offered[0].folder : "";
}

// What opens a gallery: `gallery`, swayimg in its gallery, newest first, as
// home.nix gives it, on what it's to open, a folder or a picture in one.
function opening(gallery, target) {
    return (gallery ?? []).concat([target]);
}

// What swayimg's drawer in Programs says of the screenshots, by each fact's
// id in the launcher's words: when the newest was taken, as a file's row
// says when (`newest` the one reader's, Taken.qml, with its `at` in
// milliseconds), and how many there are, as Tuple's says its last call and
// then how many; nothing while there are none. `words` are FileFacts'.
function held(newest, count, now, words) {
    if (count === 0 || newest === null || newest === undefined) {
        return [];
    }
    return [
        { id: "swayimg:screenshots:newest", value: FileFacts.when(newest.at, now, words) },
        { id: "swayimg:screenshots:count", value: String(count) }
    ];
}
