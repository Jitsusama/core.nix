// The looks folder's files, as For You reads them (Now.qml, ForYouHolds.qml): what each look is,
// which wait on Joel and which are history, and how one is written again once it's done with.
// They're read apart from the launcher, so tests/for-you.qml and tests/look-cells.qml hold them
// in Qt's own engine. The
// to-do daemon reads the looks for the shell (KP0L6A's #4032), and serves each done with as
// History's row ready to draw (modules/to-do/src/history.rs), which these rules must agree with.
.pragma library
.import "ForYou.js" as ForYou
.import "LookForm.js" as LookForm
.import "Order.js" as Order

// What a look is about, and how it stands, as the harness writes them, and
// the one version of its file this reads.
const KINDS = ["pr", "page", "file", "diff", "terminal"];
const STATES = ["waiting", "opened", "cleared", "resolved", "expired"];
const VERSION = 1;
// What's read from a look's file only for the shell's own use, never
// written back.
const OWN = ["since", "resolved", "file", "words", "offers"];
// How many of a look's choices are offered, as the harness writes at most
// four (85PKB2, 2026-11-12).
const OFFERED = 4;
// How long an answer shows as given while the node hasn't said it took
// it, before saying it may not have gone: "no answer in 5 s" (docs/
// design.md). Its side says so from then until a word comes, the node
// going included, since nothing else times out a steer it was asked.
const ANSWER_HEARD = 5 * 1000;
// How long a look waits before it goes quiet: it leaves what waits, and so
// every count of it, for History, where it's still opened as it was. By then
// why it came is lost (Joel, 2026-11-11: "by the time i find them the context
// of why they exist is lost"), and VKCF3Z's flows say "a look older than
// three days goes quiet".
const QUIET_AFTER = 3 * 24 * 60 * 60 * 1000;

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
    // Never a "__proto__" key, which as an own key of the parse would
    // become the copy's prototype and so lend it a cell it doesn't name.
    const kept = {};
    for (const key of Object.keys(read).filter(name => name !== "__proto__")) {
        kept[key] = read[key];
    }
    return Object.assign(kept, {
        why: typeof read.why === "string" ? read.why : "",
        words: LookForm.wordsOf(read),
        since: ForYou.when(read.at),
        resolved: ForYou.when(read.resolvedAt),
        offers: offers(read.choices, read.wants)
    });
}

// A quest's id as its title ends with it, "(INV-31016)", which says
// nothing to Joel (attachments/what-needs-me-table.md, 2026-11-19).
const TITLE_ID = /\s*\([A-Z][A-Z0-9]*-\d+\)\s*$/u;

// Who a look comes from, in plain words: what its session says, else its
// quest's title without its id (`quest`, the index's entry, or null), else
// "". Never the quest's id, which tells Joel nothing.
function askerOf(one, quest) {
    const asker = written(one.asker);
    return asker !== "" ? asker : String(quest?.title ?? "").replace(TITLE_ID, "");
}

// What a look asks of Joel, and what happens if he does nothing, as its
// session writes them (85PKB2, 2026-11-19), or "". Read here rather than
// into the look, since a look is written back as it was read (`rewritten`).
function askOf(one) {
    return written(one.ask);
}

function ifNothingOf(one) {
    return written(one.ifNothing);
}

function written(value) {
    return typeof value === "string" ? value : "";
}

// A look's choices as it's offered them, each { value, label, follows }:
// one naming no value left out, one naming no label labelled by its value,
// and at most OFFERED. What follows a choice is its own, else what the look
// wants back says of its label (`wants`), else "".
function offers(choices, wants) {
    const named = value => typeof value === "string" && value !== "";
    const wanted = Array.isArray(wants?.choices) ? wants.choices : [];
    const follows = (one, label) => named(one.follows) ? one.follows
        : written(wanted.find(each => each?.label === label)?.follows);
    return (Array.isArray(choices) ? choices : [])
        .filter(one => one !== null && typeof one === "object" && named(one.value))
        .slice(0, OFFERED)
        .map(one => {
            const label = named(one.label) ? one.label : one.value;
            return { value: one.value, label, follows: follows(one, label) };
        });
}

