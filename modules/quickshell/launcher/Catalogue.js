// The launcher's catalogue: every place and every thing's actions, by the
// same rows the launcher runs, so whatever it gains, what asks it from
// outside gains too (pi-ask's one tool, over `qs ipc call launcher list`
// and `run`). This holds the rules; Launcher.qml walks its rows by them.
// An id is the row's path from home, each step's steady id joined by a
// "\u203a", which no key holds, so it stays the same from one run to the
// next.
.pragma library

// How deep below home a row is listed: a place (0), what it holds (1), a
// thing's actions or parts (2), a setting's choices (3), and one more.
const DEEPEST = 4;

// Collections whose things are a list of items, each listed whole only as
// a place: what was copied, the emoji, the notices past, the shortcuts,
// and what comes and goes in Agents, the queue's jobs, AI spend's findings and
// the subagents' runs. A model asked to paste or find one opens the place
// instead.
const LISTS = new Set(["clipboard", "emoji", "notices", "shortcuts", "characters", "jobs",
    "spend", "runs"]);

// Of those, the ones long enough that a walk over every row leaves them
// out: their rows are each made one way, and proven offscreen.
const LONG = new Set(["clipboard", "emoji", "characters"]);

// Whether a collection is one of the long lists.
function long(source) {
    return LONG.has(source);
}

// Collections whose things are listed, each one opened by its own run, but
// not what each holds: every program's Quit and New window, every window's
// Size, Float and workspaces, and every quest's own, would bury the rest a
// few hundred times over. The window Joel came from keeps its own, as This
// window.
const WHOLE = new Set(["applications", "windows", "quests"]);

// What a keyboard's key does is a lookup, not something done to the
// machine, and the boards' keys are a few hundred rows.
const LOOKUPS = "keyboard:key:";

// What a row's verb says it does that can't be taken back, or that ends
// something in use: a window or a program closed, a call left, a session
// locked or ended, a radio or a device turned off or forgotten, the
// screen shared, firmware installed or a security setting fwupd fixes or
// puts back. Those are offered, never done, from outside.
const DISRUPTIVE = new Set(["close", "quit", "leave", "lock", "sleep", "reboot", "stop", "off",
    "turn off", "forget", "disconnect", "share", "install", "fix", "undo"]);

// The verb a row says now: a word, or a getter's; for one naming the act
// its Return does (`returns`, Launcher.qml's returnOf), that act's.
function verbOf(row) {
    if (row.returns) {
        return verbOf(row.returns);
    }
    const verb = row.verb;
    return typeof verb === "string" ? verb : "";
}

// Whether it's done only with Joel's own Return: one the launcher asks to
// be sure of, or one whose verb ends something.
function disruptive(row) {
    return !!row.confirm || DISRUPTIVE.has(verbOf(row));
}

// Whether it ends something in use, as words.json marks it (Words.ending):
// a row's ending action, as a window's Close or a network's Forget It, or
// a disconnect, which Return typed in insert never does (Joel,
// 2026-11-04: "it is quite dangerous how in insert mode i can type in a
// name of a wifi network and then hitting enter disconnects from it").
// Narrower than disruptive: a lock or a sleep is typed to be done.
function ends(row) {
    return row.act === "ending" || (row.ends ?? "") !== "";
}

// Whether Return over visual's selection opens it: a row Return takes
// somewhere, as a program's or a window's, and never one that changes
// something, as one that stays to show its new state does (Joel,
// 2026-10-26), nor one that ends or disrupts, nor a place, which Return
// steps into, since Return in visual never changes anything (Y2YBTS's
// yes, 2026-11-04); and never one the key language says changes, given
// the row's command as changesState is, so Return on one row and over a
// selection agree.
function opens(row, command = {}) {
    // One whose Return is an act of its own opens as that act does, unless
    // the act asks for words, which a selection has none of.
    if (row.returns) {
        return !row.returns.given && opens(row.returns, command);
    }
    return typeof row.run === "function" && kindOf(row) === "action" && !row.stays
        && !ends(row) && !disruptive(row) && !changesState(row, command);
}

