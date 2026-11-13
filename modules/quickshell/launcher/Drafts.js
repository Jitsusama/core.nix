.pragma library

// Joel's words in a talk's field, kept until they've reached whoever he
// sent them to, so a shell started again, as a switch does, gives back
// what he was typing (Joel, 2026-11-12 16:00: "had it disappear through a
// quickshell switch losing what i was typing"; QCXZEB, e23364f8). Each is
// kept by the address it goes to: `drafts` holds what's still being
// typed, with its caret, and `sending` what Return sent, which stays until
// its talk says it was delivered, never cleared on Return itself, since a
// switch between Return and delivery would lose it otherwise. Launcher.qml
// keeps the state and writes it (Drafts.qml); these say what it becomes.

// How long a draft is given back for, so one left a week ago doesn't
// come back to a field asking something else.
const daysKept = 7;
const keptFor = daysKept * 24 * 60 * 60 * 1000;

function empty() {
    return { drafts: {}, sending: {} };
}

// What a field shows as it opens for an address: what was being typed,
// else the newest words sent and not yet delivered, the caret at their
// end; or nothing, for either kept a week or longer.
function kept(state, key, now) {
    const draft = state.drafts[key];
    if (draft && now - draft.at < keptFor) {
        return { text: draft.text, caret: Math.min(draft.caret, draft.text.length) };
    }
    const waiting = (state.sending[key] ?? []).filter(one => now - one.at < keptFor);
    const newest = waiting.length > 0 ? waiting[waiting.length - 1] : null;
    return newest ? { text: newest.text, caret: newest.text.length } : { text: "", caret: 0 };
}

// The state once the field for `key` reads `field.text`, its caret at
// `field.caret`: emptied, the draft goes, as nothing's left to keep.
function typed(state, key, field, now) {
    const drafts = Object.assign({}, state.drafts);
    if (field.text === "") {
        delete drafts[key];
    } else {
        drafts[key] = { text: field.text, caret: field.caret, at: now };
    }
    return { drafts, sending: state.sending };
}

// The state once Return sent `text` to `key`: the draft becomes words
// waiting to be delivered.
function sent(state, key, text, now) {
    const drafts = Object.assign({}, state.drafts);
    delete drafts[key];
    const sending = Object.assign({}, state.sending);
    sending[key] = (sending[key] ?? []).concat([{ text: text.trim(), at: now }]);
    return { drafts, sending };
}

// The state once the talks say what reached each address (Orchestrators'
// talks, by address in upper case, each { words, stage }): words sent
// that a talk delivered, or anything after delivery, go. A talk that
// failed leaves them kept. The same state, unchanged, when none went, so
// nothing is written for it.
function settled(state, talks) {
    let changed = false;
    const sending = {};
    for (const key of Object.keys(state.sending)) {
        const heard = (talks[key.toUpperCase()] ?? [])
            .filter(talk => talk.stage !== "" && talk.stage !== "failed")
            .map(talk => String(talk.words ?? "").trim());
        const waiting = state.sending[key].filter(one => !heard.includes(one.text));
        changed = changed || waiting.length !== state.sending[key].length;
        if (waiting.length > 0) {
            sending[key] = waiting;
        }
    }
    return changed ? { drafts: state.drafts, sending } : state;
}

// The state from what was written, keeping only what's under a week old;
// none for a file that's missing or isn't one written here.
function read(text, now) {
    let written = null;
    try {
        written = JSON.parse(text || "{}");
    } catch (_error) {
        return empty();
    }
    const state = empty();
    for (const [key, draft] of Object.entries(written?.drafts ?? {})) {
        if (typeof draft?.text === "string" && now - Number(draft.at) < keptFor) {
            state.drafts[key] = { text: draft.text, caret: Number(draft.caret) || 0,
                at: Number(draft.at) };
        }
    }
    for (const [key, words] of Object.entries(written?.sending ?? {})) {
        const fresh = (Array.isArray(words) ? words : []).filter(one =>
            typeof one?.text === "string" && now - Number(one.at) < keptFor);
        if (fresh.length > 0) {
            state.sending[key] = fresh;
        }
    }
    return state;
}
