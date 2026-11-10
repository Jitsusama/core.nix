// What Programs says of each password manager's program (Passwords.qml's
// programActs and programFacts): its entries grouped by vault, and what
// its drawer says it holds; and which of the rows its entries were made
// into are kept as it lists them again. Kept apart from Passwords.js,
// which says each entry, so each stays a size a reader holds whole; the
// the-passwords-show-every-figure-but-the-secret check runs it in Qt's own
// engine (tests/passwords.qml).
.pragma library
.import "Numbers.js" as Numbers

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

// What a manager's drawer in Programs says it holds, from the entries the
// daemon already gave it (VKCF3Z, 2026-11-08): how many and in how many
// vaults, "412 in 3 vaults" by `said.inVaults` ("412 in 1 vault" by
// `said.inVault`), then how many are favourites, each left out at none,
// by its id in the launcher's words. It's never asked while the manager
// can't read, so a locked one says nothing rather than a guess.
function held(records, said) {
    const count = records.length;
    const kept = new Set(records.map(record => String(record.collection?.id ?? ""))).size;
    const liked = records.filter(record => record.favourite === true).length;
    const within = String((kept === 1 ? said.inVault : said.inVaults) ?? "")
        .replace("{count}", Numbers.format(count, 0)).replace("{vaults}", String(kept));
    return [
        ["passwords:entries", count === 0 ? "" : within],
        ["passwords:favourites", liked > 0 ? Numbers.format(liked, 0) : ""]
    ].filter(fact => fact[1] !== "").map(fact => ({ id: fact[0], value: fact[1] }));
}

// What a vault holds, at its row's right, saying what it counts, "1 entry"
// or "1,201 entries", by `said`'s words, rather than a bare figure after
// its name, where it read as part of the name (VKCF3Z, de87ff45).
function holding(count, said) {
    const words = String((count === 1 ? said.entry : said.entries) ?? "");
    return words.replace("{count}", Numbers.format(count, 0));
}

// Each entry's row, the one made before while its record says the same,
// else made again by `made`, with what to keep for the next listing:
// { rows, kept }, `kept` by key and given back as `before`. The daemon
// gives every record it lists as a new object, unchanged or not, and a
// step into Passwords asks it to list them all, so rows kept only while
// their record was the same object were each made again just after the
// step was drawn (2026-11-10).
function rowsKept(records, before, made) {
    const kept = {};
    const rows = records.map(record => {
        const was = before[record.key];
        const said = was?.record === record ? was.said : JSON.stringify(record);
        const row = was && was.said === said ? was.row : made(record);
        kept[record.key] = { record, said, row };
        return row;
    });
    return { rows, kept };
}

// What a manager's note in Programs says while it can't read: why, as
// its daemon says, less the manager's own name leading it, since the note
// sits under its program, and a note is a lower-case run rather than a
// sentence (VKCF3Z, c943eec1). "1Password asks you to let the launcher
// in" reads "asks you to let the launcher in"; a copy refused, said
// elsewhere, keeps the name.
function noted(reason, called) {
    const why = String(reason ?? "");
    const name = String(called ?? "");
    return name !== "" && why.startsWith(`${name} `) ? why.slice(name.length + 1) : why;
}

// The row leading the place for each manager that can't read, by how its
// daemon says it stands: Unlock while it's signed out, waiting on Joel or
// its app, and its program while it's refused, where only a setting there
// lets it read, so one that can't read is never left without a row
// (2026-11-10: the place was empty while 1Password refused).
function unread(managers, sources) {
    const rows = { "signed-out": "unlock", refused: "settle" };
    return managers.map(manager => ({ manager, row: rows[sources[manager.name]?.state] }))
        .filter(led => led.row !== undefined);
}
