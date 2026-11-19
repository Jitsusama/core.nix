// What a step in or out of a place spent its time on, as a key's parts are
// said (Trials.js), and what its making and holding went on, so a slow
// step's line in the journal says which part to cut. Trials.qml says them.
.pragma library
.import "Order.js" as Order

// What a step spent its time on, said as a key's parts are: `inner` the
// parts of the search that drew the place stepped into, or null, and
// `at` when the step began, made its lists, held them, opened the row
// and ended. Its lists are the step's own and those the place found its
// rows in; its rows whatever else it did after opening; its answers and
// their composing the place's own, so a slow step says where its rows
// went (RTOIMQ, 2026-11-10). The search's time before it began is said
// apart, as a key's is, since Joel's step into Machine spent 16 ms after
// opening, where it had spent 10, with nothing saying where (2026-11-11).
// A step out's `at.holding` holds the ms until it was held and the rows
// each source made while it was, said with its holding (steppedHolding).
function stepped(inner, at) {
    const finding = inner ? inner.lists : 0;
    const answers = inner ? inner.answers : 0;
    const before = inner ? inner.before : 0;
    // A walk's last step is timed from where the walk began, so the
    // places it passed on the way, listed to step through, are said as
    // passing among its lists, apart from this step's own making.
    const passing = at.walked === undefined ? 0 : Math.max(0, at.began - at.walked);
    return {
        lists: at.made - at.began + finding + passing,
        rows: Math.max(0, at.ended - at.opened - finding - answers - before),
        before,
        beforeParts: inner?.beforeParts ?? null,
        answers,
        each: Object.assign({}, inner?.each, { making: at.made - at.began, finding,
            holding: at.pushed - at.made, opening: at.opened - at.pushed, passing }),
        makingSplit: at.making ?? null,
        holdingSplit: at.holding ?? null,
        composing: inner?.composing ?? null
    };
}

// A walk's parts where it stopped short, as the search listing where it
// stopped found them, with what it passed before as passing among its
// lists, as a walk's last step says it (stepped). Kept as they were where
// it passed nothing.
function passed(parts, passing) {
    if (!parts || !(passing > 0)) {
        return parts;
    }
    return Object.assign({}, parts, { lists: (parts.lists ?? 0) + passing,
        each: Object.assign({}, parts.each, { passing }) });
}

// What a step out's holding spent on, in parentheses: the rows each source
// made while the step was held, the three that made most, the first A to
// Z where two made as many (Order.js), and the time until it was held,
// from the step's start, its first count of the rows included. Joel's
// step out of Now held 14, 22, then 38 ms in one evening, where every
// other step out held 2 to 4, and the lab's Now held 3 to 6 with nothing
// to say where it went (2026-11-18). `split` holds the
// rows by source and the ms until held; nothing for an untold split. Its
// time is said as "until held", never "before", which the line says of the
// search's own time before it began. One row is said as one, never "1 rows".
function steppedHolding(holding, split) {
    if (!split || !(holding > 0)) {
        return "";
    }
    const rows = split.rows ?? {};
    const most = Order.then(Order.larger((name) => rows[name]), Order.byName((name) => name));
    const made = Order.sorted(Object.keys(rows).filter((name) => rows[name] > 0), most)
        .slice(0, 3).map((name) => `${name} ${rows[name]} ${rows[name] === 1 ? "row" : "rows"}`);
    const until = Math.round(split.untilHeld ?? 0);
    const named = made.length > 0 ? made : ["no rows"];
    return ` (${named.concat(until > 0 ? [`until held ${until}`] : []).join(", ")})`;
}

// The three collections that took longest, if any took a millisecond,
// as a key's line says its lists, a step's own making and holding with
// what each went on (steppedMaking, steppedHolding), from the step's
// parts. Here rather than in Trials.qml, so Qt's engine holds the words
// the line says (tests/trials.qml).
function slowest(each, parts) {
    const took = each ?? {};
    const named = Order.sorted(Object.keys(took).filter((name) => took[name] > 0),
        Order.larger((name) => took[name])).slice(0, 3);
    const said = (name) => `${name} ${took[name]}`
        + (name === "making" ? steppedMaking(took[name], parts?.makingSplit)
            : name === "holding" ? steppedHolding(took[name], parts?.holdingSplit)
            : "");
    return named.length === 0 ? "" : `: ${named.map(said).join(", ")}`;
}

// What a step's making spent on, in parentheses, most first: making its
// acts' rows, its parts, putting them in order, and the rest, so a slow
// step into a program's page says which to cut (Chrome's made 87 ms of a
// 253 ms step under load, with nothing saying where, 2026-11-14). `split`
// holds the acts', the parts' and the ordering's ms; the rest is what's
// left of the making. Nothing for a step that made nothing, or untold.
function steppedMaking(making, split) {
    if (!split || !(making > 0)) {
        return "";
    }
    const each = [["its parts", Math.round(split.parts ?? 0)],
        ["its acts", Math.round(split.acts ?? 0)],
        ["ordering", Math.round(split.ordering ?? 0)]];
    const told = each.reduce((sum, [, ms]) => sum + ms, 0);
    each.push(["the rest", Math.max(0, Math.round(making) - told)]);
    each.sort((one, other) => other[1] - one[1]);
    return ` (${each.map(([name, ms]) => `${name} ${ms}`).join(", ")})`;
}
