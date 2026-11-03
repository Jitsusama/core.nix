// What a file or a folder says of itself in the launcher's chooser of
// files (open-with's FileChooser.qml): how big it is and when it last
// changed, beside its name on its row, and the plate the drawer shows for
// it while it's selected, from what file-preview says of it (VKCF3Z,
// 2026-11-03). Plain JavaScript, held to it by tests/file-facts.qml in
// Qt's own engine.
//
// A number keeps to its unit with a no-break space, so "2.4 MB" never
// leaves its MB alone on the next line.
.pragma library

const KEPT = "\u00a0";
const BETWEEN = " \u00b7 ";
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const SECONDS_IN_A_MINUTE = 60;
const SECONDS_IN_AN_HOUR = 3600;
// SI's steps, largest first, each a thousand of the one after it.
const THOUSAND = 1000;
const UNITS = [[THOUSAND ** 4, "TB"], [THOUSAND ** 3, "GB"], [THOUSAND ** 2, "MB"],
    [THOUSAND, "kB"]];
// Under this a size keeps one decimal: 9.95 rounds to 10, which doesn't.
const KEEPS_A_DECIMAL = 9.95;
// A file of text's kind by its own name, where its ending isn't one, by
// the ending file-preview says; any other is said as its ending.
const TEXT_NAMED = {
    C: "C", CSS: "CSS", CSV: "CSV", GO: "Go", HTML: "HTML", JS: "JavaScript",
    JSON: "JSON", LOG: "Log", MARKDOWN: "Markdown", MD: "Markdown", NIX: "Nix",
    PY: "Python", QML: "QML", RS: "Rust", SH: "Shell", TOML: "TOML", TS: "TypeScript",
    TXT: "Text", XML: "XML", YAML: "YAML", YML: "YAML"
};

// A size in SI units, with one decimal under 10: 12 B, 940 kB, 2.4 MB.
function weighed(bytes) {
    const unit = UNITS.find(each => bytes >= each[0]);
    if (!unit) {
        return `${Math.round(bytes)}${KEPT}B`;
    }
    const scaled = bytes / unit[0];
    const shown = scaled < KEEPS_A_DECIMAL ? scaled.toFixed(1) : String(Math.round(scaled));
    // 999.95 kB would round to 1000 kB: it's 1.0 MB instead.
    const next = UNITS[UNITS.indexOf(unit) - 1];
    if (shown === String(THOUSAND) && next) {
        return `1.0${KEPT}${next[1]}`;
    }
    return `${shown}${KEPT}${unit[1]}`;
}

// How many items a folder shows: 14 items, 1 item, or that it's empty.
function items(count) {
    if (count === 0) {
        return "empty";
    }
    return `${count}${KEPT}${count === 1 ? "item" : "items"}`;
}

function twoFigures(number) {
    return String(number).padStart(2, "0");
}

function clock(at) {
    return `${twoFigures(at.getHours())}:${twoFigures(at.getMinutes())}`;
}

function sameDay(one, other) {
    return one.getFullYear() === other.getFullYear() && one.getMonth() === other.getMonth()
        && one.getDate() === other.getDate();
}

function day(at, now) {
    const said = `${at.getDate()}${KEPT}${MONTHS[at.getMonth()]}`;
    return at.getFullYear() === now.getFullYear() ? said : `${said}${KEPT}${at.getFullYear()}`;
}

// When it last changed, as its row says it, both in milliseconds: under
// an hour ago, "12 min ago"; earlier today, "14:32"; this year, "28 Oct";
// before, "28 Oct 2024". A moment ahead of now, as a clock set wrong
// gives, is said as today's or its day's.
function when(changed, now) {
    const since = now - changed;
    if (since >= 0 && since < MINUTE) {
        return "just now";
    }
    if (since >= 0 && since < HOUR) {
        return `${Math.floor(since / MINUTE)}${KEPT}min ago`;
    }
    const at = new Date(changed);
    const today = new Date(now);
    return sameDay(at, today) ? clock(at) : day(at, today);
}

// How much is in it, as its row says: a file's size, or a folder's count
// once it's known and nothing until then.
function amount(entry, count) {
    if (!entry.folder) {
        return weighed(entry.size ?? 0);
    }
    return count === undefined || count === null ? "" : items(count);
}

// What sits beside a file's or a folder's name on its row: its size, or a
// folder's count once it's known, and when it changed, "2.4 MB · 28 Oct".
function side(entry, count, now) {
    return [amount(entry, count), when(entry.modified, now)].filter(part => part !== "")
        .join(BETWEEN);
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
function kindOf(said) {
    const text = said.lines !== undefined;
    if (said.kind !== undefined) {
        return text ? TEXT_NAMED[said.kind] ?? said.kind : said.kind;
    }
    return text ? "Text" : "File";
}

function counted(count, one, many) {
    return `${count}${KEPT}${count === 1 ? one : many}`;
}

// The eyebrow over its plate: its kind with its numbers, "PNG · 2400 ×
// 1600", "PDF · 12 pages", "MP4 · 3:41", "Markdown · 4 lines", a
// picture's numbers only when the picture is the file itself (`path`); a
// folder's "Folder"; anything else its kind alone.
function eyebrow(said, path) {
    if (said.items !== undefined) {
        return "Folder";
    }
    const kind = kindOf(said);
    if (said.pages !== undefined) {
        return `${kind}${BETWEEN}${counted(said.pages, "page", "pages")}`;
    }
    if (said.seconds !== undefined) {
        return `${kind}${BETWEEN}${length(said.seconds)}`;
    }
    if (said.picture === path && said.across) {
        return `${kind}${BETWEEN}${said.across}${KEPT}\u00d7${KEPT}${said.down}`;
    }
    if (said.count !== undefined) {
        return `${kind}${BETWEEN}${counted(said.count, "line", "lines")}`;
    }
    return kind;
}

// Its first fact: a folder's count, the number alone since its label says
// what it counts, null until it's said; or a file's size, null while it's
// unknown.
function firstFact(folder, size, said) {
    if (folder) {
        return { label: "Items", value: said ? String(said.items) : null };
    }
    return { label: "Size", value: size === undefined ? null : weighed(size) };
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

// The drawer's plate for a file or a folder, `file` its path, name,
// folder from ~ (where) and what its folder's read said of it (entry):
// the eyebrow, its name, its preview, then its facts, Size or Items,
// Changed and Where. Until file-preview has said (`said` null), it's its
// facts from the folder read alone, so they draw at once; what's not
// known yet is null, shown as the drawer's placeholder.
function plate(file, said, now) {
    const { path, name, where, entry } = file;
    const folder = entry?.folder === true || said?.items !== undefined;
    const changed = said?.changed ?? entry?.modified;
    const shown = {
        eyebrow: "",
        title: name,
        facts: [
            firstFact(folder, said?.size ?? entry?.size, said),
            // Said as its row says it, so the two never read as two dates.
            { label: "Changed", value: changed === undefined ? null : when(changed, now) },
            { label: "Where", value: where }
        ]
    };
    if (said) {
        shown.eyebrow = eyebrow(said, path);
    } else if (folder) {
        shown.eyebrow = "Folder";
    }
    const preview = previewOf(said);
    if (preview) {
        shown.preview = preview;
    }
    return shown;
}
