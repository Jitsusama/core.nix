// What the launcher's journal says, apart from the shell, so tests/pacing.qml holds it in Qt's own
// engine. Two things:
// - How long what the launcher showed took to reach niri's screen, as its journal says it as it
//   closes: keys from their press, a daemon's changes from the line that made them, each place on
//   its own (Y2YBTS, 2026-11-11), and a drawer from the key that selected its row, all against
//   Joel's 50 ms bar (Joel, 2026-11-11 20:28: "the drawer in the launcher and its time to appear
//   needs to be kept to the same performance requirements as everything else").
//   ChangesPresented.js says a daemon's changes with these.
// - What a line may say with reports off (Options.reports), by the shapes and words KeysSaid.js
//   holds: a key by its kind and words typed by how many letters, since each may carry what Joel
//   typed or a window's title. ActsSaid.js says an act, a daemon's refusal and its error so.
.pragma library
.import "KeysSaid.js" as KeysSaid

const bar = 50;

// How long each of a set took, from `from` to `at`: how many, the median, the slowest, the p99 by
// nearest rank and how many within the bar. The daily loop reads a day's p99 from these lines,
// since one slow key in a hundred is what a median hides and the slowest overstates (7QEY1S,
// 2026-11-11).
function took(times) {
    const ms = times.map(one => Math.round(one.at - one.from)).sort((a, b) => a - b);
    return {
        count: ms.length,
        median: median(ms),
        slowest: ms[ms.length - 1],
        p99: ms[Math.max(0, Math.ceil(ms.length * 0.99) - 1)],
        within: ms.filter(each => each <= bar).length
    };
}

// Each key's whole milliseconds from its press to the screen, in the order pressed, which an
// opening's trial keeps so health can pool a day's keys for their p99, as one opening's p99 can't
// be pooled (1JQPPC, 2026-11-12).
function eachTook(times) {
    return times.slice().sort((a, b) => a.from - b.from).map(one => Math.round(one.at - one.from));
}

// How long a set took, as every line here says it after what it timed: `verb` as it came,
// presented or landed, and `from` what it's timed from.
function tookSaid(one, from, verb) {
    return `${verb ?? "presented"} a median ${one.median} ms from ${from}, `
        + `the slowest ${one.slowest} ms, `
        + `the p99 ${one.p99} ms, ${one.within} within ${bar} ms`;
}

function median(ms) {
    const sorted = ms.slice().sort((a, b) => a - b);
    return sorted[(sorted.length - 1) >> 1];
}

// A monotonic time to a tenth of a millisecond, as niri's own log says its frames' times, so the
// lines here that say one line up with it.
function tenths(ms) {
    return (Math.round(ms * 10) / 10).toFixed(1);
}

// What the journal says of the keys presented, each `{from, at}` and, once its frame began,
// `heard` and `begun`, or of those `verb` names landed: how long they took, and with `apart` where
// that went, each part's median, so a key whose own frame came in 20 ms but was presented 100 ms
// after its press says whether it waited to be heard, behind a busy shell, or after its frame, in
// niri (ZINAWZ on Joel's 12:06 opening, 2026-11-11).
function keysSaid(verb, times, apart) {
    const one = took(times);
    if (one.count === 0) {
        return "";
    }
    const said = `launcher: ${one.count} keys ${tookSaid(one, "the key", verb)}`;
    return apart ? said + wentTo(times) + screensSaid(times) : said;
}

// Which screen a set of keys was shown on, each `screen` its name and `refresh` niri's frame in ms
// there (frame-presentation.patch's timesOf), said after the rest as "; on DP-4 at 60 Hz", each
// screen once, in turn, at the rate of any key niri timed there, bare where niri timed none, as on
// a nested niri's winit screen, or "" where none was named. A key on a 60 Hz screen waits twice as
// long for niri's frame as on a 120 Hz one, so a day docked read as a day the launcher slowed: its
// keys past 50 ms went from 11% to 60% when Joel docked to a 60 Hz screen (16 Nov, 10:22).
function screensSaid(times) {
    const named = times.filter(one => one.screen).map(one => one.screen);
    const screens = named.filter((one, at) => named.indexOf(one) === at).map(screen => {
        const timed = times.find(one => one.screen === screen && one.refresh > 0);
        return timed ? `${screen} at ${Math.round(1000 / timed.refresh)} Hz` : screen;
    });
    return screens.length === 0 ? "" : `; on ${screens.join(" and ")}`;
}

