// What a file or a folder says of itself in the launcher's chooser of
// files (open-with's FileChooser.qml): how big it is and when it last
// changed, beside its name on its row, and the plate the drawer shows for
// it while it's selected, from what file-preview says of it (VKCF3Z,
// 2026-11-03). Plain JavaScript, held to it by tests/file-facts.qml in
// Qt's own engine. The words it says are the launcher's (launcher/
// words.json), given it by the chooser as `words` (wordsOf), so each is
// said in one place (Y2YBTS, 2026-11-03).
//
// A number keeps to its unit with a no-break space, so "2.41 MB" never
// leaves its MB alone on the next line.
.pragma library
.import "When.js" as When

const KEPT = "\u00a0";
const BETWEEN = " \u00b7 ";
const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const SECONDS_IN_A_MINUTE = 60;
const SECONDS_IN_AN_HOUR = 3600;
// The Nix store, whose every entry is named by a hash of what built it, in
// Nix's own 32 letters and digits (no e, o, u or t), then a dash and its
// own name.
const STORE = "/nix/store/";
const HASHED = /^[0-9a-df-np-sv-z]{32}-(.+)$/;
// SI's steps, largest first, each a thousand of the one after it.
const THOUSAND = 1000;
const UNITS = [[THOUSAND ** 4, "TB"], [THOUSAND ** 3, "GB"], [THOUSAND ** 2, "MB"],
    [THOUSAND, "kB"]];
// A size keeps three significant digits, as every figure does (design.md).
const FIGURES = 3;
// A file of text's kind by its format's own name, where its ending isn't
// it, by the ending file-preview says; plain text is said as the words
// say Text, and any other ending as itself.
const TEXT_NAMED = {
    C: "C", CSS: "CSS", CSV: "CSV", GO: "Go", HTML: "HTML", JS: "JavaScript",
    JSON: "JSON", MARKDOWN: "Markdown", MD: "Markdown", NIX: "Nix",
    PY: "Python", QML: "QML", RS: "Rust", SH: "Shell", TOML: "TOML", TS: "TypeScript",
    XML: "XML", YAML: "YAML", YML: "YAML"
};

// The words it says, each thing's as `of` gives it, as Words.of does, and
// a day's as `date(date, format)` says it, as Qt.formatDate names days and
// months in the locale's words: the labels of a plate's facts, its kinds,
// and what's said of a count and a time.
function wordsOf(of, date) {
    const said = id => of(id).said ?? {};
    return {
        size: of("file-chooser:size").name,
        length: of("file-chooser:length").name,
        dimensions: of("file-chooser:dimensions").name,
        pages: of("file-chooser:pages").name,
        lines: of("file-chooser:lines").name,
        within: said("file-chooser:where").in ?? "{folder}",
        folder: of("file-chooser:folder").name,
        text: of("file-chooser:text").name,
        file: of("file-chooser:file").name,
        ago: said("file-chooser:changed"),
        counts: {
            items: said("file-chooser:items"),
            lines: said("file-chooser:text")
        },
        marked: said("verb:file-chooser:marked"),
        days: { yesterday: said("file-chooser:changed").yesterday, date }
    };
}

// A number in its words' way, "{count} lines", kept to the word after it.
function filled(said, name, number) {
    return said.replace(`{${name}} `, `${number}${KEPT}`).replace(`{${name}}`, String(number));
}

// A count with its one or its many: "1 line", "4 lines"; `more` says
// there are more than counted, "over 900 lines".
function counted(count, said, more) {
    if (more) {
        return filled(said.more, "count", count);
    }
    return filled(count === 1 ? said.one : said.many, "count", count);
}