// Joel's answer to a look, as the steer that tells it to the session that
// left it, `{ address, steer }`, at `at`: to that session where it is the
// look's cell (`cell`, cellFor's) and left it (leftIt) by its id, the
// look's address or as the cell the look names, by the address it goes by
// now, which Talk and Go To reach too, else to its quest's, whose address is the
// quest's own last part, so the look is answered where the asking goes on
// (85PKB2, 2026-11-12). A cell that is its leaver only by listing the
// look's address among its own, which any session may claim, takes it at
// the look's own address, as before cells, never at one of its choosing.
// Null where neither is known.
function answer(one, choice, cell, at) {
    const quest = String(one.quest ?? "").split("-").pop();
    const live = leftIt(one, cell);
    const upper = name => String(name ?? "").toUpperCase();
    const itself = live && (leftByItsId(one, cell)
        || [one.address, cellNamed(one)].some(name => upper(name) !== ""
            && upper(name) === upper(cell.address)));
    const address = upper((itself ? cell.address : live ? one.address : quest)
        || one.address);
    if (address === "") {
        return null;
    }
    return { address, steer: { act: "answer", look: one.id, value: choice.value,
        from: "joel", at: new Date(at).toISOString() } };
}

// The cell a look leads to, the session a look's Talk and Go To reach (Joel,
// 2026-11-16: "i have no way to go from the thing it shows to engaging with
// the thing working on it"): the one that left it, running or parked, else,
// once that has ended, the quest's own session, whose address is the
// quest's last part and where a look's answers go then (`answer`), else any
// other on its quest, a running one before a parked one. Null where none is
// held.
function cellOf(one, running, parked) {
    const held = (Array.isArray(running) ? running : [])
        .concat(Array.isArray(parked) ? parked : []);
    const left = held.find(session => ForYou.leftBy(one, session));
    const quest = typeof one.quest === "string" ? one.quest : "";
    if (left !== undefined || quest === "") {
        return left ?? null;
    }
    const own = quest.split("-").pop().toUpperCase();
    const on = held.filter(session => onQuest(session, quest));
    return on.find(session => String(session.address ?? "").toUpperCase() === own)
        ?? on[0] ?? null;
}

// The quest a session works on: the first quest it owns, else its
// questInfo's, else its `quest`, in the agents daemon's order
// (modules/agents/src/cells.rs `quest_of`). Only whether it is "" is read
// here yet, and onQuest decides which quests a session is on; the order is
// kept for Holders.js and Sessions.qml, which still read questInfo first
// and move onto this by Mon 23 Nov 2026.
// Read as cells.rs reads them, a questInfo id or quest that isn't a
// string is passed over, and a `quest` that isn't one is "".
function questOf(session) {
    return ownedBy(session)[0] ?? loadedBy(session);
}

// The quest a session says it has loaded: its questInfo's, else its older
// `quest`, else "".
function loadedBy(session) {
    return textOf(session.presence?.questInfo?.id) ?? textOf(session.presence?.quest) ?? "";
}

// The quests a session owns, in its own order.
function ownedBy(session) {
    return (Array.isArray(session.presence?.owns) ? session.presence.owns : [])
        .filter(one => typeof one === "string" && one.startsWith("QEST-"));
}

// `value` where it's a string, else undefined.
function textOf(value) {
    return typeof value === "string" ? value : undefined;
}

// Whether `session` works on `quest`: one it owns, or the one it has
// loaded (loadedBy). A look names the quest its session had loaded
// (the harness's looks.ts), which can be its questInfo's while it owns
// another, so a look's own leaver stays on it either way, as do the
// daemon's answer and the sessions on its quest. An older `quest` its
// questInfo has since replaced is not one it works on.
function onQuest(session, quest) {
    return quest !== "" && (ownedBy(session).includes(quest) || loadedBy(session) === quest);
}

// Whether opening served look `one` asks the to-do daemon to settle it:
// only once it `opened`, since a terminal whose session is gone opened
// nothing and stays as it was, and only one the shell holds no file of,
// still waiting or gone quiet, since the shell settles its own.
function servedSettles(one, opened) {
    return opened === true && one.file === ""
        && (one.state === "waiting" || one.state === "quiet");
}

