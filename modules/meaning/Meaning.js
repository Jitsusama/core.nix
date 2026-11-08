// What Meaning.qml works out, kept apart from it so tests/meaning.qml
// proves it in Qt's own engine: which of the launcher's things the meaning
// daemon is told and as what text, how what's typed is asked, the rows an
// answer adds below everything typing found, landing on the part a place
// holds rather than the place, and the pictures one adds below those
// Capture's Recent found by their names and words.
.pragma library

// The kinds whose rows come and go or are renamed as Joel works, by their
// keys' starts: a window's title, a session, a meeting, a notice, a value
// read. Each would be embedded again with every change for a row that is
// found by its name anyway, so only what stays is told: programs, actions,
// places, settings, collections, and the parts a collection's devices
// hold, its spots, as a screen's Brightness, so a meaning lands on the
// part itself. An answer's parts, as Lag's "niri took 279 ms to answer",
// are named for what they read, which changes as it's read, so they
// aren't.
// Kept as a set of each key's start up to its first colon, since a walk
// testing every start of every key cost the shell's thread 62 ms over a
// thousand things in Qt's engine (2026-11-08).
const CHANGING = ["agent:", "agents:", "attachment:", "calendar:", "copies:", "copy:",
    "drive:", "fact:", "gdoc:", "group:", "job:", "mail:", "mine:", "notice:",
    "orchestrator:", "participant:", "person:", "picture:", "quest:", "slack:", "spend:",
    "talk:", "time:", "today:", "transcript:", "window:", "workspace:"]
    .reduce((starts, start) => {
        starts[start] = true;
        return starts;
    }, {});

// How a spot's key starts (Launcher.qml's spotOf): a part a level inside a
// thing a place holds, found from the root, its trail naming each step.
const SPOT = "spot:";

// How a collection's identity starts, on a spot's trail through one.
const COLLECTION = "collection:";

// What stands between the items of a note, as Reading.js joins them.
const ITEMS_APART = " \u00b7 ";

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
        && CHANGING[key.slice(0, key.indexOf(":") + 1)] !== true
        && (!isSpot(choice) || choice.trail.some(id => String(id).startsWith(COLLECTION)));
}

// The text a thing's meaning is made from: its name, and the words it is
// known by beside it (its keywords hold its name first, and the
// launcher's synonyms after), so "dim" finds Brightness by "backlight".
function textOf(choice) {
    const known = String(choice?.keywords?.text ?? "").trim();
    return known !== "" ? known : String(choice?.name ?? "").trim();
}

// Every thing told, as the daemon's know takes them: [{ key, text }], one
// a key, in the launcher's order, its spots after its choices.
function things(choices, spots) {
    return telling(choices, spots).things;
}

// Whether a row is a spot, which stands for a part inside a place.
function isSpot(row) {
    return String(row?.key ?? "").startsWith(SPOT) && Array.isArray(row.trail);
}

// Rows by their keys, as Launcher.qml's keyed holds its choices, for the
// spots, which aren't among them. Walked by index, as `telling` is.
function keyedBy(listed) {
    const out = {};
    const all = listed ?? [];
    for (let at = 0; at < all.length; at += 1) {
        const key = all[at]?.key;
        if (typeof key === "string" && key !== "") {
            out[key] = all[at];
        }
    }
    return out;
}

// The things told, and `as`, one text standing for them all, so the shell
// tells them again only when it changes. It's walked by index and seen in
// a plain object, which Qt's engine does in a tenth of the time an
// iterator and a Set take, and `as` is joined rather than stringified.
function telling(choices, spots) {
    const seen = {};
    const out = [];
    const lines = [];
    const all = (choices ?? []).concat(spots ?? []);
    for (let at = 0; at < all.length; at += 1) {
        const choice = all[at];
        if (!told(choice) || seen[choice.key] === true) {
            continue;
        }
        const text = textOf(choice);
        if (text === "") {
            continue;
        }
        seen[choice.key] = true;
        out.push({ key: choice.key, text });
        lines.push(`${choice.key}\t${text}`);
    }
    return { things: out, as: lines.join("\n") };
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
// launcher keys it, or its spot as `how.spots` keys them, a copy saying in
// the foot how it was found, but none already shown (`shown`, a set of
// identities, as Rank.identity makes them) and none the launcher no longer
// holds, at most `how.most`, each noting `how.note` before its own note,
// so it says where it is as when typed; a note gives way from its end, so
// how it was found always stays.
// A meaning lands on the part rather than the place it lives in (VKCF3Z,
// 2026-11-08): a spot found stands where a place on its trail was found,
// and that place isn't added after it. A spot's name is meant once, so
// "too loud" is one Volume rather than every device's, the nearest.
function rows(records, keyed, shown, how) {
    const found = [];
    const taken = new Set(shown);
    const named = {};
    for (const record of records ?? []) {
        const key = record?.key;
        const choice = keyed?.[key] ?? how.spots?.[key];
        if (!choice) {
            continue;
        }
        const id = choice.is ?? choice.key;
        if (taken.has(id)) {
            continue;
        }
        if (isSpot(choice)) {
            if (named[choice.name] === true) {
                continue;
            }
            named[choice.name] = true;
            taken.add(id);
            const place = found.findIndex(row => choice.trail.includes(row.is ?? row.key));
            if (place >= 0) {
                found[place] = choice;
            } else {
                found.push(choice);
            }
            continue;
        }
        taken.add(id);
        if (!found.some(row => isSpot(row) && row.trail.includes(id))) {
            found.push(choice);
        }
    }
    return found.slice(0, how.most).map(choice => {
        const note = [how.note, choice.note ?? ""].filter(item => item !== "").join(ITEMS_APART);
        return Object.assign({}, choice, { note, byMeaning: true, tier: TIER });
    });
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
