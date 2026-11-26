.pragma library

// What Start New Work says of a start waiting for its session to come up
// (Agents.qml's openings), apart from Sprouting.js, which holds the start
// itself. A start on a loaded machine can take minutes to come up (Joel's
// at 14:30 on 25 November took five and a half), so its late words are said
// long after his Return, when the act may show another start, or the foot
// another start's end.

// The names a word of Start New Work's may fill.
const FILLED = /\{(machine|title|minutes)\}/gu;

// `word` of Start New Work's words, `said`, with each name FILLED takes
// filled from `fills` by a function, so a title's `$&` is only text, and a
// name `fills` doesn't give filled with nothing.
function filled(said, word, fills) {
    return String(said?.[word] ?? "").replace(FILLED, (_, name) => String(fills[name] ?? ""));
}

// `pressed` (by each act's key, the last time it was pressed or given
// fresh words) with act `key` stamped at `now`.
function pressedAt(pressed, key, now) {
    return Object.assign({}, pressed, { [key]: now });
}

// Whether a start whose Return was at `asked` (askedOf) may still say over
// its act `key`: not once that act was pressed again, or given fresh
// words, after it (`pressed`, as pressedAt keeps it).
function saysOnAct(pressed, key, asked) {
    return (pressed[key] ?? 0) <= asked;
}

// The most one pass of settled counts, above its timer's 5 s, so a wall
// clock set right, or a suspend, isn't said a wait of a quarter hour.
const PASS_COUNTS_MS = 30000;

// How `openings` ({key, held, machine, since, late, waited, at}) stand at
// `now`, by `found` giving each one's session or null: `up` are up, however
// late; `late` waited `patience` unmarked, and wait on in `waiting` marked
// `late`, so each is said late once; `gone` waited `watch`; `waiting` wait
// on. Each waited what it had `at` its last pass and the time since, to
// PASS_COUNTS_MS, or uncounted, the time since its `since`.
function settled(openings, found, now, { patience, watch }) {
    const stood = { late: [], up: [], gone: [], waiting: [] };
    for (const one of openings) {
        const session = found(one);
        const waited = Number.isFinite(one.at)
            ? (one.waited ?? 0) + Math.min(Math.max(now - one.at, 0), PASS_COUNTS_MS)
            : now - one.since;
        const counted = Object.assign({}, one, { waited, at: now });
        if (session) {
            stood.up.push(Object.assign(counted, { session }));
        } else if (waited >= watch) {
            stood.gone.push(counted);
        } else if (waited >= patience && one.late !== true) {
            stood.late.push(counted);
            stood.waiting.push(Object.assign({}, counted, { late: true }));
        } else {
            stood.waiting.push(counted);
        }
    }
    return stood;
}

// The openings still `waiting` (settled's), each of the `late`
// ones carrying as `told` the words `texts` said of it, in late's order, so
// an opening takes back from the foot only what was said of itself.
function toldOf(waiting, late, texts) {
    return waiting.map(one => {
        const at = late.findIndex(was => was.held === one.held && was.machine === one.machine);
        return at < 0 ? one : Object.assign({}, one, { told: texts[at] });
    });
}

// What the foot told (`footed`, Agents.qml's sproutFooted, `{ told, ... }`)
// without the first tell reading `text`, as a start's window opening
// answers what was said of it, while each other start's tell stays to be
// noticed as the launcher shuts; none when nothing is left.
function footedWithout(footed, text) {
    if (footed === null || footed === undefined) {
        return null;
    }
    const at = footed.told.indexOf(text);
    if (at < 0) {
        return footed;
    }
    const told = footed.told.filter((_, index) => index !== at);
    return told.length === 0 ? null : Object.assign({}, footed, { told });
}

// When `one`'s Return was, for what's pressed on its act after it: its
// `asked`, or for one with none (a start the node kept, heard later) its
// `since`.
function askedOf(one) {
    return one.asked ?? one.since;
}

// Whether `one`, up, opens and takes the keys: only where Joel `waits` on
// its act (Sprouting.waitsOn) and its act wasn't pressed or given fresh
// words since its Return (`pressed`, saysOnAct). A link offer comes only
// with the launcher shut, which spends every opening's keys (shutOn), so
// this holds against a press waitsOn can't see.
function takesKeys(waits, pressed, one) {
    return waits && saysOnAct(pressed, one.key, askedOf(one));
}

// The opening a start `how` on the act `key` waits on, answered `now`,
// from `given`, what its Return recorded (`at` its stamp, `keys` and
// `letters` the launcher's keysTaken and lettersAt, `token` its start's):
// what's pressed or typed counts from the Return, and the minute's
// patience from `now`, so a slow answer isn't said to be still coming up
// seconds after it, counted pass by pass (settled). Its token
// keeps the daemon's record of it untold while it's watched, so a shell
// handed over to watches for it again (Agents.qml's sproutWatched). Its
// `words` and `on`, where it was asked to start, say whether a Return
// later is the same start given again (StartSaid.underWayAgain).
function opening(how, key, given, now) {
    return { key, held: how.held, machine: how.machine, since: now, asked: given.at,
        keys: given.keys, letters: given.letters, words: String(given.words ?? ""),
        on: String(given.on ?? ""), token: given.token ?? "", waited: 0, at: now };
}

// `given` (an opening, or Agents.qml's sproutGiven) once the launcher
// shuts: it takes the keys no more, as a reopening needn't move a key and
// Joel has left the act's wait.
function spent(given) {
    return Object.assign({}, given, { keys: -1 });
}

