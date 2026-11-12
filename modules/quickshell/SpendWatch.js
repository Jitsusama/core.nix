.pragma library
// spend-watch's records, as its socket tells them (its src/records.rs),
// and what SpendWatch.qml makes of them. collection.json and budgets.json,
// read while the socket isn't there, are cut into the same records here,
// so the shell holds one shape whichever told it and remakes only what
// changed. Spend's rows aren't among them: spend-watch serves them ready
// (launcher/Spend.qml, on ReadyRows.qml), with no file to stand in.

// The kinds budgets.json is cut into; the rest are collection.json's.
const budgetKinds = ["budget", "drew"];

// Whether a key names a record of budgets.json rather than the collection.
function ofBudgets(key) {
    return budgetKinds.includes(kindOf(key));
}

// The kind a key names: "session" for "session:01a1...".
function kindOf(key) {
    const at = key.indexOf(":");
    return at > 0 ? key.slice(0, at) : "";
}

// The id a key names, after its kind: "01a1..." for "session:01a1...".
function idOf(key) {
    return key.slice(key.indexOf(":") + 1);
}

// Whether two values say the same, whatever order their keys come in:
// the daemon's serde sorts each record's keys, and the file keeps the
// order it was written in, so a record told by one and held from the
// other is still the same record.
function alike(a, b) {
    if (a === b) {
        return true;
    }
    if (typeof a !== "object" || typeof b !== "object" || a === null || b === null
        || Array.isArray(a) !== Array.isArray(b)) {
        return false;
    }
    const keys = Object.keys(a);
    return keys.length === Object.keys(b).length
        && keys.every(key => key in b && alike(a[key], b[key]));
}

// collection.json as records, by key, as the daemon cuts it.
function records(collection) {
    const cut = {};
    const session = id => {
        const key = `session:${id}`;
        if (!(key in cut)) {
            cut[key] = { key, kind: "session" };
        }
        return cut[key];
    };
    for (const [from, field] of [["sessions", "spent"], ["sessionsToday", "today"],
        ["helpers", "helpers"]]) {
        for (const [id, value] of Object.entries(collection[from] ?? {})) {
            session(id)[field] = value;
        }
    }
    for (const [id, family] of Object.entries(collection.families ?? {})) {
        cut[`quest:${id}`] = Object.assign({ key: `quest:${id}`, kind: "quest" }, family);
    }
    for (const [id, report] of Object.entries(collection.reports ?? {})) {
        cut[`report:${id}`] = { key: `report:${id}`, kind: "report", report };
    }
    for (const [id, from] of [["all", "today"], ["agents", "agentsToday"]]) {
        cut[`day:${id}`] = { key: `day:${id}`, kind: "day", spent: collection[from] ?? null };
    }
    for (const id of ["periods", "cuts", "examples"]) {
        cut[`words:${id}`] = { key: `words:${id}`, kind: "words", words: collection[id] ?? [] };
    }
    return cut;
}

// budgets.json as records, by key, as the daemon cuts it: each budget by
// its name, and the budget each session drew on last by its id.
function budgetRecords(standing) {
    const cut = {};
    for (const budget of standing?.budgets ?? []) {
        cut[`budget:${budget.name}`] = { key: `budget:${budget.name}`, kind: "budget", budget };
    }
    for (const [id, budget] of Object.entries(standing?.sessions ?? {})) {
        cut[`drew:${id}`] = { key: `drew:${id}`, kind: "drew", budget };
    }
    return cut;
}

// The properties a record's change touches, from what it was to what it
// is, either null: each of a session's figures its own, so a session
// whose spend moved and whose day didn't leaves sessionsToday as it was.
function touched(was, now) {
    const record = now ?? was;
    const differs = field => !alike(was?.[field], now?.[field]);
    switch (record.kind) {
    case "session":
        return [["spent", "sessions"], ["today", "sessionsToday"], ["helpers", "helpers"]]
            .filter(([field]) => differs(field)).map(([, property]) => property);
    case "quest":
        return ["families"];
    case "day":
        return [idOf(record.key) === "all" ? "today" : "agentsToday"];
    case "report":
        return ["reports"];
    case "words":
        return [idOf(record.key)];
    case "budget":
        return ["budgets"];
    case "drew":
        return ["budgetOf"];
    default:
        return [];
    }
}

// A property as the records held make it.
function made(held, property) {
    const of = kind => Object.keys(held).filter(key => kindOf(key) === kind)
        .map(key => held[key]);
    const byId = (kind, take) => {
        const all = {};
        for (const record of of(kind)) {
            const value = take(record);
            if (value !== undefined) {
                all[idOf(record.key)] = value;
            }
        }
        return all;
    };
    switch (property) {
    case "sessions":
        return byId("session", record => record.spent);
    case "sessionsToday":
        return byId("session", record => record.today);
    case "helpers":
        return byId("session", record => record.helpers);
    case "families":
        return byId("quest", record => ({ all: record.all, own: record.own,
            under: record.under }));
    case "today":
        return String(held["day:all"]?.spent ?? "");
    case "agentsToday":
        return String(held["day:agents"]?.spent ?? "");
    case "reports":
        return byId("report", record => record.report);
    case "budgets":
        return byId("budget", record => record.budget);
    case "budgetOf":
        return byId("drew", record => record.budget);
    default:
        return held[`words:${property}`]?.words ?? [];
    }
}
