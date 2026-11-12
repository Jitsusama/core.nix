.pragma library
// A collection's row as Provider.qml's member() makes it: the provider's
// defaults, then the fields given, with a name made ready to match only
// when it's first read, since a collection has thousands.

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