// The commands the guess holds quiet, whatever the row: it only copies,
// goes somewhere, finds, shows or holds more. The key language names none
// of them yet (modules/keys/language.toml); once it does, its flag decides,
// so tests/catalogue.qml pins the list, which only shrinks.
const quiet = ["copy", "find", "go", "more", "see", "show"];

// The command a row is named by, as the command line names its act
// (Commands.js's actLine): the one its words give it (`said`), or else its
// verb's (`verbal`, Commands.js's commandOf); and the key language's flags
// for it from `table`, where the language names it. A row whose verb alone
// says close, as an answer with nothing to copy or a bind with nothing to
// run, only closes the launcher, which the language calls :quit; a
// window's Close It has its words' :close.
function named(said, verbal, table) {
    const command = said ?? (verbal === "close" ? "quit" : verbal);
    return { command, flags: table[command] };
}

// Whether doing it changes how things stand (the vi-paradigm spec's Return),
// as the key language flags the command the row is named by (`command`,
// its command and the language's flags for it, as named gives them), so
// Return and d agree on every row. Return from the find line
// lands on such a row rather than doing it, so typing a name and Return
// can only ever show it. A row may say so itself, whatever its command.
// Where the language names no command for the row, as for the many whose
// command is read from their name, it's guessed until VCSELW's language
// names it: it ends something or can't be taken back, or it's done and
// keeps the launcher, as a network's join is, unless its command is one
// the guess holds quiet.
function changesState(row, command = {}) {
    // A row that says it changes nothing, whatever its verb, as Ask's
    // question or a file picker's mark, which only picks; and a figure,
    // which only says a value, its Return copying it or, with nothing to
    // copy yet, closing the launcher, never the :close of a window its
    // verb would name; and one that asks for words first, whose Return
    // only asks, whatever the words then do.
    if (!row || row.gone || row.changes === false || row.figure === true
        || typeof row.given === "function") {
        return false;
    }
    if (row.changes === true) {
        return true;
    }
    if (command.flags !== undefined) {
        return command.flags.changes === true;
    }
    if (ends(row) || disruptive(row)) {
        return true;
    }
    return row.stays === true && typeof row.run === "function"
        && !quiet.includes(command.command ?? "") && !row.spot && !row.from
        && !(row.verbs && row.targets);
}

// Whether Return from the find does a row's act though it changes
// something, rather than landing on it: one holding its own acts whose
// command starts something (the key language's starts), as a meeting's
// row joins from ten minutes before it and sees it before then, which the
// row says, not its act (VKCF3Z, 2026-11-05). One with no acts of its
// own, as the clipboard's Join the Call, is the act alone, which Return
// only lands on.
function startsItself(row, command = {}) {
    return !!row && row.verbs !== undefined && command.flags?.starts === true;
}

// Whether doing it ends, takes away or reaches someone, so no Return runs
// it from any mode: Return writes its command on the : line instead, and a
// fresh Return there runs it (VKCF3Z's re-spec after Joel's 11:4x, "where
// in vi does that step into shit?"). The key language's ends flag on the
// row's command decides, as `command` names it (named); :set's flag is the
// command's whatever its value, so its no form, which turns something off
// (`off`, as SetWords reads the row's value), ends too. Where the language
// names no command for it, an ending act, as words.json marks one, ends.
// What calls someone or starts something new, the language's starts flag
// (:meet, :share), goes to the line the same way, the spec's other rule
// for Return (VKCF3Z, 2026-11-05). A thing holding acts of its own (a meeting, a window, a network) opens
// on Return as ever, its ending d's among its acts, so a meeting still
// joins its call from ten minutes before it starts, though :meet starts
// something (startsItself).
// A row whose act someone else holds (its heldBy, as an organisation
// holding WARP's switch) ends nothing, since it's refused before it's
// asked: Return runs it and says why, and no d is taught (VKCF3Z,
// 2026-11-08).
function endsSomething(row, command = {}, off = false) {
    if (!row || row.gone || row.figure === true || typeof row.given === "function"
        || !!row.heldBy) {
        return false;
    }
    if (row.verbs || row.targets || row.nested || row.category) {
        return false;
    }
    if (command.flags !== undefined) {
        return command.flags.ends === true || command.flags.starts === true || off;
    }
    return ends(row) || off;
}