// The `openings` once the launcher shuts, each spent.
function shutOn(openings) {
    return openings.map(spent);
}

// How a start `how` is said at its answer, `given` its Return's record: one
// held to open but answered after the launcher shut (spent) opens no window,
// so it's said as one that shows in Agents once it's up (Sprouting.told's
// unheld), not promised a window that won't come.
function promised(how, given) {
    return how.started === true && how.held !== "" && given.keys === -1
        ? Object.assign({}, how, { held: "" }) : how;
}

// The `openings` whose acts promised a window (not spent, nor said yet to be
// still coming up), each `{ one, how }`, how its act is said once the
// launcher shuts, spending their keys (shutOn): as one that shows in Agents
// once it's up (promised).
function unpromised(openings) {
    return openings.filter(one => one.keys !== -1 && one.late !== true).map(one => ({ one,
        how: promised({ started: true, held: one.held, machine: one.machine }, { keys: -1 }) }));
}

// The opening watching a start `how` no act waits on (one kept, or answered
// late), from `act`, `{ key, asked, token }` (the act it was asked from and
// its Return, or for one kept a key of its own and its asking, its start's
// token, and `late` for one said late already), answered `now`: keys -1,
// so it never takes the keys, and Joel is told when it's up or lost. One
// kept has waited since its asking, so a shell handed over to watches out
// its quarter hour, not a fresh one. Null for one refused or with no hold
// to find, which Sprouting.reach tells, and for one `openings` already
// watch.
function watching(openings, how, act, now) {
    if (how.started !== true || how.held === ""
        || openings.some(one => one.held === how.held && one.machine === how.machine)) {
        return null;
    }
    const waited = act.kept === true && Number.isFinite(act.asked)
        ? Math.max(now - act.asked, 0) : 0;
    const one = { key: act.key, held: how.held, machine: how.machine, since: now,
        asked: act.asked, keys: -1, token: act.token ?? "", waited, at: now };
    return act.late === true ? Object.assign(one, { late: true }) : one;
}

// The tokens of the starts a pass settled (settled's `stood`),
// up or watched no more, whose records the daemon keeps until told.
function settledTokens(stood) {
    return stood.up.concat(stood.gone).map(one => one.token ?? "")
        .filter(token => token !== "");
}

// What a pass tells of starts (`told`, each `{ one, text }`, in order), by
// where Joel sees it: in the foot, `foot`, those he still `waits` on
// (Sprouting.waitsOn), whose foot he's watching; the rest as a notice,
// `notice`, since a late word in the foot would go at his next key, or put
// aside a link offer's foot telling where its words came from.
function toldWhere(told, waits) {
    const where = { foot: [], notice: [] };
    for (const each of told) {
        where[waits(each.one) ? "foot" : "notice"].push(each.text);
    }
    return where;
}

// The word said of `one` come up and left for Go To: "upLater", naming
// it, where it was said late to be still coming up, so Joel knows which
// start it was; else "up".
function upWord(one) {
    return one.late === true ? "upLater" : "up";
}

// What the foot told once opening `one` takes back the words said of it
// late (its `told`, toldOf's), as its window opens or its up word is
// told: `footed` what's left, and `cleared` the foot it was shown in where
// nothing is left, since sproutShut finds no foot then to clear; else null.
// One never said late has no `told`, which no tell reads, so it takes back
// nothing.
function takenBack(footed, one) {
    const left = footedWithout(footed, one.told);
    const was = footed ?? null;
    return { footed: left, cleared: was !== null && left === null ? was.shown : null };
}

// What the foot told once a pass (settled's `stood`) takes back what was
// said late of each up and gone, all before the pass tells a word of its
// own (Agents.qml's openSprouted), so the foot never shows a "still coming
// up" beside its "up" or "lost": `footed` what's left, and `cleared` each
// foot left with nothing.
function passTakenBack(footed, stood) {
    let left = footed ?? null;
    const cleared = [];
    stood.up.concat(stood.gone).forEach(one => {
        const taken = takenBack(left, one);
        left = taken.footed;
        if (taken.cleared !== null)
            cleared.push(taken.cleared);
    });
    return { footed: left, cleared };
}

// How a start's answer is taken, by whether one came (`answered`), how
// this shell stood on its token (`state`, Agents.qml's sproutTokens'),
// whether the patience ended it unanswered (`late`), and whether Joel's
// act still waits on it (`current`). With none, as the daemon went as it
// asked, it "awaits" the record the daemon kept before it asked the node,
// which answers it (sproutsKept), so Joel is never told both "unanswered"
// and its end; else it's "sprouted" on the act, ends "late" (lateTold),
// or was "recorded", told already by its record.
function heardAs(answered, { state, late, current }) {
    if (!answered && (state === "asking" || late))
        return "awaits";
    if (current)
        return "sprouted";
    return late ? "late" : "recorded";
}

// How a token stood at `state` stands once its answer is taken as `taken`
// (heardAs's): ended, but for one its record ended first, which stands as
// that left it, so one still watched stays kept untold (sproutTold).
function heardState(taken, state) {
    return taken === "recorded" ? state : "ended";
}

// What a start the patience ended says once it ends after all (`how`):
// on its act where it may (`onAct`, saysOnAct's), as a notice (`reached`,
// Sprouting.reach's told), and whether it's watched for. Unanswered
// again, nothing, since the patience said so and Joel would be told
// nothing answered twice.
function lateTold(how, onAct, reached) {
    if (how.unanswered === true)
        return { onAct: false, notice: null, watch: false };
    return { onAct, notice: reached, watch: how.started === true };
}
