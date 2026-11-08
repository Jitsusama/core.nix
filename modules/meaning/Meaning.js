// What Meaning.qml works out, kept apart from it so tests/meaning.qml
// proves it in Qt's own engine: which of the launcher's things the meaning
// daemon is told and as what text, how what's typed is asked, the rows an
// answer adds below everything typing found, and the pictures one adds
// below those Capture's Recent found by their names and words.
.pragma library

// The kinds whose rows come and go or are renamed as Joel works, by their
// keys' starts: a window's title, a session, a meeting, a notice, a value
// read. Each would be embedded again with every change for a row that is
// found by its name anyway, so only what stays is told: programs, actions,
// places, settings, collections.
const CHANGING = ["agent:", "agents:", "attachment:", "calendar:", "copies:", "copy:",
    "drive:", "fact:", "gdoc:", "group:", "job:", "mail:", "mine:", "notice:",
    "orchestrator:", "participant:", "person:", "picture:", "quest:", "slack:", "spend:",
    "spot:", "talk:", "time:", "today:", "transcript:", "window:", "workspace:"];

// The tier a row found by its meaning is placed in, past every tier a row
// found by its letters takes (Rank.js's, the loosest 2.5), so a collection
// gathered late is placed above it too: meaning only ever adds below.
const TIER = 3;

// The fewest letters asked about, as screen-text's search: a letter or two
// means nearly anything.
const FEWEST = 3;

// The spaces in what's typed, however many, each asked as one.
const SPACES = /\s+/;

// Whether a thing is told to the daemon: one that stays, and not a row
// that is itself a meaning's.
function told(choice) {
    const key = String(choice?.key ?? "");
    return key !== "" && choice.byMeaning !== true
        && !CHANGING.some(start => key.startsWith(start));
}

// The text a thing's meaning is made from: its name, and the words it is
// known by beside it (its keywords hold its name first, and the
// launcher's synonyms after), so "dim" finds Brightness by "backlight".
function textOf(choice) {
    const known = String(choice?.keywords?.text ?? "").trim();
    return known !== "" ? known : String(choice?.name ?? "").trim();
}

// Every thing told, as the daemon's know takes them: [{ key, text }], one
// a key, in the launcher's order.
function things(choices) {
    const seen = new Set();
    const out = [];
    for (const choice of choices ?? []) {
        if (!told(choice) || seen.has(choice.key)) {
            continue;
        }
        const text = textOf(choice);
        if (text === "") {
            continue;
        }
        seen.add(choice.key);
        out.push({ key: choice.key, text });
    }
    return out;
}

// What's typed as it's asked and kept: lower case, its spaces single.
function asked(letters) {
    return String(letters ?? "").trim().toLowerCase().split(SPACES).join(" ");
}

// Whether words are enough to ask about.
function enough(words) {
    return String(words).replace(/ /g, "").length >= FEWEST;
}

// The rows an answer adds, nearest first: each record's thing as the
// launcher keys it, a copy saying in the foot how it was found, but none
// already shown (`shown`, a set of identities, as Rank.identity makes
// them) and none the launcher no longer holds, at most `how.most`, each
// with `how.note`.
function rows(records, keyed, shown, how) {
    const out = [];
    const taken = new Set(shown);
    for (const record of records ?? []) {
        if (out.length >= how.most) {
            break;
        }
        const choice = keyed?.[record?.key];
        if (!choice) {
            continue;
        }
        const id = choice.is ?? choice.key;
        if (taken.has(id)) {
            continue;
        }
        taken.add(id);
        out.push(Object.assign({}, choice, { note: how.note, byMeaning: true, tier: TIER }));
    }
    return out;
}

// The pictures an answer adds, nearest first, by their paths: none
// already listed (`listed`, a set of paths) and none twice, at most
// `most`.
function paths(records, listed, most) {
    const out = [];
    const taken = new Set(listed);
    for (const record of records ?? []) {
        if (out.length >= most) {
            break;
        }
        const path = record?.key;
        if (record?.kind !== "picture" || typeof path !== "string" || taken.has(path)) {
            continue;
        }
        taken.add(path);
        out.push(path);
    }
    return out;
}