// A step's rows with a quiet one first, where the first would change
// something: the first row inside a thing never changes state (Show in Calendar,
// Copy, a part), so a Return that steps in and a second Return after it
// only ever show. Where every row changes something, they keep their order.
// `naming` gives each row's command as changesState takes it (named).
function quietFirst(inside, naming) {
    const changes = one => changesState(one, naming(one));
    if (inside.length < 2 || !changes(inside[0])) {
        return inside;
    }
    const still = inside.findIndex(one => !changes(one));
    if (still < 0) {
        return inside;
    }
    return [inside[still]].concat(inside.slice(0, still), inside.slice(still + 1));
}

// Whether it's a place, which is opened, or an action, which is done: a
// part holding more is a place, though Return steps into it by running.
function kindOf(row) {
    if (row.category || row.opens || verbOf(row) === "more") {
        return "place";
    }
    const holds = !!row.verbs || !!row.targets || !!row.nested;
    return holds && !row.run && !row.returns && !row.given ? "place" : "action";
}

// What a row's name says without what it's done to, "Turn it off" as
// "turn off".
const DONE_TO = / (?:it|them)\b/;

// Whether a row inside does this act: its verb is it, or its name is it
// or starts with it ("Disconnect it", "Copy what it said"). Every figure
// copies its own value, which isn't copying the row it's in, so a copy is
// known by its name alone.
function saysAct(one, said) {
    if (said !== "copy" && verbOf(one).toLowerCase() === said) {
        return true;
    }
    const name = String(one.name ?? "").toLowerCase().replace(DONE_TO, "");
    return name === said || name.startsWith(`${said} `);
}

// What a step into a row holds, as the right arrow and the catalogue
// both show it: everything it can be told, Return's own act first, even
// where its provider left it out, so the right arrow always does more
// than Return and never less (Joel's concept map: Return is the default
// act and the right arrow does more; agreed with 15J7N3 for every row).
// A row that only steps in, asks first or asks for words has no act of
// its own to add, nor does one holding nothing. One giving both its verbs
// and its targets holds rows, as a holder, a talk or a meeting's Show in Calendar
// does, so its Return is the step itself. One already inside, by its
// verb or its name, is moved first rather than said twice. A copy of its
// own wears `copyMark`, the theme's copy glyph. A row naming the act its
// Return does (`returns`) keeps its acts in their own order, as a
// session's are fixed (VKCF3Z, 2026-11-06: Go To, Talk, Pause or Resume,
// Stop, Close), its Return being one of them.
function leading(row, inside, copyMark) {
    if (row.returns) {
        return inside;
    }
    const verb = verbOf(row);
    const said = verb.toLowerCase();
    // A copy is never first inside: a row whose only act is copying steps
    // in on Return, so its copy is listed after what can be done and
    // before what's said (Joel, 2026-10-28: "the copy at the top of most
    // screens is confusing").
    const copying = said === "copy";
    const holds = !!row.verbs && !!row.targets && !copying;
    if (inside.length === 0 || kindOf(row) !== "action" || row.confirm || row.given || holds
        || said === "") {
        return inside;
    }
    const already = inside.findIndex(one => saysAct(one, said));
    if (copying) {
        return copiedLast(already >= 0 ? inside[already] : ownAct(row, verb, copyMark),
            inside.filter((_one, at) => at !== already));
    }
    if (already === 0) {
        return inside;
    }
    if (already > 0) {
        return [inside[already]].concat(inside.slice(0, already), inside.slice(already + 1));
    }
    return [ownAct(row, verb, copyMark)].concat(inside);
}

// A step into a row that does something and keeps the launcher, as a
// setting does, led by that act, which runs and closes: the right arrow
// then Return does it and closes, where Return alone does it in place, so
// no chord with Ctrl is needed for it (Joel, 2026-11-02: "all of those
// ctrl keys do not feel very vim to me"). One already inside, by its verb
// or its name, is moved first and closes rather than being said twice.
// A row that always stays (`alwaysStays`), as a power profile does, keeps
// the launcher from its step too: Return there never closes (Joel,
// 2026-11-08, a profile picked from its step closed and was lost).
function closingFirst(row, inside, copyMark) {
    const verb = verbOf(row);
    const said = verb.toLowerCase();
    const already = said === "" ? -1 : inside.findIndex(one => saysAct(one, said));
    // It no longer keeps the launcher, unless the row always does, but it
    // changes what the row did.
    const act = Object.assign({}, already >= 0 ? inside[already]
        : ownAct(row, verb, copyMark),
        { stays: row.alwaysStays === true, changes: true });
    return [act].concat(inside.filter((_one, at) => at !== already));
}

