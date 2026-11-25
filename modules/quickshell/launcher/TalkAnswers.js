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

// The spaces after a whole question.
const SPACE_AFTER = /\s+$/u;

// What a want asks in full, as the agents daemon passes on the harness's whole question
// (wants.rs's `whole`: its paragraphs, list lines and indentation kept), its blank lines
// before and its spaces after let go; "" where it says no more than its short line. Its
// blank lines are let go line by line, not by a pattern: an anchored /^\s*\n/u kept a
// leading newline in one engine the checks run.
function whole(want) {
    const lines = String(want?.whole ?? "").split("\n");
    const first = lines.findIndex(line => line.trim() !== "");
    return first < 0 ? "" : lines.slice(first).join("\n").replace(SPACE_AFTER, "");
}

// What the session asks, in TalkLine.js's exchange shape, as the talk's first: who asks,
// muted, with since when at its right, then what it asks, whole where the want says more than
// its short line, and what it takes where it offers answers, from `now`'s want while it's
// `asking` (Agents.qml's answersOf). Null while it asks nothing, or says nothing of what nor
// offers answers: the daemon's `asked` is then the session's title, or blank, which says
// nothing it asks.
function question(now, name, words, dayOf) {
    const want = now?.want ?? null;
    const asking = now?.asking === true;
    const what = whole(want) || String(want?.asked ?? "").trim();
    const untold = whole(want) === "" && String(want?.said ?? "").trim() === ""
        && what === String(want?.title ?? "");
    const answers = asked(want, asking);
    const unsaid = untold || what === "";
    if (!asking || (unsaid && answers === null)) {
        return null;
    }
    // What it takes goes in `takes`, apart from the question, which TalkSpoken.qml draws
    // whole under it, wrapped and never elided, so his Return never answers choices he
    // can't see; one that says nothing of what it asks still says what it takes.
    return { line: filled(words.asks, { name: String(name ?? "") }),
        answers: unsaid ? [] : [what], waiting: "", stands: "", by: "",
        takes: answers === null ? "" : takes(answers, words),
        side: sideOf(want?.since, dayOf) };
}

// An answer said in this shell to the want `want`: his words, when, and how it stands:
// "sent" until the node says whether it put it in the session's inbox, then "answered", or
// "failed" with why, or "unheard" where nothing said whether it went.
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
// while it asks one; else, once it asks nothing (""), those said in the last SHOWN_AFTER_MS,
// so one long done, or to an ask since replaced, never leads a later Talk. One not delivered,
// or not known to be, in that time shows under any ask, a null `want` (one he can't answer
// here) among them, since the node refuses an answer to an ask since replaced, and the
// refusal is shown under the ask that replaced it.
function standing(list, want, nowMs) {
    const recent = one => nowMs - Date.parse(one.at) < SHOWN_AFTER_MS;
    const unput = one => one.stage === "failed" || one.stage === "unheard";
    return (list ?? []).filter(one => (want === "" ? recent(one)
        : one.want === want || (unput(one) && recent(one))));
}

// The `want` standing takes for a Talk drawing `answers` (asked's) of what a session asks
// `now` (Agents.qml's answersOf): the id of what it asks while it asks one he can answer
// here, null while it asks one he can't, and "" once it asks nothing.
function shownWant(now, answers) {
    return now?.asking === true ? answers?.want ?? null : "";
}

// What Joel's Return in the Talk kept as `draft` answers: what that Talk last drew it asking
// (`drawnBy`, by draft) while it's the Talk held (`heldDraft`), so he answers what he was shown
// rather than what's held by the time he presses it; else, a walk from a terminal drawing
// none, what it asks `now`. Null where it asks nothing he can answer, and his words go as a
// talk.
function answering(heldDraft, draft, drawnBy, now) {
    const held = heldDraft === draft && draft in (drawnBy ?? {});
    return held ? drawnBy[draft] : now;
}

// What a Talk drew its Return answering: `answers` (asked's) where it drew the question
// (`drawnQuestion`, question's), null where it drew none, so he never answers a question he
// wasn't shown, and his words go as a talk.
function drawn(drawnQuestion, answers) {
    return drawnQuestion === null || drawnQuestion === undefined ? null : answers ?? null;
}

// What Joel's Return in a Talk does with his words, given what they answer (`answering`'s):
// `{ act: "talk" }` where they answer nothing, and go to the session's inbox;
// `{ act: "answer", answers }` where they answer what it asks and come from his own Return
// (`fromJoel`, Sprouting.js's); `{ act: "refused" }` where they'd answer it from anywhere
// else, since an answer goes as his.
function onReturn(answers, fromJoel) {
    if (answers === null || answers === undefined) {
        return { act: "talk" };
    }
    return fromJoel === true ? { act: "answer", answers } : { act: "refused" };
}

// Whether a session's answers (`list`) hold one of his to `want` sent or answered, so a second
// Return isn't shown as going: the session takes the first answer to an ask, and one after it
// finds the ask gone (the harness's answerSteered). One not delivered, or not known to be,
// leaves it his to answer again.
function answeredAlready(list, want) {
    return (list ?? []).some(one => one.want === String(want ?? "")
        && (one.stage === "sent" || one.stage === "answered"));
}

// Whether his words to `answers` stay to be given back once the node has said of them: only
// where they didn't go (`went`) and the session still asks what they answer (`now`, asked's),
// so given back they never answer a newer ask.
function keepsWords(went, now, answers) {
    return went !== true && now !== null && now !== undefined && now.want === answers?.want;
}

// The words of a session's answers (`list`) kept to be given back that no longer answer what
// it asks, `want` as standing takes it: those not delivered, or not known to be, to another
// want. A refusal can come before the want that replaced its ask, so they're let go once
// that want is drawn, or given back they'd answer it.
function stale(list, want) {
    return (list ?? []).filter(one => (one.stage === "failed" || one.stage === "unheard")
        && one.want !== want).map(one => one.words);
}

// What the node's put made of the answer `one`: answered where it went, failed with `why`
// where it didn't, and unheard where nothing came back to say (`unheard`), the agents daemon
// gone while it was asked, so it may have gone or not.
function put(one, went, why, unheard) {
    if (!went && unheard === true) {
        return Object.assign({}, one, { stage: "unheard", why: "" });
    }
    return Object.assign({}, one, went ? { stage: "answered", why: "" }
        : { stage: "failed", why: String(why ?? "") });
}

// How an answer of his reads under the field: his words, with when at their right; sent
// until the node puts it; then, breathing, working while the session `working` works on, else
// answered; or not delivered, and why; or not known to have arrived.
function shown(one, working, words, dayOf) {
    const told = { line: one.words, answers: [], waiting: "", stands: "", by: "",
        side: sideOf(one.at, dayOf) };
    if (one.stage === "failed") {
        const why = String(one.why ?? "");
        return Object.assign(told, { stands: [String(words.unsent ?? "")]
            .concat(why === "" ? [] : [why]).join(" \u00b7 ") });
    }
    if (one.stage === "sent" || one.stage === "unheard") {
        return Object.assign(told, { stands: String(words[one.stage] ?? "") });
    }
    return Object.assign(told, working === true ? { waiting: String(words.working ?? "") }
        : { stands: String(words.answered ?? "") });
}