// Whether `cell`, the session a look's Talk reaches, left look `one`: by
// the session or an address the look names (ForYou.leftBy), or as the very
// cell the look names (cellNamed), so Joel's answer and who asks go where
// Talk does.
function leftIt(one, cell) {
    const named = cellNamed(one).toUpperCase();
    return cell !== null && cell !== undefined && (ForYou.leftBy(one, cell)
        || (named !== "" && named === String(cell.address ?? "").toUpperCase()));
}

// Whether `session` is the one that left look `one`, by pi's own id for it,
// which no other session lists and which carries across a move.
function leftByItsId(one, session) {
    return typeof one.session === "string" && one.session !== ""
        && session.presence?.session === one.session;
}

// pi's id for a session, as the harness gives each one.
const SESSION_ID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/iu;
// How many of its id's hex digits the address a session goes by holds.
const ADDRESS_DIGITS = 8;

// The cell a look names: its own `cell`, else, for one left before looks
// named their cell, the address the session that left it went by, which
// the harness makes from pi's id for it as it makes a look's `cell` now
// (lib/mesh/address.ts's sessionAddress: its last eight hex digits, in
// capitals), so a look still waiting from then is led as a new one is;
// else "", for one naming no session of pi's.
function cellNamed(one) {
    if (typeof one.cell === "string" && one.cell !== "") {
        return one.cell;
    }
    const id = typeof one.session === "string" ? one.session : "";
    return SESSION_ID.test(id) ? id.split("-").join("").slice(-ADDRESS_DIGITS).toUpperCase() : "";
}

// The address the agents daemon says the cell a look names (cellNamed)
// goes to now, by its `agent-cell` records by key, `cells`
// (modules/agents/src/cells.rs): null where the record says it goes
// nowhere, and "" where it names no cell or the daemon holds no record.
function cellGoes(one, cells) {
    const cell = cellNamed(one);
    const record = cell === "" ? undefined : cells?.[`agent-cell:agents:${cell}`];
    if (record !== undefined && record !== null && record.goes === null) {
        return null;
    }
    return typeof record?.goes === "string" ? record.goes : "";
}

// The cell a look leads to: the cell itself while the lists hold it on the
// look's quest or on none yet, never one going by its address on another
// (cells.rs refuses a name contested across quests), else the live
// session the agents daemon names (cellGoes), found among `running` and
// `parked` by the address it goes to, where it is on the look's quest
// (onQuest). The daemon reads the cell's quest, not the look's, and once
// the node forgets the cell it names its one lister on any quest (cells.rs
// `claim`), so its answer alone never sends Talk to another quest. Where
// that is held nowhere, or the daemon says the cell goes nowhere or hasn't
// answered (away, or not yet served), cellOf's search decides among the
// sessions on the look's quest alone: the one that left it, by its id or
// any address it answers to, as cells.rs `claim` ranks a cell's old name
// on its quest, then the quest's own, so the daemon going away never moves
// Talk off the look's quest; the one that left it, by its own id, counts
// there on no quest yet too, as the cell itself does, since a session
// started again can name its quest a beat after it's held. A look on no
// quest reaches only the session that left it, by its own id, wherever it
// is, since sessions on no quest share nothing, and there a session that
// merely lists the cell's address is never found. The daemon and the
// lists each read the node and either can be a beat behind, so a cell
// back again wins over an older answer. A look left before looks named
// their cell is led by the one its session went by (cellNamed), so no
// look searches every quest's sessions.
function cellFor(one, cells, running, parked) {
    const cell = cellNamed(one);
    const goes = cellGoes(one, cells);
    const held = (Array.isArray(running) ? running : [])
        .concat(Array.isArray(parked) ? parked : []);
    const quest = typeof one.quest === "string" ? one.quest : "";
    const itself = cell === "" ? undefined : held.find(session => session.address === cell
        && (quest === "" || questOf(session) === "" || onQuest(session, quest)));
    if (itself !== undefined) {
        return itself;
    }
    if (quest === "") {
        return held.find(session => leftByItsId(one, session)) ?? null;
    }
    const on = held.filter(session => onQuest(session, quest)
        || (questOf(session) === "" && leftByItsId(one, session)));
    return on.find(session => goes !== null && goes !== "" && session.address === goes)
        ?? cellOf(one, on, []);
}

