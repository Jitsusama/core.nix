// The spots of each thing a collection holds, kept thing by thing across
// closings (Launcher.qml's spotsFrom). A collection whose source changed
// made every thing's parts again: Joel's closing at 14:36 on 2026-11-11
// made 249 rows, Sound's 111 and Displays' 87, for a change to one
// device or one screen's level. A source that says what one thing's
// parts are made of (its spotsStampOf) has only the things whose stamp
// moved made again. It's a stopgap: each place's daemon serving the
// parts ready ends it, Screens' (F8TT8O) by 12 November and Sound's
// (1KZFHC) by 13 November.
.pragma library

// Each of `things` with its spots, kept from `before` (what the last
// making kept, by the thing's key) while `stampOf` says the same of it,
// or made by `make`. With no `stampOf`, or a stamp of null, every thing
// is made, as it was. Gives what's kept for the next making, by key, and
// how many things were made.
function eachKept(before, things, stampOf, make) {
    const kept = {};
    let made = 0;
    for (const thing of things) {
        const stamp = stampOf ? stampOf(thing) ?? null : null;
        const was = before?.[thing.key];
        if (stamp !== null && was !== undefined && was.stamp === stamp) {
            kept[thing.key] = was;
            continue;
        }
        kept[thing.key] = { stamp, spots: make(thing) };
        made += 1;
    }
    return { kept, made };
}

// The spots of everything kept, in the things' order.
function spotsOf(things, kept) {
    let spots = [];
    for (const thing of things) {
        spots = spots.concat(kept[thing.key]?.spots ?? []);
    }
    return spots;
}
