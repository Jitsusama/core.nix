.pragma library
// What the drawer says of AI spend's rows (Spend.qml): a budget's, a
// window's and a cap's, from how spend-watch says each budget stands
// (SpendWatch.budgets, its budgets.json), and a period's, from the facts
// its row carries in collection.json; and of the budget a session draws
// on, in its drawer (Agents.qml). The daemon words every fact, so this
// only chooses which to show and in what order, and leaves out what it
// didn't say rather than guessing. By the drawer's rules
// (VKCF3Z, 2026-11-07): no state in an eyebrow, nothing the row's side
// or the title already says, what moves most first and identifiers
// last; and no dollar figure while the screen is shared, as the rows
// show none.

const DOT = " \u00b7 ";

// Whether a word was said at all.
function said(words) {
    return (words ?? "") !== "";
}

// A budget's row is keyed "budget:<name>", and each of its windows'
// "budget:<name>:<window>" (spend-watch's collection.rs). The budget is
// found by its whole name rather than by splitting at a colon, so a name
// holding one still finds it. Null for any other row.
function keyed(key, budgets) {
    const prefix = "budget:";
    if (!String(key ?? "").startsWith(prefix)) {
        return null;
    }
    const rest = key.slice(prefix.length);
    if (budgets[rest]) {
        return { budget: budgets[rest], allowance: null };
    }
    const found = Object.keys(budgets)
        .filter(name => rest.startsWith(`${name}:`))
        .map(name => ({
            budget: budgets[name],
            allowance: (budgets[name].allowances ?? [])
                .find(one => one.name === rest.slice(name.length + 1)) ?? null
        }))
        .find(one => one.allowance !== null);
    return found ?? null;
}

// How near an allowance is, as Meter's stroke: in the warning's ink only
// from where it warns, as Context's is from 80%. None for one not read
// yet.
function stroke(allowance) {
    const near = allowance.nearest;
    if (typeof near !== "number") {
        return null;
    }
    return { shape: "stroke", values: [Math.min(1, Math.max(0, near))],
        warn: near >= allowance.warn };
}

function fact(label, value, figure) {
    return Object.assign({ label, value }, figure ? { figure } : {});
}

// Whether an allowance counts dollars, which go while the screen is
// shared.
function hidden(allowance, words) {
    return allowance.unit !== "share" && words.sharing === true;
}

// A window's facts beyond its name, in the order they move: what its feed
// says, which is the one thing to act on, while it says anything; its
// standing, where `standing` asks for it; when it resets; when an
// override of it ends and how it last changed; whose reading it is and
// when; and where it warns. A share's row already says how much,
// so its meter rides on Resets' line instead of a Standing that would
// repeat it, and on Read By's where it has no reset; `meter` false leaves
// it off, for a budget's drawer that already drew it. Dollars go while
// the screen is shared, and a dollar window's meter, which holds none,
// rides as a share's does.
function windowFacts(allowance, words, standing, meter) {
    const told = allowance.said ?? {};
    const quiet = hidden(allowance, words);
    const figure = meter ? stroke(allowance) : null;
    const stands = standing && allowance.unit !== "share" && !quiet
        && said(allowance.standing);
    const riding = stands ? null : figure;
    const resets = said(told.resets);
    return [
        ["state", told.state, null, true],
        ["standing", allowance.standing, figure, stands],
        ["resets", told.resets, riding, resets],
        ["override", told.ends, null, true],
        ["changed", told.changed, null, !quiet],
        ["read", told.read, resets ? null : riding, said(told.read)],
        ["warns", told.warns, null, !quiet]
    ].filter(([, value, , shows]) => shows && said(value))
        .map(([id, value, shape]) => fact(words.name(id), value, shape));
}

// A budget's drawer: what it is over its name; then each window by its
// feed's name, standing in its own unit with how near it is; a budget of
// one window says that window's other facts too, since it has no row of
// its own to step into; then what it spent and what a plan's turns were
// worth at list, how its feeds read, and the providers it always draws
// for, last as identifiers are.
function budgetDrawer(budget, words) {
    const allowances = budget.allowances ?? [];
    const shown = allowances.filter(one => !hidden(one, words));
    const windows = shown.map(one => fact(one.label,
        said(one.standing) ? one.standing : words.unread, stroke(one)));
    const only = allowances.length === 1
        ? windowFacts(allowances[0], words, false, shown.length === 0) : [];
    const told = budget.said ?? {};
    const amounts = words.sharing !== true;
    const more = [
        ["spent", amounts ? told.spent : ""],
        ["worth", amounts ? told.worth : ""],
        ["reading", told.reading],
        ["providers", (budget.providers ?? []).join(DOT)]
    ].filter(([, value]) => said(value))
        .map(([id, value]) => fact(words.name(id), value));
    return { eyebrow: budget.words ?? "", title: budget.label,
        facts: windows.concat(only, more), body: "" };
}

// A window's drawer: where it is and what it is over its name, "Your AI
// proxy token \u00b7 window", set as it's written since it names
// something (Drawer.qml); then its facts, its standing among them.
function windowDrawer(budget, allowance, words) {
    return { eyebrow: [budget.label, words.window].filter(said).join(DOT),
        eyebrowNames: true, title: allowance.label,
        facts: windowFacts(allowance, words, true, true), body: "" };
}

// A period's drawer: its days over its name, then its facts as the
// daemon gave them, each named by the vocabulary, in the words face where
// it says words rather than a name or a figure. The daemon leaves out
// what has an amount while the screen is shared.
function periodDrawer(title, drawn, words) {
    const facts = (drawn.facts ?? []).filter(one => said(one.value))
        .map(one => Object.assign(fact(words.name(one.id), one.value),
            one.words === true ? { words: true } : {}));
    return { eyebrow: drawn.eyebrow ?? "", title, facts, body: "" };
}

// The drawer for a row of AI spend, or null for one with none: a
// period's from its own facts, and a budget's, a window's or a cap's from
// the budget or window its `budget` names in budgets.json, null where it
// doesn't hold it. `words` holds the vocabulary's: `name` names each fact
// by its id, `unread` is what a window not read yet says, `window` what a
// window is; and `sharing` whether the screen is shared.
function drawer(row, budgets, words) {
    if (row.drawn) {
        return periodDrawer(row.name, row.drawn, words);
    }
    const found = keyed(row.budget, budgets ?? {});
    if (found === null) {
        return null;
    }
    return found.allowance === null ? budgetDrawer(found.budget, words)
        : windowDrawer(found.budget, found.allowance, words);
}

// The budget a session drew on last and how it stands, as its drawer's
// Budget says it: "Your AI proxy token \u00b7 $24,349 of $40,000". Empty
// when budgets.json doesn't name the session or the budget it names.
function sessionBudget(budgets, budgetOf, session) {
    const budget = budgets?.[budgetOf?.[session ?? ""] ?? ""] ?? null;
    if (budget === null) {
        return "";
    }
    return [budget.label, budget.standing].filter(said).join(DOT);
}