// The words a Go To whose session isn't held says, `{ gone, why }`, from
// its notice's `said` (words/notices.json's verb:notice:goTo): its `moved`
// pair where the session has `moved` on to another quest, else its own, and
// its own again where the moved pair says nothing, so the foot is never
// bare.
function goneWords(said, moved) {
    const pair = one => ({ gone: typeof one?.gone === "string" ? one.gone : "",
        why: typeof one?.why === "string" ? one.why : "" });
    const onward = pair(said?.moved);
    return moved === true && onward.gone !== "" ? onward : pair(said);
}

// Whether the session that left look `one` is still held, running or
// parked, by pi's id for it or the very address the look names, where
// cellFor leads nowhere, and on a quest other than the look's: it has
// moved on to another quest rather than ended, so Go says so. One that
// merely lists the look's address among its own is no sign the leaver
// lives, and one on no quest hasn't moved on to another.
function movedOn(one, running, parked) {
    const held = (Array.isArray(running) ? running : [])
        .concat(Array.isArray(parked) ? parked : []);
    const address = typeof one.address === "string" ? one.address : "";
    const quest = typeof one.quest === "string" ? one.quest : "";
    return held.some(session => (leftByItsId(one, session)
        || (address !== "" && session.address === address))
        && questOf(session) !== "" && (quest === "" || !onQuest(session, quest)));
}

// The label of the answer Joel gave a look still waiting, by `answered`'s
// { [look id]: { label, at, sent, taken, failed } }, for ANSWER_HEARD
// while the node may yet say it took it, or "".
function answering(one, answered, now) {
    const given = answered?.[one.id];
    return one.state === "waiting" && given && !given.failed && now - given.at < ANSWER_HEARD
        ? given.label : "";
}

// Why Joel's answer to a look still waiting may not have gone, "nobody"
// to tell, "unsent" with no node connected, "untaken" where the node
// couldn't give it, refused or its machine out of reach, "dropped" where
// it went before saying, or "unheard" past ANSWER_HEARD with no word, else
// "", said until he answers again or the look is settled, however much
// later he comes back to it: his choices coming back unsaid left him no
// way "to in any way follow through" (Joel, 2026-11-22).
function unanswered(one, answered, now) {
    const given = answered?.[one.id];
    if (one.state !== "waiting" || !given || given.taken) {
        return "";
    }
    if (typeof given.failed === "string") {
        return given.failed;
    }
    return now - given.at >= ANSWER_HEARD ? "unheard" : "";
}

// Whether Joel's answer to a look still waiting is out: given, with no
// word come of it, which only the node answering or going ends. Its
// choices stay away meanwhile, so a second answer can't follow one still
// on its way and the session hear both. Once the node went they come
// back, though it may have written the first, since a look left with no
// way to answer until the shell restarts is worse; the side's "yet" says
// it may still come.
function awaiting(one, answered) {
    const given = answered?.[one.id];
    return one.state === "waiting" && !!given && !given.taken && !given.failed;
}

// How long until an answer's side next changes as time passes, or null:
// its 5 s, while no word has come.
function answersChange(answered, now) {
    const ahead = Object.values(answered ?? {})
        .filter(given => given && !given.taken && !given.failed)
        .map(given => given.at + ANSWER_HEARD).filter(at => at > now);
    return ahead.length === 0 ? null : Math.min(...ahead) - now;
}

// The looks as `looks` splits them, with each waiting one whose answer the
// node took (`answered`'s { [look id]: { label, at, taken } }) in History,
// answered then, its side as `saying` words its choice: Joel's answer has
// handled it, so it leaves what waits at once (VKCF3Z, 2026-11-12), though
// its file waits until its session reads the steer and writes it resolved.
function taken(split, answered, saying) {
    const given = answered ?? {};
    const moved = split.waiting.filter(one => given[one.id]?.taken === true);
    if (moved.length === 0) {
        return split;
    }
    const done = moved.map(one => Object.assign({}, one, { state: "answered",
        how: saying(given[one.id].label), resolved: given[one.id].at }));
    return {
        waiting: split.waiting.filter(one => given[one.id]?.taken !== true),
        reading: split.reading ?? [],
        history: Order.sorted(done.concat(split.history),
            Order.stream(one => one.resolved ?? one.since))
    };
}

