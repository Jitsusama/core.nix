// The windows as the windows daemon says them (modules/windows), kept in
// the order the shell lists them, the one used last first, by what each
// record's `order` says, so a change places only the windows it names
// rather than sorting all of them again. Windows.qml and Applications.qml
// keep their rows by it, and tests/held-windows.qml runs it in Qt's own
// engine.
.pragma library

// What's held: each record by its key, each window as niri describes one
// (`windows`), and the keys in their order.
function kept() {
    return { records: {}, windows: {}, order: [] };
}

// Whether two records say the same, field by field; a record's fields
// are plain values.
function same(a, b) {
    const keys = Object.keys(a);
    if (keys.length !== Object.keys(b).length) {
        return false;
    }
    return keys.every(key => a[key] === b[key]);
}

// Whether the record `a` is listed before `b`: used later first, and of
// two used at once, the newer window.
function before(a, b) {
    return a.order > b.order || (a.order === b.order && a.id > b.id);
}

// Where a record goes among those held, found by halving.
function placeOf(held, record) {
    let low = 0;
    let high = held.order.length;
    while (low < high) {
        const middle = (low + high) >> 1;
        if (before(held.records[held.order[middle]], record)) {
            low = middle + 1;
        } else {
            high = middle;
        }
    }
    return low;
}

function unlist(held, key) {
    const at = held.order.indexOf(key);
    if (at >= 0) {
        held.order.splice(at, 1);
    }
}

// A window as niri describes one, from its record, so whatever reads a
// window (its verbs, where its row says it is) reads it as it did when
// the shell heard niri itself.
function asWindow(record) {
    return {
        id: record.id,
        app_id: record.app_id,
        opened: record.opened,
        session: record.session,
        title: record.title,
        pid: record.pid,
        workspace_id: record.workspace,
        is_floating: record.floating,
        is_urgent: record.urgent,
        is_focused: record.focused,
        layout: {
            pos_in_scrolling_layout: record.column === null ? null : [record.column, record.down]
        },
        focus_timestamp: record.order === 0 ? null
            : { secs: Math.floor(record.order / 1000), nanos: (record.order % 1000) * 1e6 }
    };
}

// Whether any program's windows, as Applications lists them by owner,
// came, went or were used in another order between `was` and `now`: what
// a program's row says beside its name and where Go To goes. A window
// retitled alone moves nothing here.
function ownersMoved(was, now) {
    const ids = windows => (windows ?? []).map(window => window.id).join(" ");
    const owners = Object.keys(was).concat(Object.keys(now));
    return owners.some(owner => ids(was[owner]) !== ids(now[owner]));
}

// Takes these records and lets these keys go, answering the keys whose
// records differ from what was held, in the order they came: a whole
// answer says every window again, and only those that changed are
// remade.
function take(held, records, gone) {
    for (const key of gone) {
        if (key in held.records) {
            delete held.records[key];
            delete held.windows[key];
            unlist(held, key);
        }
    }
    const differ = [];
    for (const record of records) {
        const key = record.key;
        const was = held.records[key];
        if (was !== undefined && same(was, record)) {
            continue;
        }
        if (was !== undefined) {
            unlist(held, key);
        }
        held.records[key] = record;
        held.windows[key] = asWindow(record);
        held.order.splice(placeOf(held, record), 0, key);
        differ.push(key);
    }
    return differ;
}

// Takes when windows were used, as a watch of `order` alone tells it
// while the launcher is shut, placing each window held where it now goes
// and making nothing else, so the next opening lists them in the order
// used now. Answers the keys it doesn't hold, windows opened since, whose
// whole records are asked for.
function used(held, records) {
    const unknown = [];
    for (const record of records) {
        const was = held.records[record.key];
        if (was === undefined) {
            unknown.push(record.key);
            continue;
        }
        if (was.order === record.order) {
            continue;
        }
        unlist(held, record.key);
        const now = Object.assign({}, was, { order: record.order });
        held.records[record.key] = now;
        held.windows[record.key] = asWindow(now);
        held.order.splice(placeOf(held, now), 0, record.key);
    }
    return unknown;
}

// The workspaces a window on this one could go to, from the daemon's
// records by key (`said`): the others on its screen, in the order the
// desktop record lists that screen's, so nothing is sorted and no window
// walked. None while the daemon hasn't said the window's workspace.
function spacesFrom(said, workspace) {
    const from = said[`workspace:${workspace}`];
    if (from === undefined) {
        return [];
    }
    const ids = said.desktop?.spaces?.[from.output] ?? [];
    return ids
        .filter((id) => id !== from.id)
        .map((id) => said[`workspace:${id}`])
        .filter((space) => space !== undefined);
}

// What a workspace has on it, as its row's side says it: each program
// once, as the daemon counts them (`inside`), with how many of its
// windows when more than one, in the words Words.qml gives (`words`:
// `empty`, and `counted` with {program} and {count}).
function insideSaid(inside, words) {
    if (inside.length === 0) {
        return words.empty;
    }
    const said = (one) =>
        words.counted.replace("{program}", one.program).replace("{count}", String(one.count));
    return inside.map((one) => (one.count === 1 ? one.program : said(one))).join(", ");
}
