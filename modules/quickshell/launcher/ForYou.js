// For You, Now's first row (Now.qml): what needs Joel in one list, weighed
// so the most pressing leads (VKCF3Z, 2026-11-08; Joel: "should somehow
// weight ordering so the most critical stuff bubbles up"). What a row
// weighs is worked out here from its parts, each { id, points }, and so
// is which band it stands in, what its side says and how the looks
// folder's files are read, apart from the launcher, so tests/for-you.qml
// holds them in Qt's own engine.
.pragma library
.import "Order.js" as Order

const SECOND = 1000;
const MINUTE = 60 * SECOND;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

// What each part counts, as the design weighs them (VKCF3Z, 2026-11-08). A
// session waiting on Joel counts most, since nothing moves there until he
// does, and each more it blocks adds less than the first. A deadline
// passed counts as much, one coming soon less, one far off nothing yet.
// What he can do now beats what only waits, a thing to do beats a thing to
// look at, and waiting rises slowly, so an old thing climbs without ever
// passing what blocks or is due.
const BLOCKS_FIRST = 40;
const BLOCKS_MORE = 15;
const DUE_PASSED = 40;
const DUE_WITHIN_HOUR = 30;
const DUE_WITHIN_DAY = 10;
const READY = 20;
const KIND_POINTS = { todo: 10, look: 5 };
const WAITED_MOST = 20;
const WAITED_RATE = 5;

// What a look is about, and how it stands, as the harness writes them, and
// the one version of its file this reads.
const KINDS = ["pr", "page", "file", "diff", "terminal"];
const STATES = ["waiting", "opened", "cleared", "resolved"];
const VERSION = 1;
// What's read from a look's file only for the shell's own use, never
// written back.
const OWN = ["since", "resolved", "file"];
// Each kind's mark, the theme's own (modules/theme/glyphs.nix): a pull
// request as its branch, a page as the web, a file as a path, a diff as
// lines of text and a terminal as a shell's prompt.
const GLYPHS = { pr: "branch", page: "web", file: "path", diff: "lines", terminal: "prompt" };

// The theme's name for a look's mark, by its kind.
function glyphOf(kind) {
    return GLYPHS[kind] ?? "path";
}

// The theme's name for For You's own mark, an in-tray, by how many it
// holds now and later: emptied only when it holds neither. Never the
// person with a clock its to-dos wear, since a place and a kind of thing
// in it are two meanings (VKCF3Z, 2026-11-08).
function markOf(now, later) {
    return now > 0 || later > 0 ? "forYou" : "forYouEmpty";
}

// A time as milliseconds, from a number or an ISO time, or null.
function when(at) {
    if (typeof at === "number") {
        return Number.isFinite(at) ? at : null;
    }
    if (typeof at !== "string" || at === "") {
        return null;
    }
    const read = Date.parse(at);
    return Number.isNaN(read) ? null : read;
}

function blocking(count) {
    return count > 0 ? BLOCKS_FIRST + BLOCKS_MORE * (count - 1) : 0;
}

function dueing(due, now) {
    if (due === null) {
        return 0;
    }
    if (due <= now) {
        return DUE_PASSED;
    }
    if (due - now <= HOUR) {
        return DUE_WITHIN_HOUR;
    }
    return due - now <= DAY ? DUE_WITHIN_DAY : 0;
}

function waiting(since, now) {
    if (since === null) {
        return 0;
    }
    const hours = Math.max(0, now - since) / HOUR;
    return Math.min(WAITED_MOST, Math.round(WAITED_RATE * Math.log2(1 + hours)));
}

// What a thing weighs: { weight, parts }, its parts always in the same
// order. A thing is { kind: "todo" | "look", blocks, due, ready, since },
// each time a number or an ISO time.
function weigh(thing, now) {
    const parts = [
        { id: "blocks", points: blocking(Number(thing.blocks ?? 0)) },
        { id: "due", points: dueing(when(thing.due), now) },
        { id: "ready", points: thing.ready === true ? READY : 0 },
        { id: "kind", points: KIND_POINTS[thing.kind] ?? 0 },
        { id: "waited", points: waiting(when(thing.since), now) }
    ];
    return { weight: parts.reduce((sum, part) => sum + part.points, 0), parts };
}

// The rows, each with its `weight`, heaviest first, those alike in the
// order they came, since Qt's engine's own sort doesn't keep it.
function ordered(rows) {
    return Order.sorted(rows, (one, other) => other.weight - one.weight);
}

// The kinds For You gives a row of their own after what needs Joel now,
// in the order the design lists them (VKCF3Z, 2026-11-09; Joel: "there is
// no to look at section"): To Look At, then To Try.
const OWN_ROWS = ["look", "try"];