// Where a set's time went, each part's median over those stamped with when the shell heard it and
// when its frame began, or "" where none was: a key from its press.
function wentTo(times) {
    return partsSwapped(times, [["from", "heard", "before the shell heard it"],
        ["heard", "begun", "in the shell"], ["begun", "at", "from its frame's start"]]);
}

// Each part's median as partsSaid says them, and where the time from the frame's start went, over
// those whose frame's swap the render thread timed (frame-presentation.patch's timesOf,
// `swapped`): to its swap, which holds the shell's polish, sync and render and any wait the swap
// made for the frame before it, then after it, from niri taking the frame to showing it. Joel's
// drawers spent 45 to 48 ms from their frame's start to the screen, whatever they held, and
// nothing said whose those were (KP0L6A, 2026-11-12).
function partsSwapped(times, parts) {
    const said = partsSaid(times, parts);
    const timed = times.filter(each => parts.every(([from, to]) => each[from] !== undefined
        && each[to] !== undefined) && each.swapped > 0);
    if (said === "" || timed.length === 0) {
        return said;
    }
    const toSwap = median(timed.map(each => Math.round(each.swapped - each.begun)));
    const after = median(timed.map(each => Math.round(each.at - each.swapped)));
    return `${said} (${toSwap} to its swap and ${after} after it)`;
}

// How many of niri's frames of `refresh` ms (frame-presentation.patch's timesOf) passed from an
// opening's swap until `at`, the vblank niri showed it at, counted back along the vblanks from it.
// Joel's openings had their GPU work done a median 1 ms after their swap, then waited a median 6
// frames at 120 Hz for niri (2026-11-15), so the count is what a fix in niri moves. "" where niri
// gave no refresh or the swap wasn't timed. The swap and the vblank are said too, in
// CLOCK_MONOTONIC ms as the patch reads both, so they line up with niri's own log of when it asked
// for each frame: its frames reached the screen within two refreshes of its ask, so the wait lies
// between the swap and the ask (LR3WMN, 2026-11-19).
function shownSaid(times, at) {
    const refresh = times.refresh ?? 0;
    if (!(refresh > 0) || !(times.swapped > 0)) {
        return "";
    }
    const hz = Math.round(1000 / refresh);
    // A hundredth of a frame's slack, for a time read a little apart from the vblank it lies on.
    const count = Math.max(1, Math.ceil((at - times.swapped) / refresh - 0.01));
    const frames = `${count} ${count === 1 ? "frame" : "frames"} at ${hz} Hz`;
    return `; ${frames} from its swap to the screen`
        + ` (swapped ${tenths(times.swapped)}, shown ${tenths(at)} monotonic ms)`;
}

// When each key was pressed, its frame swapped and niri showed it, in CLOCK_MONOTONIC ms as
// KeyTimes and frame-presentation.patch read them, in the order pressed, or "" for no keys. The
// line the keys' medians make can't be lined up with niri's own log of each frame it asked for: on
// the 60 Hz screen keys spent a median 16 ms from their swap to the screen, against 11 on the 120
// Hz panel, and which frames waited needs each key's times (LR3WMN, 2026-11-19). niri says a
// frame's steps only for one asked for after 100 ms idle (says-a-frames-steps-after-idle.patch),
// so in a burst of keys the first has a line of niri's to match and the rest don't. A press niri
// gave no time for, as a typed one's, reads "-", as does a swap the render thread didn't time or
// no longer keeps, 16 of the shell's frames on (`stamped`, `swapped`); a held key's repeat is
// timed from when Qt made it, as keytimes.cpp says, and reads as a press. A press is said in whole
// ms, as niri stamps it: keytimes.cpp's fraction past it is when the shell handled the key, not
// when it was pressed. A key niri answered for on a later frame of the shell's than its own says
// how many later (`later`, keyClockOf), since its swap is its own frame's and its showing the
// later one's; those are the shell's frames, not niri's refreshes. Only keys niri showed are said,
// the last 300 (keysKept). Keys typed into an ask are never timed (Launcher.qml's searchedNow), so
// none is secret.
function keysClockSaid(times) {
    if (times.length === 0) {
        return "";
    }
    const each = times.slice().sort((a, b) => a.from - b.from).map(one => {
        const pressed = one.stamped === true ? String(Math.floor(one.from)) : "-";
        const swapped = one.swapped > 0 ? tenths(one.swapped) : "-";
        const later = one.later > 0
            ? `, ${one.later} ${one.later === 1 ? "frame" : "frames"} of the shell's after its own`
            : "";
        return `${pressed} ${swapped} ${tenths(one.at)}${later}`;
    });
    return `launcher: ${times.length} keys shown, each pressed, swapped and shown in monotonic ms: `
        + each.join("; ");
}

