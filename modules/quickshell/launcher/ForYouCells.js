// What needs Joel now grouped by the cell behind each, What Needs Me (Joel,
// 2026-11-16: "one 'what needs me' view grouped by cell"; shape agreed by
// Y2YBTS, 2026-11-18). Apart from the launcher, so tests/for-you.qml holds
// it in Qt's own engine, and Now.qml only lays out what it gives; the
// agents daemon takes it over once it serves For You's rows (Y2YBTS).
.pragma library

// [{ cell, items }], `cellOf(item)` giving the session behind an item, or
// null. `items` come in their band's order (ForYou.js's banded), so each
// cell's rows keep it, heaviest first, and the cells stand by their
// heaviest, the first each holds. Those with no cell, or one with no
// address, are one group of Joel's own, `cell` null, placed by its heaviest
// as a cell is. A cell is one by its address, whatever its case. What a
// cell asks, `leads(item)`, leads it whatever a look it left weighs, since
// the ask is what holds the session up, and a look left an hour before
// can weigh as much.
function byCell(items, cellOf, leads) {
    const groups = [];
    // No prototype, so an address is never taken for one of Object's own.
    const at = Object.create(null);
    for (const item of items ?? []) {
        const found = cellOf(item) ?? null;
        const address = typeof found?.address === "string" ? found.address.toUpperCase() : "";
        if (!(address in at)) {
            at[address] = groups.length;
            groups.push({ cell: address === "" ? null : found, items: [] });
        }
        groups[at[address]].items.push(item);
    }
    const first = leads ?? (() => false);
    return groups.map(group => Object.assign(group, { items: group.items.filter(first)
        .concat(group.items.filter(item => !first(item))) }));
}

// The groups' rows, `rowsOf(items)` each, under each one's heading,
// `headingOf(cell)`, a group whose rows all went as they were made left
// out. Each is headed while a cell's rows are among those left; Joel's own
// alone need no heading to say whose they are. Decided once the rows are
// made, since decided before, his own heading stood alone once a cell's
// rows had all gone.
//
// `asked`, `{ rows, weight }`, is Agents Asking's row (or none) and its
// heaviest's weight. While a heading shows it stands first, above them
// all, since a heading claims what's under it and one between the groups
// would read as the cell's above it; with none, among the loose rows
// before the first that weighs less, as ForYou.js's agentAt places it
// (Y2YBTS, 2026-11-18 15:49). A lone ask is never gathered, so it stays
// under its own cell's heading as itself.
function laidOut(groups, rowsOf, headingOf, asked) {
    const gathered = asked?.rows ?? [];
    const made = groups.map(group => ({ cell: group.cell,
        items: group.items.map(item => ({ item, rows: rowsOf([item]) })) }))
        .map(group => Object.assign(group, { items: group.items.filter(one =>
            one.rows.length > 0) }))
        .filter(group => group.items.length > 0);
    const rowsIn = items => items.reduce((rows, one) => rows.concat(one.rows), []);
    if (made.some(group => group.cell !== null)) {
        return made.reduce((rows, group) =>
            rows.concat([headingOf(group.cell)], rowsIn(group.items)), gathered);
    }
    const loose = made.reduce((items, group) => items.concat(group.items), []);
    const lighter = loose.findIndex(one => (one.item.weight ?? 0) < (asked?.weight ?? 0));
    const at = lighter < 0 ? loose.length : lighter;
    return rowsIn(loose.slice(0, at)).concat(gathered, rowsIn(loose.slice(at)));
}

// A cell's heading as Agents' own row says the cell: its name and how it
// stands as the agents daemon words them (`served`, Agents' row for it, or
// null), never words made here (Y2YBTS, 2026-11-18); its title alone, or
// its address, while the daemon says neither; and Joel's own heading where
// no cell is, each from on:waiting's words (`said`).
function headingName(cell, served, said) {
    if (cell === null) {
        return String(said.yours ?? "");
    }
    const title = String(served?.name || cell.title || String(cell.address ?? "").toUpperCase());
    const doing = String(served?.doing ?? "");
    return doing === "" ? title : filledOnce(said.cell, { title, doing });
}

// A template with each {key} filled in one pass, so what a value says is
// never read again: filled key by key, a title holding "{doing}" was
// filled a second time, and String.replace read "$&" in one as a pattern.
function filledOnce(template, fill) {
    return String(template ?? "").replace(/\{(?<key>\w+)\}/gu, (whole, key) =>
        Object.keys(fill).includes(key) ? String(fill[key]) : whole);
}