// A look waiting past its time, as History holds it: expired at its expiresAt, which the
// harness gives one asking nothing (85PKB2, 2026-11-12), or gone quiet QUIET_AFTER after it
// came, which History's side says. Its file still says it waits until a session of its quest
// writes it expired; nothing's written here, and the look as its file was read, kept to be
// read again, is left as it was.
function quieted(one, now) {
    if (one.state !== "waiting") {
        return one;
    }
    const expires = ForYou.when(one.expiresAt);
    if (expires !== null && now >= expires) {
        return Object.assign({}, one, { state: "expired", resolved: expires });
    }
    const gone = one.since !== null && now - one.since > QUIET_AFTER;
    return gone ? Object.assign({}, one, { state: "quiet", resolved: one.since + QUIET_AFTER })
        : one;
}

// Whether a look asks nothing of Joel, only to be seen, so it's to read rather than to look
// at: the harness says so as `asks: false`, and one saying nothing asks (85PKB2, 2026-11-12).
function asksNothing(one) {
    return one.asks === false;
}

// The looks folder's files, each { file, text }, as { waiting, reading, history }: those
// waiting that ask something, then those that ask nothing (To Read, VKCF3Z, 2026-11-12), each
// in the order they came, the oldest first, the rest newest first by when each was resolved,
// expired or went quiet, as at `now`. One still being written, `*.writing`, or unreadable is
// left out. A file whose text is as `kept.by` holds it isn't parsed again: an act changes one of
// Joel's 166, and parsing all took 9 ms (2026-11-10).
function looks(files, kept, now) {
    const was = kept.by ?? {};
    const read = {};
    const listed = (files ?? []).filter(one => !String(one.file ?? "").endsWith(".writing"))
        .map(one => {
            const file = String(one.file ?? "");
            const same = file in was && was[file].text === one.text;
            const each = same ? was[file].look : look(one.text);
            const made = each === null ? null : Object.assign(each, { file });
            read[file] = { text: one.text, look: made };
            return made === null ? null : quieted(made, now);
        }).filter(one => one !== null);
    kept.by = read;
    const waits = listed.filter(one => one.state === "waiting");
    return {
        waiting: Order.sorted(waits.filter(one => !asksNothing(one)),
            Order.smaller(one => one.since)),
        reading: Order.sorted(waits.filter(asksNothing), Order.smaller(one => one.since)),
        history: Order.sorted(listed.filter(one => one.state !== "waiting"),
            Order.stream(one => one.resolved ?? one.since))
    };
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

// Whether the to-do daemon's text of a look, `told`, says just what the
// text the shell showed it settled in, `shown`, did, but for when it was
// settled: the daemon writes the same look in its own key order at its own
// moment, so taking its text made the look's row again a moment after the
// act's own frame, and For You was drawn again for nothing.
function settledAlike(shown, told) {
    const read = text => {
        try {
            const one = JSON.parse(text);
            return one !== null && typeof one === "object" ? one : null;
        } catch (_unread) {
            return null;
        }
    };
    const was = read(shown);
    const is = read(told);
    if (was === null || is === null) {
        return false;
    }
    const unsettled = one => canonical(Object.assign({}, one, { resolvedAt: null }));
    return unsettled(was) === unsettled(is);
}

// A value as JSON with every object's keys in order, at any depth, since
// the daemon may order them otherwise than the look's session did.
function canonical(value) {
    if (Array.isArray(value)) {
        return `[${value.map(canonical).join(",")}]`;
    }
    if (value !== null && typeof value === "object") {
        return `{${Object.keys(value).sort()
            .map(key => `${JSON.stringify(key)}:${canonical(value[key])}`).join(",")}}`;
    }
    return JSON.stringify(value);
}