// A copy placed last, after what can be done and what's only said, as
// docs/design.md's Return rule has it: "Copy is never the first row of a
// step, unless it's the only one, and is always the last".
function copiedLast(copy, inside) {
    return inside.concat([copy]);
}

// Return's own act as a row inside, doing what Return on the row does,
// wearing its row's glyph, but for a copy, which wears `copyMark` as every
// Copy row does: a row's own glyph may say a caution, as a budget's past
// its warning does, which its copy doesn't (VKCF3Z, 2026-11-07).
function ownAct(row, verb, copyMark) {
    const said = verb.toLowerCase();
    // Every field a provider's row has (Provider.qml's `thing`), so the
    // list draws it as it draws them; the launcher prepares its name.
    return {
        key: `return:${said}`,
        name: verb.charAt(0).toUpperCase() + verb.slice(1),
        verb,
        what: "",
        chord: [],
        side: "",
        count: "",
        context: "",
        note: "",
        below: null,
        verbs: null,
        plain: false,
        later: false,
        // A copy is set while the launcher holds the keys, as the row's is.
        now: row.now === true,
        glyph: said === "copy" ? copyMark ?? "" : row.glyph ?? "",
        hue: "action",
        stays: row.stays === true,
        // Asked through the row, so a run that depends on how the row
        // stands now, a program's switch or start, is read when it's done.
        run: () => row.run()
    };
}

// Whether the catalogue lists it: a row with a name that does something
// or holds more, never a figure, a copy of what's shown, a command's
// start to finish in the field, or a thing that's gone.
function listed(row) {
    if (!row || row.gone || (row.name ?? "") === "" || row.start !== undefined) {
        return false;
    }
    if (String(row.key ?? "").startsWith(LOOKUPS)) {
        return false;
    }
    const holds = !!row.verbs || !!row.targets || !!row.nested || !!row.category || !!row.opens;
    if (holds) {
        return true;
    }
    if (row.figure === true) {
        return false;
    }
    const verb = verbOf(row);
    return verb !== "copy" && verb !== "close" && (!!row.run || !!row.given);
}

// Whether Return does something on a row (Joel, 2026-10-26: every row
// has a Return act; a row that holds more steps in, a row that only shows
// a value copies it, and a row that can do neither doesn't belong in the
// launcher). One still being read, whose value is an ellipsis, isn't held
// against it, since it copies once it's read.
function acts(row) {
    if (!row || row.gone || row.side === "\u2026") {
        return true;
    }
    const steps = !!row.verbs || !!row.targets || !!row.nested || !!row.category
        || !!row.opens || !!row.from;
    if (steps || !!row.given || !!row.returns || row.start !== undefined) {
        return true;
    }
    return !!row.run && !(row.figure === true && verbOf(row) === "close");
}

// Whether what a row holds is listed too, at `depth` below home, inside
// `above`, the row holding it, or null at home.
function descends(row, depth, above) {
    if (depth >= DEEPEST) {
        return false;
    }
    if (row.opens && LISTS.has(row.opens)) {
        return false;
    }
    if (above && WHOLE.has(above.opens)) {
        return false;
    }
    return !String(row.key ?? "").startsWith("shortcut-level:");
}

// Whether the walk for where rows live (lives) looks inside this one,
// `depth` below home, inside `above`: not Now, which holds what lives
// elsewhere, nor a long list's items or a program's or window's own, which
// are found only from their place, so the walk stays a key's worth.
function followsInto(row, depth, above) {
    // A measured figure's parts are read as they're made, Machine's CPU
    // listing the processes, so a walk on every Ctrl+] never makes them.
    if (depth >= DEEPEST || row.opens === "on" || row.parted === true) {
        return false;
    }
    if (row.opens && LONG.has(row.opens)) {
        return false;
    }
    return !(above && WHOLE.has(above.opens));
}