// Whether a thing needs Joel now, whatever its kind, as a critical notice
// is lifted over the rest: it blocks a session, or its deadline has
// passed or comes within the day, as its weight counts one.
function pressing(thing, now) {
    return Number(thing.blocks ?? 0) > 0 || dueing(when(thing.due), now) > 0;
}

// The band a thing of a kind stands in: a to-do, or anything pressing,
// among what needs Joel now, `now`; a look or a thing to try otherwise in
// its kind's own row, `look` or `try`.
function bandOf(kind, thing, now) {
    return OWN_ROWS.includes(kind) && !pressing(thing, now) ? kind : "now";
}

// For You's items in their bands, { now, look, try }, each item
// { kind, thing } with whatever else its caller keeps, and given its
// `weight`. Each band is heaviest first, those alike the oldest first,
// then in the order they came (VKCF3Z, 2026-11-09). Each item's time is
// read once, beside it, rather than at each of the sort's comparisons,
// since To Try's landings say theirs as text and number in the hundreds.
// A stopgap until the agents daemon serves Landed's rows, their times
// numbers (KP0L6A, 2026-11-12).
function banded(items, now) {
    const bands = { now: [], look: [], try: [] };
    for (const item of items ?? []) {
        bands[bandOf(item.kind, item.thing, now)].push({
            row: Object.assign({}, item, { weight: weigh(item.thing, now).weight }),
            since: when(item.thing.since)
        });
    }
    const order = Order.then((one, other) => other.row.weight - one.row.weight,
        Order.smaller(one => one.since));
    const sorted = band => Order.sorted(band, order).map(one => one.row);
    return { now: sorted(bands.now), look: sorted(bands.look), try: sorted(bands.try) };
}

// How many each band holds, as items for a row's side: "1 to do \u00b7 44
// to look at \u00b7 2 to try", a band holding none left out, what needs Joel
// now needed most, so short of room To Try gives way first, then To Look
// At. For You's own row counts all three; home's Now row says the last
// two only while nothing needs him now, since then it leads with what
// he acts on (VKCF3Z, 3a92c450). `counts` holds `now`, `look` and
// `try`, any left out counting none; `said` holds the words.
function bandsRun(counts, said) {
    return [["now", "toDo", 3], ["look", "toLookAt", 2], ["try", "toTry", 1]]
        .filter(([band]) => (counts[band] ?? 0) > 0)
        .map(([band, word, keep]) => ({ said: filled(said[word], { count: counts[band] }),
            keep }));
}

// What For You's drawer names as the heaviest it holds (VKCF3Z,
// 16efb1dc): what needs Joel now as home's Now row leads with it, its
// title and minutes but never how many more, which `lead` keeps third
// ("Answer the ask \u00b7 2 min"), else the heaviest To Look At holds,
// by `heaviestLook`, its name. Holding only things to try, it names
// none, and For You's row has no drawer.
function heaviest(lead, heaviestLook) {
    const first = (lead ?? []).filter(item => item.keep < 3).map(item => item.said);
    return first.length > 0 ? first.join(" \u00b7 ") : heaviestLook ?? "";
}

// What For You reads of a session wanting Joel, as text, so a move of a
// session that changes none of it, as its working or its step, is heard
// as none: the node's digest of its record (`told`, everything but how
// it works now, mycelium's session records) where it carries one, else
// the fields For You reads, its title and want and the names it goes by.
function toldOf(session) {
    const address = String(session?.address ?? "");
    if (typeof session?.told === "string" && session.told !== "") {
        return `${address} ${session.told}`;
    }
    const presence = session?.presence ?? {};
    return JSON.stringify([address, session?.title ?? "", presence.session ?? "",
        presence.addresses ?? [], presence.owns ?? [], presence.wants ?? null]);
}

// Whether a look was left by a session, by the session or the address the
// look names, matched against the session's address, its presence's
// session or any address its presence answers to.
function leftBy(one, session) {
    const names = [one.session, one.address].filter(name => typeof name === "string"
        && name !== "");
    return names.some(name => name === session.address
        || name === session.presence?.session
        || (session.presence?.addresses ?? []).includes(name));
}

// A look as it's weighed and banded: a thing to look at, waiting since it
// was left, due by its deadline where its file says one, and blocking the
// session that left it while that session is among those `wanting` Joel,
// since nothing says what else it waits on, so such a look stands among
// what needs him now (VKCF3Z, 2026-11-09).
function lookThing(one, wanting) {
    return {
        kind: "look",
        blocks: (wanting ?? []).filter(session => leftBy(one, session)).length,
        due: one.due ?? null,
        since: one.since
    };
}

