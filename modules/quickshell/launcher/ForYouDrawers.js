// For You's Later and History, each a place, as their drawers say them
// (Now.qml's laterRow and historyRow): each row has one, and a drawer
// that would only repeat its row is a row still missing its facts
// (docs/design.md, Every Row Says a Little More), so each says what its
// row can't. Apart from the view, so tests/for-you.qml holds them in Qt's
// own engine; every word comes from the vocabulary, worked out by the
// view and passed in.
.pragma library

// Later's drawer, a place's: its eyebrow saying so, as every place's
// does, and a fact for each item it holds, named by the item's whole
// title and saying what it waits for, as its own row in Later says it at
// its side, and how long it has waited, since Later's note names them and
// says neither (docs/design.md, Every Row Says a Little More). Each of
// `items` is { name, waits, waited }, `waited` already in words ("waiting
// 3 h"); one saying neither is left out, and null where none says either,
// for the view to name them as any place does (PlaceRow.js's drawer).
function laterDrawer(title, items, eyebrow) {
    const facts = (items ?? []).map(one => ({ label: String(one.name ?? ""),
        value: [one.waits, one.waited].filter(part => (part ?? "") !== "").join(" \u00b7 ") }))
        .filter(fact => fact.label !== "" && fact.value !== "");
    return facts.length === 0 ? null : { eyebrow, title, facts };
}

// History's drawer, a place's: its eyebrow saying so, and how many it
// keeps from each source, so it says what its side's count is made of,
// which neither its side nor its note does. Each of `kept` is { name,
// count }, in the order the facts stand, For You's own sources before the
// world's; two of one name are one fact, where the first stands, and one
// keeping none or named nothing is left out; null where none is left.
function historyDrawer(title, kept, eyebrow) {
    const facts = [];
    const counted = (kept ?? []).filter(one => String(one.name ?? "") !== ""
        && Number.isFinite(one.count) && one.count > 0);
    for (const one of counted) {
        const label = String(one.name);
        const same = facts.find(fact => fact.label === label);
        if (same === undefined) {
            facts.push({ label, count: one.count });
        } else {
            same.count += one.count;
        }
    }
    return facts.length === 0 ? null : { eyebrow, title,
        facts: facts.map(fact => ({ label: fact.label, value: String(fact.count) })) };
}
