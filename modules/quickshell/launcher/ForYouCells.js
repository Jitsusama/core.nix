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

// How a notice names the session that raised it, as the notices daemon
// serves it (notice.rs's Cell): `mycelium:cell/<ADDRESS>[@<machine>][#<anchor>]`.
// Its group is named, as biome holds every pattern to, but read by place,
// since Qt's engine gives a match no `groups`.
const CELL_LINK = /^mycelium:cell\/(?<address>[A-Za-z0-9]+)(?:[@#]|$)/u;

// The address of the session that raised `notice`, as its `cell` names it,
// uppercased, or "" where it names none, as one not a session's doesn't.
function cellNamed(notice) {
    const named = CELL_LINK.exec(typeof notice?.cell === "string" ? notice.cell : "");
    return named === null ? "" : named[1].toUpperCase();
}

// The session among `held` that raised `notice`, by any address it answers
// to, or null: one naming no cell, or a session no longer held, stands
// among Joel's own. Away mode names a session by its own address, which it
// keeps whatever its quest.
function noticeCell(notice, held) {
    const address = cellNamed(notice);
    if (address === "") {
        return null;
    }
    const answers = session => [session.address].concat(session.presence?.addresses ?? [])
        .some(name => String(name ?? "").toUpperCase() === address);
    return (Array.isArray(held) ? held : []).find(answers) ?? null;
}

// The notices that stand under their cell among what needs Joel now: pi's
// own, unread and kept, from a session among `held`, but for one among
// `wanting`, whose want's row already says what it asks, or one whose want
// Joel put off for now (`putOff`, by address in capitals), which he asked
// to hear nothing of until then. A session's own notices are pi's, which
// Desktop leaves to its want's row (Lately.js's LISTED), so one about a
// session no longer asking, as away mode's saying a run ended, stood in no
// list until now. Another program's naming a cell stays in Desktop alone,
// as one from a session gone does, so none is listed twice.
function joining(notices, held, wanting, putOff) {
    const off = putOff ?? {};
    return (Array.isArray(notices) ? notices : []).filter(notice => {
        if (notice === null || typeof notice !== "object" || notice.app !== "pi"
            || notice.read === true || notice.transient === true) {
            return false;
        }
        const cell = noticeCell(notice, held);
        return cell !== null && noticeCell(notice, wanting) === null
            && off[String(cell.address).toUpperCase()] !== true;
    });
}

// `kept` while it holds the notices `joined` does, by id, else `joined`:
// so what For You lists from them, and bands, is made again only as which
// notices join moves, not as each notice is said anew, as one read while
// For You is open is (Lately.js's holdingStill).
function alike(kept, joined) {
    const before = Array.isArray(kept) ? kept : [];
    return before.length === joined.length
        && joined.every((notice, at) => notice.id === before[at].id) ? before : joined;
}

// A session's notice as For You weighs it: a thing of no kind it counts,
// waiting since it came, so it stands among what needs Joel now, under its
// session's heading, by how long it's waited.
function noticeThing(notice) {
    return { kind: "notice", since: notice.at ?? null };
}

// A template with each {key} filled in one pass, so what a value says is
// never read again: filled key by key, a title holding "{doing}" was
// filled a second time, and String.replace read "$&" in one as a pattern.
function filledOnce(template, fill) {
    return String(template ?? "").replace(/\{(?<key>\w+)\}/gu, (whole, key) =>
        Object.keys(fill).includes(key) ? String(fill[key]) : whole);
}
