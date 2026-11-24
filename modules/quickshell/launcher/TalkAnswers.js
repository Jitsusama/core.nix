// Talk to It on a session that asks Joel something (Agents.qml's talked): what it asks shows
// in full as the talk's first exchange, and his Return answers it, rather than going to an
// inbox the session doesn't read while it asks (Joel, 13:15 on 2026-11-24, of a cell asking
// him from the realm's map: "talking to the agent is worse than talking to the void via that
// interface. no feedback at all"). His answer then reads sent, answered once the node has put
// it in the session's inbox, working while the session works on, or why it never went.
// tests/talk-answers.qml holds it to its cases in Qt's own engine.
.pragma library

.import "../TalkLine.js" as Said

// How long an answer still shows once its session asks nothing, a quarter hour, so a Talk
// opened later doesn't lead with an answer long done; and how many are kept for a session.
const SHOWN_AFTER_MS = 900000;
const KEPT = 3;

// What a session asks that Joel's words to it answer: its want's answers as the agents
// daemon serves them (modules/agents/src/wants.rs), while it runs `asking` him. Null for a
// want he can't answer here, one with no choices or no id, or a session not asking, parked
// among them, whose words go as a talk.
function asked(want, asking) {
    const answers = want?.answers ?? null;
    const answerable = answers !== null && typeof answers === "object"
        && typeof answers.want === "string" && answers.want !== ""
        && Array.isArray(answers.choices) && answers.choices.length > 0;
    return asking === true && answerable ? answers : null;
}

// How `text` answers `answers`: the choice it names by its label, else by its value, in any
// case, as `{ value, label }`; else his own words as `{ other, label }` where it takes them;
// null where it takes neither, or nothing was said.
function answerOf(text, answers) {
    const spoken = String(text ?? "").trim();
    if (spoken === "") {
        return null;
    }
    const folded = spoken.toLowerCase();
    const by = field => answers.choices
        .find(one => String(one[field] ?? "").toLowerCase() === folded);
    const named = by("label") ?? by("value");
    if (named) {
        return { value: String(named.value), label: String(named.label ?? named.value) };
    }
    return answers.own === true ? { other: spoken, label: spoken } : null;
}

// `template` with `{key}` put in from `fills`, each as written, a `$` in it included.
function filled(template, fills) {
    return Object.keys(fills).reduce((text, key) => text.split(`{${key}}`).join(fills[key]),
        String(template ?? ""));
}

// What `answers` takes, as one line: its choices' labels in its order, the one it recommends
// first, and whether his own words go too (`words.choices`, else `words.only`).
function takes(answers, words) {
    const choices = answers.choices.map(one => String(one.label ?? one.value)).join(" \u00b7 ");
    return filled(answers.own === true ? words.choices : words.only, { choices });
}

// When `at` was, as a talk's side says it: its clock, after its day where `dayOf` names one,
// as SessionTalks.js's spoken does; nothing for a time it can't read.
function sideOf(at, dayOf) {
    const since = Date.parse(String(at ?? ""));
    if (Number.isNaN(since)) {
        return "";
    }
    const day = typeof dayOf === "function" ? dayOf(since) : "";
    return day === "" ? Said.clockOf(since) : `${day} ${Said.clockOf(since)}`;
}

// What the session asks, in TalkLine.js's exchange shape, as the talk's first: who asks,
// muted, with since when at its right, then what it asks in full and what it takes where it
// offers answers, from `now`'s want while it's `asking` (Agents.qml's answersOf). Null while
// it asks nothing, or says nothing of what: the daemon's `asked` is then the session's title,
// which says nothing it asks.
function question(now, name, words, dayOf) {
    const want = now?.want ?? null;
    const asking = now?.asking === true;
    const what = String(want?.asked ?? "").trim();
    const untold = String(want?.said ?? "").trim() === "" && what === String(want?.title ?? "");
    if (!asking || what === "" || untold) {
        return null;
    }
    const answers = asked(want, asking);
    return { line: filled(words.asks, { name: String(name ?? "") }),
        answers: [what].concat(answers === null ? [] : [takes(answers, words)]),
        waiting: "", stands: "", by: "", side: sideOf(want?.since, dayOf) };
}

// An answer said in this shell to the want `want`: his words, when, and how it stands:
// "sent" until the node says whether it put it in the session's inbox, then "answered", or
// "failed" with why.
function said(text, at, want) {
    return { at: new Date(at).toISOString(), want: String(want ?? ""),
        words: String(text ?? "").trim(), stage: "sent", why: "" };
}

// A session's answers, newest first, once `one` is said or has moved on: in its place where
// it's there already, else first; the newest KEPT, so one dropped never comes back.
function kept(list, one) {
    const same = other => other.at === one.at && other.want === one.want;
    const all = list ?? [];
    if (all.some(same)) {
        return all.map(other => (same(other) ? one : other));
    }
    return one.stage === "sent" ? [one].concat(all).slice(0, KEPT) : all;
}

// Which of a session's answers its Talk shows at `nowMs`: those to the want it asks, `want`,
// while it asks one; else, once it asks nothing, those said in the last SHOWN_AFTER_MS, so
// one long done, or to an ask since replaced, never leads a later Talk.
function standing(list, want, nowMs) {
    return (list ?? []).filter(one => (want === "" ? nowMs - Date.parse(one.at) < SHOWN_AFTER_MS
        : one.want === want));
}

// What the node's put made of the answer `one`: answered where it went, failed with `why`
// where it didn't.
function put(one, went, why) {
    return Object.assign({}, one, went ? { stage: "answered", why: "" }
        : { stage: "failed", why: String(why ?? "") });
}

// How an answer of his reads under the field: his words, with when at their right; sent
// until the node puts it; then, breathing, working while the session `working` works on, else
// answered; or not delivered, and why.
function shown(one, working, words, dayOf) {
    const told = { line: one.words, answers: [], waiting: "", stands: "", by: "",
        side: sideOf(one.at, dayOf) };
    if (one.stage === "failed") {
        const why = String(one.why ?? "");
        return Object.assign(told, { stands: [String(words.unsent ?? "")]
            .concat(why === "" ? [] : [why]).join(" \u00b7 ") });
    }
    if (one.stage === "sent") {
        return Object.assign(told, { stands: String(words.sent ?? "") });
    }
    return Object.assign(told, working === true ? { waiting: String(words.working ?? "") }
        : { stands: String(words.answered ?? "") });
}
