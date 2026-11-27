// How the launcher's journal says an act and what came of it with reports off (Options.reports):
// its key, a daemon's refusal, code and error, whom and whence a link came from, the rows a walk
// took, and the shell's own failure. Apart from Presented.js, whose words it builds on, so each
// stays one file biome can hold to its length, and tests/pacing.qml holds both in Qt's engine.
.pragma library
.import "KeysSaid.js" as KeysSaid
.import "Presented.js" as Presented

// An act's key as its lines say it: as rowShown says a row's, or a daemon's own act of a kind, as
// `displays:sleep`, by its kind and act where its act is one of KeysSaid's actWords, never
// more, though a key of one word, perhaps a record's name, as a row; and a record's, a window's
// or a verb's as keyShown says it; bare, as the desktop's tests read it, with what quoting
// escapes escaped, so a line break can't forge another line.
function actShown(key, whole) {
    const said = Presented.rowShown({ key }, whole);
    const parts = String(key).split(":").slice(0, 2);
    const kinds = whole || KeysSaid.keysSaidByKind.test(String(key)) || parts.length < 2 ? ""
        : Presented.kindsLed(parts, 1).join(":");
    return Presented.quoted(kinds.length > said.length ? kinds : said).slice(1, -1);
}

// A refusal come after its line was done, as the journal says it: the daemon's code where it gave
// one (codeShown), whatever the reports setting, since it names no one and tells a daemon refusing
// from one gone, then its words quoted while reports are on, otherwise by how many letters.
function refusalShown(why, code, whole) {
    return `${code ? `${code}, ` : ""}${Presented.typedShown(why, whole)}`;
}

// A daemon's code for a refusal, as wifid's WrongPassword, said whole whatever the reports setting
// since it names no one, escaped as actShown escapes a key; "no answer" where none came, and
// "no code" where one came without.
function codeShown(answer) {
    const code = answer ? (answer.error?.refused ?? answer.error?.code ?? "no code") : "no answer";
    return Presented.quoted(String(code)).slice(1, -1);
}

// The fields of a daemon's error that are codes, no one's words, so said whole whatever the
// reports setting: its code, its kind, and a refusal's own, as wifid's WrongPassword.
const errorCodes = new Set(["code", "kind", "refused"]);

// A daemon's error as a line in the journal may say it, quoted as JSON: whole where `whole`, and
// otherwise its codes, numbers and truths whole and each of its words as typedShown says typed
// words, since a daemon's own words can name a row, as "couldn't join Harbour House" (Y2YBTS,
// 2026-11-20, 41ca61c4). An error that's words alone, against the contract, is said as those are.
function errorShown(error, whole) {
    if (whole || error === null || typeof error !== "object") {
        const words = typeof error === "string" && !whole;
        return Presented.quoted(words ? Presented.typedShown(error, false) : (error ?? null));
    }
    const told = {};
    for (const [field, value] of Object.entries(error)) {
        const plain = value === null || typeof value === "number" || typeof value === "boolean";
        told[field] = errorCodes.has(field) || plain ? value
            : Presented.typedShown(typeof value === "string" ? value : JSON.stringify(value), false);
    }
    return Presented.quoted(told);
}

// Whom a link from outside named, as TalkTo.js's whom gives it: whole where `whole`, and
// otherwise by its kind and how many letters the name it gave has, since a page wrote it.
function whomShown(whom, whole) {
    if (whole) {
        return Presented.quoted(whom);
    }
    const named = whom?.address ?? whom?.name ?? whom?.said;
    return named === undefined ? String(whom?.to)
        : `${whom?.to}, ${Presented.typedShown(named, false)}`;
}

// The session new work a link offered is from, by its address as Sprouting.js's offered gives it:
// whole where `whole`, and otherwise by how many letters, as whomShown says a link's session, or
// as none where the link named no session here. Sprouting.js gives only an address it found, but
// it's escaped as actShown's are all the same, so no address could break the line.
function whenceShown(address, whole) {
    if (address === undefined) {
        return "from no session here";
    }
    return whole ? `from ${Presented.quoted(String(address)).slice(1, -1)}`
        : `from a session, ${Presented.typedShown(address, false)}`;
}

// The rows a walk from outside or a terminal took, by their names where `whole`, and otherwise
// each by its kind as rowShown says it, since a source named them, escaped as actShown's are,
// though a key said whole is an id's letters alone (keyShown), so that's only a fail-safe.
function pathShown(rows, whole) {
    return whole ? Presented.quoted(rows.map(row => row.name).join(" \u203a "))
        : Presented.quoted(rows.map(row => Presented.rowShown(row, false)).join(" \u203a "))
            .slice(1, -1);
}

// The shell's own failure, as the engine threw it: whole where `whole`, and otherwise by its
// error's name, since its message may quote what it failed on, or by letters where it has none.
const errorName = /^[A-Z][A-Za-z]*$/;
function failureShown(error, whole) {
    if (whole) {
        return Presented.quoted(String(error));
    }
    const name = error?.name;
    return errorName.test(name ?? "") ? name : Presented.typedShown(String(error), false);
}