// Whether niri gave a key's press a time (keyTimed's `stamped`), and how many frames after the
// key's own the one niri answered for came (`later`). A record that doesn't say is untimed, so a
// heard time is never said as niri's press. That is no count of frames niri dropped: when niri
// drops one the shell asks again only once it hears, and the frame begun before that goes untimed,
// shown or not (frame-presentation.patch asks once).
function keyClockOf(key, answered) {
    const later = key.frame !== undefined && answered > key.frame ? answered - key.frame : 0;
    return { stamped: key.stamped === true, later };
}

// Each part's median, `[from, to, words]`, over those stamped with every part's ends, or "" where
// none was, said as ": a median A ms <words>, B <words> and C <words>".
function partsSaid(times, parts) {
    const split = times.filter(each => parts.every(([from, to]) => each[from] !== undefined
        && each[to] !== undefined));
    if (split.length === 0) {
        return "";
    }
    const said = parts.map(([from, to, words]) =>
        `${median(split.map(each => Math.round(each[to] - each[from])))} ${words}`);
    return `: a median ${said[0].replace(" ", " ms ")}`
        + said.slice(1, -1).map(one => `, ${one}`).join("") + ` and ${said[said.length - 1]}`;
}

// Where a drawer's time went from the key that selected its row: waiting for the selection to be
// followed, then until its frame began, which holds making it, then from its frame's start to the
// screen.
const drawerParts = [["from", "followed", "before it followed"],
    ["followed", "begun", "until its frame began"], ["begun", "at", "from its frame's start"]];

// A drawer presented, `{ key, place, from, followed, begun, swapped, at, screen, refresh }`, said
// as it's presented, as an opening is, so each one's time from Joel's key can be read as he goes.
// The screen comes last, as a key's does, so what reads the words before it still does: a drawer
// on the 60 Hz screen took 54 ms, on the 120 Hz panel 33, and the day's median followed which he
// worked on (2026-11-17).
function drawerSaid(one) {
    return `launcher: drawer for ${quoted(one.key)} presented ${Math.round(one.at - one.from)} ms `
        + `from the key${partsSwapped([one], drawerParts)}, ${one.came ?? "the frame after"}`
        + screensSaid([one]);
}

// The keys gone into a drawer, `{ key, page, from, at, swapped, laid, screen, refresh }`, said as
// the frame they came in is presented, with how much of it was the shell's, to that frame's swap
// where it was kept, and how many of its lines that frame laid: Ctrl+W l into Emoji's laid all
// 1,833 lines first, where its view shows some 37, until only those were (2026-11-19). The page is
// named, since going back from its list of keys is a going-in under the same row.
function intoSaid(one) {
    const swap = one.swapped > 0
        ? `, ${Math.round(one.swapped - one.from)} to the shell's swap` : "";
    const key = quoted(one.key);
    const page = one.page ? ` on ${quoted(one.page)}` : "";
    return `launcher: into the drawer for ${key}${page} presented `
        + `${Math.round(one.at - one.from)} ms from the key${swap}, ${one.laid} `
        + `${one.laid === 1 ? "line" : "lines"} laid first${screensSaid([one])}`;
}

// A slow drawer's time from the work begun for it to the frame that drew it, said as a slow key
// is, by its row's key, quoted so a title in it can't end the line early or forge another: `parts`
// what of it went on making it, `split` what of that went on which, as Trials.makingSplit says it,
// then waiting for its frame and drawing it.
function drawerTook(key, ms, parts, split) {
    return `launcher: drawer for ${quoted(key)} took ${ms} ms to its frame,`
        + ` ${parts.making} ms making${split}, ${parts.waiting} waiting for its frame,`
        + ` ${parts.drawing} drawing it, ${parts.came}`;
}

