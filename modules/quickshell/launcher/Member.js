.pragma library
// A collection's row as Provider.qml's member() makes it: the provider's
// defaults, then the fields given, with a name made ready to match only
// when it's first read, since a collection has thousands. And the rows a
// search found, made again as the launcher shows them.

// The row of `fields` over `defaults`, its `prepared` read through
// `prepare` from its own name once something asks for it. The getter is
// the row's first field, made before any other is copied, and a
// `prepared` among the fields is left behind, since it was another row's.
// A row made again from a row made copies that getter as a plain field,
// and turning a field the row has into a getter in place makes Qt 6.12
// keep the getter's slot over the next field of every row grown from the
// same shape: a field then read as nothing, or a write went past the
// row's end, which crashed the shell three times at 23:05 on 2026-11-11.
function made(defaults, fields, prepare) {
    const row = {};
    preparing(row, prepare);
    for (const name of Object.keys(defaults)) {
        if (!(name in fields)) {
            row[name] = defaults[name];
        }
    }
    for (const name of Object.keys(fields)) {
        if (name !== "prepared") {
            row[name] = fields[name];
        }
    }
    return row;
}

// `row`'s getter, made here rather than in made(), since Qt's engine keeps
// every argument of a function alive with any closure made in it
// (qv4compilerscanfunctions.cpp's argumentsCanEscape): made there, it kept
// the defaults and fields each row was made from, 290 bytes of every ready
// row in the shell (tests/member-light.qml says what it costs).
function preparing(row, prepare) {
    let prepared = null;
    Object.defineProperty(row, "prepared", {
        enumerable: true,
        get: () => {
            if (prepared === null) {
                prepared = prepare(row.name);
            }
            return prepared;
        }
    });
}

// `row` made again over `fields`, each of its own fields standing over
// theirs and its getters copied unread, as a search makes what it found
// (Launcher.qml). Object.assign read every getter a row has, so making "m"
// read what each of 96 programs holds and every act it has, 17 ms of a key
// for the nine rows drawn (2026-11-12), and froze what's meant to be read
// as the row is. Its names come from its descriptors, since Qt's
// Object.keys reads each value it names (qv4objectproto.cpp's method_keys),
// and it's all laid on a fresh row at once, so no field the row has is
// turned into a getter.
function copied(row, fields) {
    const laid = Object.getOwnPropertyDescriptors(fields);
    const own = Object.getOwnPropertyDescriptors(row);
    for (const name of Object.keys(own)) {
        if (own[name].enumerable) {
            laid[name] = own[name];
        }
    }
    return Object.defineProperties({}, laid);
}

// `row` made again over `fields` as copied() makes it, with what it was
// made from, or `last` itself when that was made from this very row over
// the same fields. A search makes every row it finds afresh, so a row kept
// across a letter was a new one, and the launcher's row showing it ran
// each of its bindings again; found again alike, it's the same row, which
// a binding to it hears as no change. The keys lab's "co" showed one row
// new in 40 to 67 ms, its laying 6 to 12 of it (2026-11-15).
function copiedAgain(last, row, fields) {
    if (last !== undefined && last.from === row && alike(last.fields, fields)) {
        return last;
    }
    return { from: row, fields, made: copied(row, fields) };
}

// Whether two rows' makings give each field the same value.
function alike(was, now) {
    const names = Object.keys(now);
    return names.length === Object.keys(was).length
        && names.every(name => was[name] === now[name]);
}

// The rows found, each made by `copy` in its place, and the milliseconds
// `clock` saw each kind take, `sortOf` naming a row's kind. Copying a row
// reads every getter it has, and Joel's first "c" spent 11 ms making 112
// rows (2026-11-12), so a slow key says whose rows it spent on. Each kind
// is made together and timed once, so timing costs nothing a row.
function madeByKind(rows, sortOf, copy, clock) {
    const each = new Array(rows.length);
    const at = Object.create(null);
    rows.forEach((row, index) => {
        const sort = sortOf(row);
        if (!at[sort]) {
            at[sort] = [];
        }
        at[sort].push(index);
    });
    const took = {};
    for (const sort of Object.keys(at)) {
        const began = clock();
        for (const index of at[sort]) {
            each[index] = copy(rows[index]);
        }
        took[sort] = clock() - began;
    }
    return { made: each, took };
}