// How many sessions an item blocks: those whose want is an action of the
// item's quest or its file's address, since a session's presence mirrors
// what its quest has waiting on Joel (Waiting.js's asksJoel).
function blocked(item, sessions) {
    return (sessions ?? []).filter(session => {
        const presence = session.presence ?? {};
        if (presence.wants?.kind !== "action") {
            return false;
        }
        const names = (presence.owns ?? []).concat(presence.addresses ?? []);
        return names.includes(item.quest) || names.includes(item.from);
    }).length;
}

// A template filled in: each {key} by its value.
function filled(template, fill) {
    let words = String(template ?? "");
    for (const key of Object.keys(fill)) {
        words = words.split(`{${key}}`).join(String(fill[key]));
    }
    return words;
}

// What blocking says: "blocks 1 session", "blocks 2 sessions".
function blocksSaid(count, say) {
    return filled(count === 1 ? say.blocks : say.blocksMany, { count });
}

// What a deadline says: "due 15:00", or "overdue since 15:00".
function dueSaid(due, now, say) {
    return filled(due <= now ? say.overdue : say.due, { time: say.clock(due) });
}

// The strongest reason a thing is there, for its row's side: what it
// blocks, else when it's due, else how long ago it came, else nothing.
// `say` holds the words (words.json's on:waiting said) with `clock(ms)`
// and `ago(ms)`.
function reason(thing, now, say) {
    const blocks = Number(thing.blocks ?? 0);
    if (blocks > 0) {
        return blocksSaid(blocks, say);
    }
    const due = when(thing.due);
    if (due !== null) {
        return dueSaid(due, now, say);
    }
    // How long ago within the day, and before it the day, as When says one
    // ("yesterday"), never a count of days (VKCF3Z, 2026-11-09).
    const since = when(thing.since);
    if (since === null) {
        return "";
    }
    const day = say.day(since, now);
    return day === "" ? say.ago(since) : day;
}

// A span waited, in its largest whole unit: "12 min", "2 h", "3 d".
function span(length, say) {
    if (length < HOUR) {
        return filled(say.spanMinutes, { count: Math.floor(Math.max(0, length) / MINUTE) });
    }
    if (length < DAY) {
        return filled(say.spanHours, { count: Math.floor(length / HOUR) });
    }
    return filled(say.spanDays, { count: Math.floor(length / DAY) });
}

// The weight as the drawer's fact says it: the total, then each part that
// counts and says something of its own, in words. Being ready and its
// kind count, but say nothing the row and the eyebrow don't already
// (VKCF3Z, 2026-11-08: "40 \u00b7 due tomorrow 15:00").
function weightSaid(thing, now, say) {
    const weighed = weigh(thing, now);
    const words = {
        blocks: () => blocksSaid(Number(thing.blocks), say),
        due: () => dueSaid(when(thing.due), now, say),
        ready: () => "",
        kind: () => "",
        waited: () => filled(say.waitedFor, { span: span(now - when(thing.since), say) })
    };
    const said = weighed.parts.filter(part => part.points > 0).map(part => words[part.id]());
    return [String(weighed.weight)].concat(said.filter(one => one !== "")).join(" \u00b7 ");
}

// A moment as For You says every one, its day as When.js says a day,
// back or forward, then its time, and today its time alone: "15:00",
// "tomorrow 15:00", "Wednesday 14:10", "28 Nov 09:30" (VKCF3Z,
// 2026-11-08: one moment reads one way everywhere, as When.js writes it).
// `at` is milliseconds or an ISO time; "" for none. `say` holds its
// words, with `day(ms, now)` as When.js says a day, "" today, and
// `time(ms)` as the clock does. It is When.moment's rule with its day
// and clock handed in rather than read from When, so tests/for-you.qml
// can give them in UTC and hold in any timezone; Now.qml hands in
// When.day and When.clock, so the shell says it as When does.
function moment(at, now, say) {
    const read = when(at);
    if (read === null) {
        return "";
    }
    const day = say.day(read, now);
    return day === "" ? say.time(read) : filled(say.moment, { day, time: say.time(read) });
}

// A drawer's session, and how its quest stands where it isn't active,
// as Agents says it (SessionDrawer.js): "pi \u00b7 core \u00b7 paused". Its
// quest is the eyebrow's, so no fact names it again (VKCF3Z, 2026-11-08);
// with no session, how it stands has no line to go on.
function sessionSaid(title, status) {
    const stands = status && status !== "active" ? status : "";
    return title === "" ? "" : [title, stands].filter(part => part !== "").join(" \u00b7 ");
}

// Whether an item's deadline is said already, by its row's side, as its
// owner tells (`sideSaysDue`), or by its weight, which says it while it's
// within the day or past.
function dueSaidElsewhere(item, sideSaysDue, now) {
    const due = when(item.deadline);
    return due !== null && (sideSaysDue === true || dueing(due, now) > 0);
}

