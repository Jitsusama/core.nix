// Whether an act a daemon serves is whole, and the words an ask act asks
// Joel to type before it's sent, its `fields` (ready-rows.md's acts; F8TT8O
// and Y2YBTS, 2026-11-13, 1ace0a22): where each answer goes in the ask.
// ReadyRows.js does no act these say is misformed, ReadyRows.qml asks the
// fields, and tests/ready-rows.qml runs them in Qt's own engine.
.pragma library

// How long Return waits for a copy's words it asks for before it says none
// came, in ms: a daemon answers from what it holds in a few, so this is only
// reached when it's stuck or busy, as behind an act its doer is still doing on
// the same connection, and then said rather than left silent.
const COPY_WAIT = 2000;

function asks(act) {
    return act?.ask !== null && typeof act?.ask === "object";
}

// Why an act can't be done as it's served, or "" where it can. One that
// shuts the launcher holds nothing drawn, so it can't say what its row
// becomes. Only an ask asks for words or says how it goes as it goes
// (`streams`), each word asked by a words key and set at a place in the ask,
// and one that asks for words leaves the launcher only once they're sent.
function misformed(act) {
    if (act?.closes === true && act.becomes !== undefined) {
        return "closes and becomes";
    }
    if (act?.fields !== undefined && !(asks(act) && Array.isArray(act.fields))) {
        return "fields without an ask";
    }
    const named = text => typeof text === "string" && text !== "";
    const whole = field => named(field?.ask) && named(field?.into);
    if ((act?.fields ?? []).some(field => !whole(field))) {
        return "a field without its ask or into";
    }
    if (act?.fields !== undefined && act.closes === true) {
        return "closes and asks for words";
    }
    return act?.streams !== undefined && !asks(act) ? "streams without an ask" : "";
}

// The words an ask act asks for before it's sent, in the order asked, each
// `{ ask, into, secret }`: `ask` the words key its prompt is read from and
// `into` where in the ask the answer goes. None for an act that asks for
// none, or that is misformed.
function fieldsOf(act) {
    return asks(act) && misformed(act) === "" && Array.isArray(act.fields) ? act.fields : [];
}

// The ask with `answer` set at `into`, a dotted path, as `account.identity`,
// each object along it copied so the ask served is never changed.
function answeredInto(ask, into, answer) {
    const path = into.split(".");
    const top = Object.assign({}, ask);
    let at = top;
    for (const step of path.slice(0, -1)) {
        const below = at[step];
        at[step] = below !== null && typeof below === "object" ? Object.assign({}, below) : {};
        at = at[step];
    }
    at[path[path.length - 1]] = answer;
    return top;
}

// Whether a copy asks its daemon for its words at Return, `copy: { ask }`,
// rather than carrying them, as a spend report's whole markdown, which
// changes with each of its figures and so was sent again with each
// (ready-rows.md's acts; Y2YBTS, 2026-11-18, 00b48fb4). Carrying them
// stays the rule; asking is only for words that large. Only an ask named
// copy is one, so an ask a daemon names otherwise, as its act, is never sent
// under a copy's name.
function copyAsked(act) {
    return act?.copy !== null && typeof act?.copy === "object" && asks(act.copy)
        && act.copy.ask.ask === "copy";
}

// Asks a copy's words of `link`, the rows' own connection: `copied` is given
// them once answered, `refused` the answer refusing it, or null where none came
// or it held no words (empty ones too), so each end is said. Only the first end is, and the
// function returned ends the wait, said as refused, by the refusal it's given
// (as the launcher closing first) or as no answer, unless it's ended with
// `true`, quietly, as a newer copy overtaking it is; a late answer then copies
// nothing.
function askCopy(act, link, copied, refused) {
    let waiting = true;
    const ends = (then, given) => {
        if (waiting) {
            waiting = false;
            then(given);
        }
    };
    if (link) {
        link.ask(act.copy.ask, answer => {
            if (typeof answer?.copy === "string" && answer.copy !== "") {
                ends(copied, answer.copy);
            } else {
                ends(refused, answer?.error ? answer : null);
            }
        });
    } else {
        ends(refused, null);
    }
    return ending => {
        if (ending === true) {
            waiting = false;
        } else {
            ends(refused, ending?.error ? ending : null);
        }
    };
}

// Where Joel stands in the launcher, each part changing as he moves on:
// what's typed, the steps he's in and the one he's in, the row he's on by
// its key (its place moves as rows come above it), the act he did that kept
// it, whether it's letting go for another copy or a paste, and `clipped`,
// the clipboard's changes. Last, whether visual, an operator or the command
// line holds the rows, which movedOn counts only as begun since.
function standing(launcher, clipped) {
    const held = launcher?.held ?? [];
    return [launcher?.typed, held.length, held[held.length - 1]?.row?.key,
        launcher?.matches?.[launcher?.current]?.key, launcher?.acted,
        launcher?.lettingGo === true, clipped, launcher?.holdingRows === true];
}

// The places of the rows with these keys among `matches` now, those gone
// left out, so a flash after a wait finds its rows where they've moved to.
function placesOf(keys, matches) {
    return keys.map(key => matches.findIndex(row => row.key === key)).filter(at => at >= 0);
}

// Whether Joel moved on while a copy's words were asked for, from where he
// stood as it was asked to where he stands (standing above): any
// part changed, but the last, a state holding the rows, only begun since,
// since y over visual ends it as it copies.
function movedOn(stood, stands) {
    const last = stood.length - 1;
    return stood.some((part, at) => at < last && part !== stands[at])
        || (!stood[last] && Boolean(stands[last]));
}

// What y does over `copies` (Operators.js's copiesIn), `text` their words
// joined (its copied): the words to copy, or else the one copy whose own
// act y runs, as a picture's, what y says, the places of the rows it
// flashes, and those it flashes only once its run has copied (`later`). An
// asked copy (`alone`) holds no words to join, so among others y copies the
// rest, says so, and flashes only what it copied; with none to copy it says
// why and flashes nothing. Alone, its words are asked for, so it flashes
// once they're on the clipboard, never before.
function yanking(copies, text, words) {
    const alone = copy => copy.action.alone === true;
    const among = copies.length > 1 && copies.some(alone);
    if (text !== "") {
        return { text, run: null, says: among ? words.alone : "",
            flashed: copies.filter(copy => !(among && alone(copy))).map(copy => copy.at),
            later: [] };
    }
    if (copies.length === 1) {
        const asked = alone(copies[0]);
        return { text, run: copies[0].action, says: "", flashed: asked ? [] : [copies[0].at],
            later: asked ? [copies[0].at] : [] };
    }
    return { text, run: null, says: among && copies.every(alone) ? words.allAlone : words.noCopy,
        flashed: [], later: [] };
}

// An ask as it's sent: with the activation token the act was given where
// it carries `token: true`, so its sender can bring its window forward;
// null where it needs one and none came, so it isn't sent without.
function tokened(act, token) {
    if (act.token !== true) {
        return act.ask;
    }
    return typeof token === "string" && token !== ""
        ? Object.assign({}, act.ask, { token }) : null;
}