// A size in SI units at three significant digits: 12 B, 940 kB, 2.41 MB,
// 19.3 GB, as every size the launcher says reads (VKCF3Z, 2026-11-08).
function weighed(bytes) {
    const unit = UNITS.find(each => bytes >= each[0]);
    if (!unit) {
        return `${Math.round(bytes)}${KEPT}B`;
    }
    // 999.6 kB would round to 1000 kB: it's 1.00 MB instead.
    const scaled = Number((bytes / unit[0]).toPrecision(FIGURES));
    const next = UNITS[UNITS.indexOf(unit) - 1];
    if (scaled >= THOUSAND && next) {
        return `${(scaled / THOUSAND).toPrecision(FIGURES)}${KEPT}${next[1]}`;
    }
    return `${scaled.toPrecision(FIGURES)}${KEPT}${unit[1]}`;
}

// A file's name, the last part of its path as Joel reads it (`said`, from
// home as ~): "~/src/core.nix" is core.nix, and / and ~ are themselves.
// One kept straight in the Nix store goes by its own name, past the hash
// its path begins with, as "track.mp3", since the hash says nothing to
// him (VKCF3Z, 2026-11-08). One `listed` among the store's own entries
// keeps the hash, or the store's listing would read as rows of one name.
function named(said, listed) {
    const cut = said.lastIndexOf("/");
    const last = cut < 0 || said === "/" ? said : said.slice(cut + 1);
    const own = HASHED.exec(last);
    return own !== null && !listed && said === `${STORE}${last}` ? own[1] : last;
}

// Where a name's own part ends, before its extension, as the caret starts
// there in a save's name so typing renames it: before the last dot, but
// for a dot that begins a hidden name or ends one, which isn't one.
function nameEnd(name) {
    const dot = name.lastIndexOf(".");
    return dot > 0 && dot < name.length - 1 ? dot : name.length;
}

// How many items a folder shows: 14 items, 1 item, or that it's empty.
function items(count, words) {
    if (count === 0) {
        return words.counts.items.none;
    }
    return counted(count, words.counts.items, false);
}

function twoFigures(number) {
    return String(number).padStart(2, "0");
}

function clock(at) {
    return `${twoFigures(at.getHours())}:${twoFigures(at.getMinutes())}`;
}

// When it last changed, as the launcher says when on a row (Y2YBTS,
// 2026-11-03), both in milliseconds: under a minute, "just now"; under an
// hour, "12 min ago"; earlier today, "14:32"; else its day as When.js
// says every day, never broken across lines. A moment ahead of now, as a
// clock set wrong gives, is said as today's time, or else its day.
function when(changed, now, words) {
    const since = now - changed;
    if (since >= 0 && since < MINUTE) {
        return words.ago.now;
    }
    if (since >= 0 && since < HOUR) {
        return filled(words.ago.minutes, "minutes", Math.floor(since / MINUTE));
    }
    const day = When.day(changed, now, words.days);
    return day === "" ? clock(new Date(changed)) : day.replace(/ /g, KEPT);
}

// How much is in it, as its row says: a file's size, or a folder's count
// once it's known and nothing until then.
function amount(entry, count, words) {
    if (!entry.folder) {
        return weighed(entry.size ?? 0);
    }
    return count === undefined || count === null ? "" : items(count, words);
}

// What sits beside a file's or a folder's name on its row: its size, or a
// folder's count once it's known, and when it changed, "2.41 MB · 28 Oct".
function side(entry, count, now, words) {
    return [amount(entry, count, words), when(entry.modified, now, words)]
        .filter(part => part !== "").join(BETWEEN);
}

// How long a video runs: 0:42, 3:41, 1:02:03.
function length(seconds) {
    const whole = Math.round(seconds);
    const hours = Math.floor(whole / SECONDS_IN_AN_HOUR);
    const minutes = Math.floor(whole % SECONDS_IN_AN_HOUR / SECONDS_IN_A_MINUTE);
    const rest = twoFigures(whole % SECONDS_IN_A_MINUTE);
    return hours > 0 ? `${hours}:${twoFigures(minutes)}:${rest}` : `${minutes}:${rest}`;
}

// What kind of file it is: its ending, a file of text's by its own name,
// or, with none, "Text" or "File".
function kindOf(said, words) {
    const text = said.lines !== undefined;
    if (said.kind === "TXT" && text) {
        return words.text;
    }
    if (said.kind !== undefined) {
        return text ? TEXT_NAMED[said.kind] ?? said.kind : said.kind;
    }
    return text ? words.text : words.file;
}

