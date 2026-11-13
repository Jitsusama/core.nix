// Whether an act a daemon serves is whole, and the words an ask act asks
// Joel to type before it's sent, its `fields` (ready-rows.md's acts; F8TT8O
// and Y2YBTS, 2026-11-13, 1ace0a22): where each answer goes in the ask.
// ReadyRows.js does no act these say is misformed, ReadyRows.qml asks the
// fields, and tests/ready-rows.qml runs them in Qt's own engine.
.pragma library

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
