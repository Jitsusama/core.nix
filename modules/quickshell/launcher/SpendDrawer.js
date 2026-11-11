.pragma library
// What the drawer says of AI spend's rows (Spend.qml), each as the row
// brings it from spend-watch, a period's, a budget's, a window's and a
// cap's alike (its src/drawers.rs); and of the budget a session draws on,
// in its drawer (Agents.qml). The daemon chooses, orders and words every
// fact, leaving out what has an amount while the screen is shared, so
// this only names each fact by the vocabulary and draws it.

const DOT = " \u00b7 ";

// Whether a word was said at all.
function said(words) {
    return (words ?? "") !== "";
}

// The drawer for a row of AI spend, or null for one without: its eyebrow,
// set as it's written where it names something from the world; its title,
// the row's name unless it says one; and its facts, each named by its
// label where it carries one from the world, a window's, else by the
// vocabulary's name for its id, its meter beside it, in the words face
// where it says words rather than a name or a figure. `name` names a
// fact by its id.
function drawer(row, name) {
    const drawn = row.drawn;
    if (!drawn) {
        return null;
    }
    const facts = (drawn.facts ?? []).filter(one => said(one.value))
        .map(one => Object.assign({
            label: said(one.label) ? one.label : name(one.id),
            value: one.value
        }, one.figure ? { figure: one.figure } : {},
        one.words === true ? { words: true } : {}));
    return Object.assign({
        eyebrow: drawn.eyebrow ?? "",
        title: said(drawn.title) ? drawn.title : row.name,
        facts,
        body: ""
    }, drawn.names === true ? { eyebrowNames: true } : {});
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