// The eyebrow over its plate, where, then kind, as every file's drawer
// says it: "in ~/picked \u00b7 MP4", a folder's kind "Folder" (VKCF3Z,
// 2026-11-08). Its folder is set as written, since a folder is a name
// never re-cased; short of room the kind gives way first, then the folder
// from its middle, and the folder never goes. Until file-preview has
// said what a file is, it's where alone. A place that says where in its
// own words (`within`, "in My Drive") is said so.
function placed(where, kind, words, within) {
    const said = within || (where === "" ? "" : words.within.replace("{folder}", where));
    const parts = [
        { said, keep: 2, path: where },
        { said: kind, keep: 1 }
    ].filter(part => part.said !== "");
    return {
        eyebrow: parts.map(part => part.said).join(BETWEEN),
        eyebrowItems: parts,
        eyebrowNames: true
    };
}

// What its plate says that its row doesn't, once file-preview has said:
// a video's length, a picture's dimensions when the picture is the file
// itself (`path`), a document's pages, and a text's lines, "over 900"
// where file-preview stopped counting, each count the number alone since
// its label says what it counts. A duration or dimensions is a fact,
// never the eyebrow's (VKCF3Z, 2026-11-08).
function told(said, path, words) {
    if (!said) {
        return [];
    }
    return [
        said.seconds === undefined ? null : { label: words.length, value: length(said.seconds) },
        said.picture === path && said.across ? {
            label: words.dimensions,
            value: `${said.across}${KEPT}\u00d7${KEPT}${said.down}`
        } : null,
        said.pages === undefined ? null : { label: words.pages, value: String(said.pages) },
        said.count === undefined ? null : {
            label: words.lines,
            value: said.more ? filled(words.counts.lines.over, "count", said.count)
                : String(said.count)
        }
    ].filter(fact => fact !== null);
}

// What the plate previews: a picture, a file's first lines or a folder's
// first items, quiet; or nothing, with no stand-in in its place.
function previewOf(said) {
    if (said?.picture) {
        return { picture: said.picture, across: said.across ?? 0, down: said.down ?? 0 };
    }
    if ((said?.lines ?? []).length > 0) {
        return { lines: said.lines };
    }
    if ((said?.names ?? []).length > 0) {
        return { lines: said.names, quiet: true };
    }
    return undefined;
}

// A name's runs of digits, read as numbers so 2 comes before 10.
const RUNS = /(\d+)/;

// Two names A to Z, case aside, each run of digits as its number: "note
// 2" before "Note 10".
function natural(one, other) {
    const ones = one.toLowerCase().split(RUNS);
    const others = other.toLowerCase().split(RUNS);
    const differs = ones.findIndex((mine, at) => at < others.length && mine !== others[at]);
    if (differs >= 0) {
        const [mine, theirs] = [ones[differs], others[differs]];
        // split puts each run of digits at an odd place.
        if (differs % 2 === 1 && Number(mine) !== Number(theirs)) {
            return Number(mine) - Number(theirs);
        }
        return mine < theirs ? -1 : 1;
    }
    return ones.length - others.length || (one < other ? -1 : one > other ? 1 : 0);
}

// How big an entry is to order by: a file's bytes, a folder's items once
// counted (`counts`, by path), anything not yet known the least.
function bulk(entry, counts) {
    const known = entry.folder ? counts[entry.path] : entry.size;
    return typeof known === "number" ? known : -1;
}

// A folder's entries in `order`: by name, A to Z (natural); by when each
// changed, newest first; or by size, largest first, a folder by how many
// items it holds. Folders come first in every order, ordered among
// themselves the same way, and a tie goes by name (VKCF3Z, 2026-11-04);
// `reversed` turns each round, as :sort! does.
function ordered(entries, order, counts, reversed) {
    const by = {
        changed: (one, other) => (other.modified ?? 0) - (one.modified ?? 0),
        size: (one, other) => bulk(other, counts) - bulk(one, counts)
    }[order];
    const way = reversed === true ? -1 : 1;
    return entries.slice().sort((one, other) => (other.folder === true) - (one.folder === true)
        || way * ((by ? by(one, other) : 0) || natural(one.name, other.name)));
}