// Whether a row is a place, which the look for where a row lives goes
// through first (livesOf): a category, or a collection it opens.
function isPlace(row) {
    return !!row.category || !!row.opens;
}

// Where the row `id` lives, which Ctrl+] goes to as Vim follows a tag:
// `path`, the ids from one of home's `rows` down to the shallowest row
// that is it, or null. It's looked for on the press, breadth first and
// only as far as it's found: a walk of every row, kept until a provider
// changed, made 96% of People's rows (2026-11-09, `made`). `ways` says a
// row's steady id (`idOf`), the rows it holds inside the row above it
// (`holds`), and optionally whether the look goes inside it (`enters`),
// whether before the rest (`first`, as places are: the rest wait, and the
// look carries on into them, counting no row twice), and whether its
// budget is spent (`spent`, given the rows `listed`), which ends it unfound
// rather than holding the key (Y2YBTS, 2026-11-09). A provider makes a
// list whole, so the budget is asked once it's made, after each row.
function livesOf(rows, id, ways) {
    const queue = rows.map(row => ({ row, path: [ways.idOf(row)], above: null, depth: 0 }));
    let count = queue.length;
    const said = (path, spent) => ({ path, listed: count, spent });
    const found = queue.find(one => one.path[0] === id);
    if (found) {
        return said(found.path, false);
    }
    const seen = new Set();
    const waiting = [];
    for (let next = 0; next < queue.length || waiting.length > 0; next += 1) {
        if (next === queue.length) {
            queue.push(...waiting.splice(0));
        }
        const one = queue[next];
        const key = one.path[one.path.length - 1];
        if (seen.has(key) || !followsInto(one.row, one.depth, one.above)
            || (ways.enters && !ways.enters(one.row, one.above))) {
            continue;
        }
        if (!one.later && ways.first && !ways.first(one.row, one.above)) {
            waiting.push(Object.assign({}, one, { later: true }));
            continue;
        }
        seen.add(key);
        // Each row is asked whether it's the one as it's listed.
        for (const inside of ways.holds(one.row, one.above)) {
            const path = one.path.concat([ways.idOf(inside)]);
            if (path[path.length - 1] === id) {
                return said(path, false);
            }
            queue.push({ row: inside, path, above: one.row, depth: one.depth + 1,
                later: one.later });
            count += 1;
            if (ways.spent?.(count)) {
                return said(null, true);
            }
        }
    }
    return said(null, false);
}

// Where Ctrl+] goes for a row living at `path`, by steady ids, from where
// the launcher stands (`here`, the ids of the places held): into the
// places above it, onto it, as a spot is gone to; or null where it lives
// nowhere, or here, so Ctrl+] has nowhere to go.
function followed(path, here) {
    if (path === null) {
        return null;
    }
    const trail = path.slice(0, -1);
    if (trail.length === here.length && trail.every((step, at) => step === here[at])) {
        return null;
    }
    return { trail, spot: path[path.length - 1] };
}

// Whether a walk to a place got there: what's held (`held`, its steps'
// steady ids) ends on the place, all of `ids` deep. A place can refuse
// its step, as Passwords does while it's walked from outside, and a row
// can go between the ask and the walk, so a place asked open is said
// opened only where it is (Y2YBTS, 2026-11-13).
function reached(held, ids) {
    return held.length === ids.length && held.every((step, at) => step === ids[at]);
}

// One entry: its id, what it is, its words and where it sits as Joel reads
// it, what it says now, whether only Joel's Return does it, and the words
// it asks for, for one that takes them.
function entry(row, id, within) {
    const out = { id, kind: kindOf(row), words: row.name, in: within.join(" \u203a "),
        disruptive: disruptive(row) };
    if (typeof row.side === "string" && row.side !== "") {
        out.says = row.side;
    } else if (row.form && row.count) {
        // A command says what it does after its form rather than at its right.
        out.says = row.count;
    }
    if (row.given) {
        out.args = [{ name: "words", words: row.ask ?? "" }];
    }
    return out;
}
