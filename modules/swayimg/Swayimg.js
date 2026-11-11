// What swayimg's provider in the launcher works out (Screenshots.qml), kept
// apart from Quickshell's types so tests/swayimg-galleries.qml runs it in
// Qt's own engine: the folders its page offers to look through, from the
// pictures in swayimg's Recent, newest first, each once, and which picture
// each opens on; the command that reads how many pictures each holds, and
// what it printed; what each gallery's row says beside it; which row leads
// its page; the command that opens a gallery; what each gallery's drawer
// shows; and what swayimg's own drawer says of the screenshots. A count
// and a time are said as the launcher says them of any file (FileFacts.js).
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

// A second, in the milliseconds a moment is said in.
const SECOND = 1000;

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

// The programs daemon's key for what's in a folder, which it reads itself
// (modules/programs/src/folder.rs): its files, none inside folders in it
// and none hidden, as swayimg's gallery leaves those out.
function folderKey(folder) {
    return `folder:${folder}`;
}

// What the daemon said of the folders read (`within`), a file a line, when
// it last changed in seconds since 1970, its size and its whole path, a
// tab between: how many
// pictures each holds, which pictures are there and each one's newest,
// { within, counts, there, newest }, `counts` and `newest` by folder and
// `there` by path, a newest { path, at } with `at` in milliseconds, as
// the newest gallery tile swayimg shows first. A folder it read that
// holds none, or is gone, counts none and has no newest.
function read(lines, within) {
    const counts = {};
    const there = {};
    const newest = {};
    for (const folder of within) {
        counts[folder] = 0;
    }
    for (const line of lines) {
        const tab = line.indexOf("\t");
        const path = line.slice(line.indexOf("\t", tab + 1) + 1);
        const at = Number(line.slice(0, tab)) * SECOND;
        const folder = folderOf(path);
        if (!(folder in counts && isPicture(path))) {
            continue;
        }
        counts[folder] += 1;
        there[path] = true;
        if (Number.isFinite(at) && at > (newest[folder]?.at ?? -1)) {
            newest[folder] = { path, at };
        }
    }
    return { within: within.slice(), counts, there, newest };
}

// Each folder's gallery, as its page offers them, newest first: { folder,
// count, target, newest }, `count` null until `listed` (read's) has
// counted it, `target` what swayimg opens: the newest picture of Recent's
// still there, so its gallery opens on it, or the folder alone, opening on
// its newest, where none is; and `newest` the folder's newest picture,
// read's, null until it's read. A folder counted holding no picture isn't
// offered, since its gallery would open on nothing.
function galleries(recent, listed, skipped) {
    const offered = [];
    for (const one of folders(recent, skipped)) {
        if (!(listed?.within ?? []).includes(one.folder)) {
            offered.push({ folder: one.folder, count: null, target: one.pictures[0],
                newest: null });
            continue;
        }
        const count = listed.counts[one.folder] ?? 0;
        if (count === 0) {
            continue;
        }
        const kept = one.pictures.find(path => listed.there[path] === true);
        offered.push({ folder: one.folder, count, target: kept ?? one.folder,
            newest: listed.newest?.[one.folder] ?? null });
    }
    return offered;
}

// A gallery's drawer, as a folder's plate is drawn wherever a file's is
// (FileFacts.js): where its folder is, then its kind, a folder; its
// folder's name; the newest picture, previewed; then that picture's name,
// when it was taken, for the screenshots', or added, for any other
// folder's, and how many pictures there are. `gallery` is { where, name,
// count, newest, target, taken }, `where` its folder from ~ and `newest`
// a { path, at }, or null before it's read, when what the gallery opens
// on, `target`, is previewed in its place and nothing more is said;
// `words` are { file, labelled }, FileFacts' words and each fact's label
// by its id.
function plate(gallery, now, words) {
    const { file, labelled } = words;
    const newest = gallery.newest ?? null;
    const facts = newest === null ? [] : [
        {
            label: labelled("swayimg:gallery:newest"),
            value: FileFacts.named(newest.path, false)
        },
        {
            label: labelled(gallery.taken ? "swayimg:gallery:taken" : "swayimg:gallery:added"),
            value: FileFacts.when(newest.at, now, file)
        },
        { label: labelled("swayimg:gallery:count"), value: String(gallery.count ?? "") }
    ].filter(fact => fact.value !== "");
    const shown = Object.assign(FileFacts.placed(gallery.where, file.folder, file), {
        title: gallery.name,
        facts
    });
    const preview = FileFacts.pictured(newest?.path ?? gallery.target ?? "");
    if (preview) {
        shown.preview = preview;
    }
    return shown;
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