// The drawer's plate for the files marked, in the order they were, each
// its name, its folder from ~ (where) and its size where its folder's read
// said it: the eyebrow how many, a quiet line each with its folder beside
// it, then their Size together, not yet known while any one's isn't
// (VKCF3Z, 2026-11-03).
function markedPlate(title, files, words) {
    const sizes = files.map(file => file.size);
    const known = sizes.every(size => typeof size === "number");
    return {
        eyebrow: filled(words.marked.marked, "count", files.length),
        title,
        preview: {
            lines: files.map(file => file.where === "" ? file.name
                : `${file.name}${BETWEEN}${file.where}`),
            quiet: true
        },
        facts: [{
            label: words.size,
            value: known ? weighed(sizes.reduce((all, size) => all + size, 0)) : null
        }]
    };
}

// The drawer's plate for a file or a folder, `file` its path, name,
// folder from ~ (where) and what its folder's read said of it (entry):
// where and what kind above, its name, its preview, then what its row
// doesn't say. Its size or count and when it changed are its row's side,
// and where it is its eyebrow's, so none is a fact: the drawer is the
// preview plus what the row doesn't say (VKCF3Z, 2026-11-08).
function plate(file, said, words) {
    const { path, name, where, entry } = file;
    const folder = entry?.folder === true || said?.items !== undefined;
    const kind = folder ? words.folder : said ? kindOf(said, words) : "";
    const shown = Object.assign(placed(where, kind, words), {
        title: name,
        facts: folder ? [] : told(said, path, words)
    });
    const preview = previewOf(said);
    if (preview) {
        shown.preview = preview;
    }
    return shown;
}

// A file or a folder kept elsewhere, as Google's Drive keeps one, is
// drawn as one here is, from what its place said as it listed it,
// `entry` ({ name, folder, size, modified }), since nothing here can be
// previewed before it's fetched. Its row says its size, or nothing for a
// folder whose items its place doesn't count, and when it changed, each
// where its place says it.
function sideElsewhere(entry, now, words) {
    const size = !entry.folder && typeof entry.size === "number" ? weighed(entry.size) : "";
    const changed = typeof entry.modified === "number" ? when(entry.modified, now, words) : "";
    return [size, changed].filter(part => part !== "").join(BETWEEN);
}

// Its plate, `file` its name, the folders it's in as its place names them
// (where), where it is in its place's own words (within, "in My Drive"),
// its entry and what more its place says of it (facts): the eyebrow where
// it is then its kind, by its ending as file-preview says one here, or a
// folder's, as the place's own drawer says them; its name; then its
// place's facts. Its size and when it changed are its row's side, and
// where it is its eyebrow's, as a file's here are (VKCF3Z, 2026-11-08).
function plateElsewhere(file, words) {
    const kind = file.entry.folder ? words.folder : kindByName(file.name, words);
    return Object.assign(placed(file.where ?? "", kind, words, file.within ?? ""), {
        title: file.name,
        facts: file.facts ?? []
    });
}

// What kind of file a name says it is, where nothing has read the file:
// its ending in capitals, as file-preview names a kind ("PNG", "PDF"), or
// a file's word for a name with none, so every file's eyebrow says where
// and then what, whoever lists it.
function kindByName(name, words) {
    const ending = nameEnd(name);
    return ending < name.length ? name.slice(ending + 1).toUpperCase() : words.file;
}

// A picture the shell draws itself, by its ending.
const DRAWN = /\.(?:png|jpe?g|webp|avif|gif)$/i;

// A picture previewed as it is, where nothing has read it yet, as a
// picture taken is (launcher/Captured.js): its path, its size left to
// the picture itself; or undefined for a file that isn't one.
function pictured(path) {
    return DRAWN.test(String(path ?? "")) ? { picture: path, across: 0, down: 0 } : undefined;
}
