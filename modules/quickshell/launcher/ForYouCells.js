// What needs Joel now grouped by the cell behind each, What Needs Me (Joel,
// 2026-11-16: "one 'what needs me' view grouped by cell"; shape agreed by
// Y2YBTS, 2026-11-18). Apart from the launcher, so tests/for-you.qml holds
// it in Qt's own engine, and Now.qml only lays out what it gives; the
// agents daemon takes it over once it serves For You's rows (Y2YBTS).
.pragma library

// [{ cell, items, headed }], `cellOf(item)` giving the session behind an
// item, or null. `items` come in their band's order (ForYou.js's banded),
// so each cell's rows keep it, heaviest first, and the cells stand by their
// heaviest, the first each holds. Those with no cell, or one with no
// address, are one group of Joel's own, `cell` null, placed by its heaviest
// as a cell is. A cell is one by its address, whatever its case. Each group
// is headed while any cell is among them; Joel's own alone needs no heading
// to say whose they are.
function byCell(items, cellOf) {
    const groups = [];
    const at = {};
    for (const item of items ?? []) {
        const found = cellOf(item) ?? null;
        const address = typeof found?.address === "string" ? found.address.toUpperCase() : "";
        if (!(address in at)) {
            at[address] = groups.length;
            groups.push({ cell: address === "" ? null : found, items: [] });
        }
        groups[at[address]].items.push(item);
    }
    const headed = groups.some(group => group.cell !== null);
    return groups.map(group => Object.assign(group, { headed }));
}

// Whether Agents Asking stands first, above every heading, among `groups`
// (byCell's): so while any heading shows, since a heading claims what's
// under it, and one between the groups would read as the cell's above it;
// with none shown, ForYou.js's agentAt places it by weight among the loose
// rows (Y2YBTS, 2026-11-18 15:49). A lone ask is never gathered, so it
// stays under its own cell's heading as itself.
function askedFirst(groups) {
    return groups.some(group => group.headed);
}