// A row's key quoted as JSON, so a quote or a line break in a row's name can't end the line early
// or forge another, with the line ends JSON leaves raw escaped too, since a reader may split on
// any of them.
function quoted(key) {
    return JSON.stringify(key).replace(/[\u2028\u2029\u0085]/g,
        end => `\\u${end.charCodeAt(0).toString(16).padStart(4, "0")}`);
}

// A key in a drawer, `{ act, key, from, into, at, swapped, screen, refresh }`, said as its frame
// is presented: from its press, or where it had none from when it was heard, and from going in
// (`into`) where it came soon after, so a key typed a set time after Ctrl+W l says its whole wait,
// queued behind the drawer's lay or not, as that less the time it was typed after.
function keyInSaid(one) {
    const swap = one.swapped > 0
        ? `, ${Math.round(one.swapped - one.from)} to the shell's swap` : "";
    const into = one.into === undefined ? "" : `, ${Math.round(one.at - one.into)} from going in`;
    return `launcher: ${one.act} in the drawer for ${quoted(one.key)} presented `
        + `${Math.round(one.at - one.from)} ms from the key${swap}${into}${screensSaid([one])}`;
}

// How long after going in a key in a drawer still says its time from it.
const intoSoon = 1000;

// When a key pressed at `keyAt` says its time from going in at `into`, both on KeyTimes's clock:
// always where `whole`, as while reports are on and the drawer lab reads it for every key, however
// long the lay held it; otherwise only within a second of going in, which is what that time is
// read for, since two minutes on it would read as a fault. Undefined where it says none.
function intoKept(keyAt, into, whole) {
    return whole || keyAt - into < intoSoon ? into : undefined;
}

// Which of the launcher's own kinds a key is of, as `place:`, or undefined where it's of none.
function ownKind(key) {
    const text = String(key);
    return KeysSaid.keysOwn.find(kind => text.startsWith(kind));
}

// The key a served row is made with (ReadyRows.qml): its thing, else its key, as its daemon
// wrote them. One claiming a kind of the launcher's own, as `place:`, is kept under `row:` as a
// daemon's record is, since keyShown says a key of those kinds whole as the launcher's id, and a
// daemon's may hold a name. Kept here beside ownKind, as ReadyRows.js is at biome's length.
function keyMade(row, key) {
    const made = row.thing ?? key;
    return ownKind(made) === undefined ? made : `row:${made}`;
}

// A program's own id, as a power profile's hold names it: whole where `whole`, else where it's
// shaped as an app's, and otherwise as app, since a program may give any words as its id; bare,
// with what quoting escapes escaped, as ActsSaid.js's actShown, so a line break can't forge one.
function appShown(app, whole) {
    const text = String(app ?? "");
    return whole ? quoted(text).slice(1, -1)
        : KeysSaid.appWord.test(text) && !KeysSaid.appsUnnamed.includes(text) ? text : "app";
}

// A key's parts up to the first that isn't a kind's word, among its first `kinds`, or an act's,
// after them.
function kindsLed(parts, kinds) {
    const end = parts.findIndex((part, at) =>
        (at < kinds ? !KeysSaid.kindWord.test(part) : !KeysSaid.actWords.has(part)));
    return end < 0 ? parts : parts.slice(0, end);
}

// A row's key as a line in the journal may say it: whole where `whole`, as while reports are on;
// otherwise its kind, after a record's `row:`, as wifid's `row:wifi:network:<name>` is `wifi`,
// since most keys carry what someone else wrote, a network's name or a window's title. The
// launcher's own ids are whole, and an own kind's key holding more than an id is said by that kind
// alone. `window:<app>:<title>` is `window:<app>`, and the programs daemon's `program:<id>` whole,
// so each program's drawer is timed apart (RTOIMQ, 2026-11-24), only where its id is shaped as an
// app's and else by its kind, as another daemon's so keyed. A `verb:` key, its daemon's own words,
// goes to its kind, and its act where that's one of KeysSaid's actWords: `verb:agent:close` whole,
// `verb:disk:nvme0:/home` as `verb:disk`, `verb:notice:offer:<id>` as `verb:notice:offer`. A new
// kind says nothing more until it's named here, and a first part that isn't a kind's word, as a
// name with a colon in it, nothing at all. A number is read as text.
function keyShown(key, whole) {
    const text = String(key);
    if (whole) {
        return text;
    }
    const own = ownKind(text);
    if (own !== undefined) {
        return KeysSaid.ownId.test(text.slice(own.length))
            ? [...text].slice(0, KeysSaid.ownLength).join("")
            : own.slice(0, -1);
    }
    const record = text.startsWith("row:");
    const parts = (record ? text.slice(4) : text).split(":");
    if (parts[0] === "window") {
        const app = parts.length > 2 ? parts[1] : "";
        return KeysSaid.appWord.test(app) && !KeysSaid.appsUnnamed.includes(app)
            ? `window:${app}` : "window";
    }
    if (parts[0] === "program" && !record) {
        const id = parts.length === 2 ? parts[1] : "";
        return KeysSaid.appWord.test(id) && !KeysSaid.appsUnnamed.includes(id)
            ? `program:${id}` : "program";
    }
    if (parts[0] === "verb") {
        return kindsLed(parts.slice(0, 3), 2).join(":");
    }
    return parts.length > 1 && KeysSaid.kindWord.test(parts[0]) ? parts[0] : "";
}

