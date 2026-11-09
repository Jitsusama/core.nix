// What Programs says of each password manager's program (Passwords.qml's
// programActs): its entries grouped by vault. Kept apart from Passwords.js,
// which says each entry, so each stays a size a reader holds whole; the
// the-passwords-show-every-figure-but-the-secret check runs it in Qt's own
// engine (tests/passwords.qml).
.pragma library

// Its entries grouped by vault, as Programs holds a manager's: each
// { id, name, records } by the vault's name, its records by title.
function vaults(records) {
    const by = {};
    const order = [];
    for (const record of records) {
        const id = String(record.collection?.id ?? "");
        if (!(id in by)) {
            by[id] = { id, name: String(record.collection?.name ?? ""), records: [] };
            order.push(id);
        }
        by[id].records.push(record);
    }
    const title = record => String(record.title ?? "");
    return order.map(id => by[id])
        .sort((a, b) => a.name.localeCompare(b.name))
        .map(vault => Object.assign(vault, {
            records: vault.records.sort((a, b) => title(a).localeCompare(title(b)))
        }));
}
