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
        changed: of("file-chooser:changed").name,
        where: of("file-chooser:where").name,
        items: of("file-chooser:items").name,
        folder: of("file-chooser:folder").name,
        text: of("file-chooser:text").name,
        file: of("file-chooser:file").name,
        ago: said("file-chooser:changed"),
        counts: {
            items: said("file-chooser:items"),
            lines: said("file-chooser:text"),
            pages: said("file-chooser:file")
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

// The eyebrow over its plate: its kind with its numbers, "PNG · 2400 ×
// 1600", "PDF · 12 pages", "MP4 · 3:41", "Markdown · 4 lines" ("over
// 900 lines" where file-preview stopped counting), a picture's numbers
// only when the picture is the file itself (`path`); a folder's "Folder";
// anything else its kind alone.
function eyebrow(said, path, words) {
    if (said.items !== undefined) {
        return words.folder;
    }
    const kind = kindOf(said, words);
    if (said.pages !== undefined) {
        return `${kind}${BETWEEN}${counted(said.pages, words.counts.pages, false)}`;
    }
    if (said.seconds !== undefined) {
        return `${kind}${BETWEEN}${length(said.seconds)}`;
    }
    if (said.picture === path && said.across) {
        return `${kind}${BETWEEN}${said.across}${KEPT}\u00d7${KEPT}${said.down}`;
    }
    if (said.count !== undefined) {
        return `${kind}${BETWEEN}${counted(said.count, words.counts.lines, said.more)}`;
    }
    return kind;
}

// Its first fact: a folder's count, the number alone since its label says
// what it counts, null until it's said; or a file's size, null while it's
// unknown.
function firstFact(folder, size, said, words) {
    if (folder) {
        return { label: words.items, value: said ? String(said.items) : null };
    }
    return { label: words.size, value: size === undefined ? null : weighed(size) };
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
// the eyebrow, its name, its preview, then its facts, Size or Items,
// Changed and Where. Until file-preview has said (`said` null), it's its
// facts from the folder read alone, so they draw at once; what's not
// known yet is null, shown as the drawer's placeholder.
function plate(file, said, now, words) {
    const { path, name, where, entry } = file;
    const folder = entry?.folder === true || said?.items !== undefined;
    const changed = said?.changed ?? entry?.modified;
    const shown = {
        eyebrow: "",
        title: name,
        facts: [
            firstFact(folder, said?.size ?? entry?.size, said, words),
            // Said as its row says it, so the two never read as two dates.
            {
                label: words.changed,
                value: changed === undefined ? null : when(changed, now, words)
            },
            { label: words.where, value: where }
        ]
    };
    if (said) {
        shown.eyebrow = eyebrow(said, path, words);
    } else if (folder) {
        shown.eyebrow = words.folder;
    }
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
// folder's, as the place's own drawer says them (VKCF3Z, 2026-11-08); its
// name; then Size and Changed as a file's here, Where too for a place
// that says no `within`, and its place's facts after them. A figure its
// place doesn't give is left out, never awaited, since nothing more is
// coming.
function plateElsewhere(file, now, words) {
    const entry = file.entry;
    const shown = plate({ path: "", name: file.name, where: file.where ?? "", entry }, null, now,
        words);
    const ending = nameEnd(file.name);
    const kind = entry.folder ? words.folder
        : ending < file.name.length ? file.name.slice(ending + 1).toUpperCase() : words.file;
    const within = file.within ?? "";
    const said = shown.facts.filter(fact => fact.value !== null && fact.value !== ""
        && (within === "" || fact.label !== words.where));
    return Object.assign(shown, {
        eyebrow: within === "" ? kind : `${within}${BETWEEN}${kind}`,
        facts: said.concat(file.facts ?? [])
    }, within === "" ? {} : { eyebrowNames: true });
}
