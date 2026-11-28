// The desktop's kinds of file, for the launcher's chooser of files
// (FileChooser.qml): the table shared-mime-info's files give, read into
// one, and whether a program's filter passes a file by a glob or by its
// kind. Plain JavaScript apart from the chooser, which holds the table and
// the filter in use and gives them to each function here, as FileFacts.js
// is given what it says; the chooser's own check
// (the-launcher-chooses-files-as-asked) holds it, a filter passing a file
// by a kind it's one of.
//
// The table is { names, endings, broader }: a kind by a whole name, a kind
// by an ending, and the kinds each kind is one of.
.pragma library

// How many kinds broader a kind is followed to, so a table that says a
// kind is one of its own never loops.
const DEEPEST = 8;
// What a glob says literally that a pattern would read otherwise, its
// wildcards, and a glob that's only an ending ("*.md") or holds none.
const LITERAL = /[.+^${}()|\\]/gu;
const ANY = /\*/gu;
const ONE = /\?/gu;
const ENDING = /^\*\.[^*?[]+$/u;
const WILD = /[*?[]/u;
const SPACES = /\s+/u;

// Whether the filter in use passes a file by its name: by a glob, or by
// its kind, as the desktop's kinds name files. A file whose kind is said
// (`mime`), as one kept elsewhere is, is known by that, else by its name.
// With no filter in use, every file passes.
function wants(kinds, filter, name, mime) {
    if (filter === null) {
        return true;
    }
    const kind = mime || kindOf(kinds, name);
    return (filter.globs ?? []).some(glob => globbed(glob).test(name))
        || kind !== "" && (filter.kinds ?? []).some(wanted => isKind(kinds, kind, wanted, 0));
}

function globbed(glob) {
    const said = glob.replace(LITERAL, "\\$&").replace(ANY, ".*").replace(ONE, ".");
    // A program's glob is read as a pattern always read it: with Unicode's
    // flag, Qt's engine folds letters further, a Kelvin sign passing as a
    // k (modules/quickshell/tests/file-kinds.qml).
    // biome-ignore lint/style/useUnicodeRegex: a program's glob is read as it always was.
    return new RegExp(`^${said}$`, "i");
}

// A file's kind, by its whole name, else by its longest ending.
function kindOf(kinds, name) {
    const lower = name.toLowerCase();
    if (kinds.names[name] || kinds.names[lower]) {
        return kinds.names[name] ?? kinds.names[lower];
    }
    for (let at = lower.indexOf("."); at >= 0; at = lower.indexOf(".", at + 1)) {
        const kind = kinds.endings[lower.slice(at)];
        if (kind) {
            return kind;
        }
    }
    return "";
}

// Whether a kind is the one wanted, a kind of it ("image/*" holds
// image/png), or one the desktop says it's a kind of (text/markdown is
// text/plain).
function isKind(kinds, kind, wanted, depth) {
    if (kind === wanted || wanted.endsWith("/*") && kind.startsWith(wanted.slice(0, -1))) {
        return true;
    }
    return depth < DEEPEST
        && (kinds.broader[kind] ?? []).some(each => isKind(kinds, each, wanted, depth + 1));
}

// The desktop's kinds of file (shared-mime-info's globs2, each line a
// weight, a kind and a glob, the heaviest first), by a whole name or an
// ending, in place of those the table held.
function kindsFrom(kinds, text) {
    const names = {};
    const endings = {};
    for (const line of text.split("\n")) {
        const [, kind, glob] = line.split(":");
        if (line.startsWith("#") || !(kind && glob)) {
            // A comment, or a line saying no kind and glob, names nothing.
        } else if (ENDING.test(glob)) {
            // The first a name or an ending is given is the heaviest.
            const ending = glob.slice(1).toLowerCase();
            if (!(ending in endings)) {
                endings[ending] = kind;
            }
        } else if (!(WILD.test(glob) || glob in names)) {
            names[glob] = kind;
        }
    }
    return Object.assign({}, kinds, { names, endings });
}

// The kinds each kind is one of (shared-mime-info's subclasses): a line a
// kind, then the broader one, in place of those the table held.
function broaderFrom(kinds, text) {
    const broader = {};
    for (const line of text.split("\n")) {
        const [kind, wider] = line.trim().split(SPACES);
        if (kind && wider) {
            broader[kind] = (broader[kind] ?? []).concat([wider]);
        }
    }
    return Object.assign({}, kinds, { broader });
}