// What an item's owner said of it that its drawer shows beyond its weight
// and how long it's waited (VKCF3Z, 2026-11-08): when it's due, only
// where neither the row's side nor its weight says it (no fact repeats
// another), and when it's set for, each a moment; the machine it's done
// on, as its owner names it; the window it's done in, by its title, its
// session being a fact of its own; and, as written, its
// owner's address and the command that checks it's done. Each is ""
// where nothing says it. `told` holds `now`, whether its row's side
// says its deadline (`sideSaysDue`), as a weighed row's does while it
// blocks nothing, and the open `window`'s title; `say` the words, as
// `moment` takes them.
function figures(item, told, say) {
    const named = value => typeof value === "string" ? value : "";
    return {
        due: dueSaidElsewhere(item, told.sideSaysDue, told.now) ? ""
            : moment(item.deadline, told.now, say),
        setFor: moment(item.at, told.now, say),
        machine: named(item.where?.machine),
        where: named(told.window),
        address: named(item.owner?.address),
        check: named(item.doneWhen?.check)
    };
}

// Whether a file's JSON is a look this reads whole: version 1, with what
// it's about and how it stands.
function whole(read) {
    const named = value => typeof value === "string" && value !== "";
    return read !== null && typeof read === "object" && read.v === VERSION && named(read.id)
        && named(read.title) && named(read.target) && KINDS.includes(read.kind)
        && STATES.includes(read.state);
}

// One look from its file's text, or null for one that can't be read
// whole: not JSON, not version 1, or missing what it's about.
function look(text) {
    let read = null;
    try {
        read = JSON.parse(text);
    } catch (_malformed) {
        return null;
    }
    if (!whole(read)) {
        return null;
    }
    return Object.assign({}, read, {
        why: typeof read.why === "string" ? read.why : "",
        since: when(read.at),
        resolved: when(read.resolvedAt)
    });
}

// The looks folder's files, each { file, text }, as { waiting, history }:
// those waiting in the order they came, the oldest first, and the rest
// newest first by when each was resolved. A file still being written,
// `*.writing`, is left out, and so is one that can't be read.
function looks(files) {
    const read = (files ?? []).filter(one => !String(one.file ?? "").endsWith(".writing"))
        .map(one => {
            const each = look(one.text);
            return each === null ? null : Object.assign(each, { file: String(one.file ?? "") });
        }).filter(one => one !== null);
    return {
        waiting: Order.sorted(read.filter(one => one.state === "waiting"),
            Order.smaller(one => one.since)),
        history: Order.sorted(read.filter(one => one.state !== "waiting"),
            Order.stream(one => one.resolved ?? one.since))
    };
}

// What the launcher's sources give For You beside its own, each source's
// `forYou` a list, as Passwords gives its one to-do for what its last
// check found: { waiting, done }, each still to do { row, thing } with the
// thing weigh reads, and each done { row, done: { how, at } }, its time a
// number. One saying neither, or a source giving no list, is left out.
function given(sources) {
    // Qt's engine has no flatMap, so the lists are joined by concat.
    const lists = (sources ?? []).map(one => Array.isArray(one?.forYou) ? one.forYou : []);
    const each = [].concat(...lists)
        .filter(one => one !== null && typeof one === "object" && one.row);
    return {
        waiting: each.filter(one => !one.done && one.thing),
        done: each.filter(one => one.done && when(one.done.at) !== null)
            .map(one => ({ row: one.row, done: { how: String(one.done.how ?? ""),
                at: when(one.done.at) } }))
    };
}

// The to-dos Joel said were done, for History, newest first: each
// { item, resolved }, when he said it, as settled.json keeps it. What's
// kept of an item's words goes with its quest's file, so it's there only
// while that is. One the to do daemon found done isn't Joel's word, and
// its verdict says how in no words he reads, so it isn't here.
function settledItems(asked, said, identity) {
    const done = said?.done ?? {};
    const resolved = (asked ?? []).map(item => {
        const at = when(done[identity(item)]);
        return at === null ? null : { item, resolved: at };
    }).filter(one => one !== null);
    return Order.sorted(resolved, Order.stream(one => one.resolved));
}

// What a look in History says at its side: how it went and when, "merged
// 14:02", or its state where it says no how.
function historySide(one, say) {
    const how = typeof one.how === "string" && one.how !== "" ? one.how : say[one.state] ?? "";
    const at = one.resolved ?? one.since;
    return filled(say.history, { how, time: at === null ? "" : say.clock(at) }).trim();
}

// A look's file as it's written again once it's done with, whole, opened
// by Joel or cleared: everything it said, and how and when it ended.
function rewritten(one, state, how, now) {
    const kept = {};
    for (const key of Object.keys(one).filter(name => !OWN.includes(name))) {
        kept[key] = one[key];
    }
    return JSON.stringify(Object.assign(kept, { state, how,
        resolvedAt: new Date(now).toISOString() }), null, 2);
}