// The row a drawer is for as the journal may say it: by its key, or by its name where it has none,
// as a place's own answers have, but only while reports are on, since a name is what a source
// wrote. Otherwise such a row is said as `answer`, and one whose key has no kind, as the
// stopwatch's, as `row`, the launcher's own words, since a daemon's key may be a record's name. A
// row with neither, or no row at all, is `row` whether reports are on or off: so a drawer is never
// said as `drawer for ""`, and the place it's tallied in tells whose.
function rowShown(row, whole) {
    let said = "";
    if (whole) {
        said = row?.key || row?.name;
    } else if (row?.key) {
        said = keyShown(row.key, false);
    } else if (row?.name) {
        said = "answer";
    }
    return said ? String(said) : "row";
}

// Words typed as a line in the journal may say them: quoted where `whole`, as while reports are
// on, so they can't end the line early or forge another; otherwise only how many letters, since
// what's typed may be anything Joel searches for, and how many is what a search's time turns on.
function typedShown(typed, whole) {
    if (whole) {
        return quoted(String(typed));
    }
    const letters = [...String(typed)].length;
    return letters === 1 ? "1 letter" : `${letters} letters`;
}

// A slow key's line says what was typed as typedShown does, or for a step ("→ Programs") its
// arrow and, where not `whole`, the row it took by `key` where keyShown says that whole, a
// launcher's own place or a program by its id, so each program's page is timed apart (RTOIMQ,
// 2026-11-26), and otherwise the row's name counted, so a step still reads as one.
function timedShown(typed, step, whole, key) {
    if (!step || whole) {
        return typedShown(typed, whole);
    }
    const text = String(key ?? "");
    const own = text.startsWith("program:")
        || KeysSaid.keysOwn.some(kind => text.startsWith(kind));
    const said = own && keyShown(text, false) === text ? text : typedShown(step.slice(2), false);
    return `${step.slice(0, 1)} ${said}`;
}

// A key listed before every place it waited on answered, as the journal says it: its words as
// typedShown says them, the places that hadn't answered, and how long its list waited.
function listedWithoutSaid(typed, places, waited, whole) {
    return `${typedShown(typed, whole)} listed without ${places.join(", ")},`
        + ` which hadn't answered in ${waited} ms`;
}

// A place's answer come after its key was listed without it, as the journal says it: the words as
// typedShown says them, how late it came, and how long the list waited.
function answeredLateSaid(place, typed, { late, waited, whole }) {
    return `${place} answered ${typedShown(typed, whole)} ${late} ms after its key, past the`
        + ` ${waited} ms its list waited`;
}

// Words typed inside a row's step (ready-rows.md's within) as a line may say them, as typedShown
// does, and the row quoted, by its key as a drawer's is.
function typedInSaid(typed, within, whole) {
    return `${typedShown(typed, whole)} typed in ${quoted(rowShown({ key: within }, whole))}`;
}

// The drawers presented since the launcher opened, a line a place, as a place's changes are, each
// ending with the screens they were shown on.
function drawersSaid(drawers) {
    const places = [...new Set(drawers.map(one => one.place))].sort();
    return places.map(place => {
        const its = drawers.filter(each => each.place === place);
        const one = took(its);
        return `launcher: ${one.count} ${one.count === 1 ? "drawer" : "drawers"} in ${place} `
            + `${tookSaid(one, "the key")}${partsSwapped(its, drawerParts)}${screensSaid(its)}`;
    });
}
