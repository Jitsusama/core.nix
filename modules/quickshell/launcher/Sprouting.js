.pragma library

// Start New Work, Agents' act that begins new work from what Joel says: a
// link alone, or words in one of the repositories his open quests work in
// (Joel, 16 November: starting new work was too hard to do at all). It
// asks the agents daemon, which asks the realm's node for a `Sprout`, the
// request `mycelium sprout` makes, so the launcher and the command line
// never choose differently: a quest of its own, seeded from what he said,
// its session held. He asked for it, so its window opens for him as soon
// as it's up (Joel, 18 November: "start new work does not seem to work
// from the launcher", when it started in no window and said so only on
// the row he'd left); agents' own windows still go behind his.
// The daemon asks only for the shell (modules/agents/src/asker.rs), since
// the node takes it for Joel. The repositories come from the quests
// daemon (modules/quests/src/folders.rs).
//
// Only a sprout is asked, and only its own shape: what he said as one,
// and a folder only when it's a whole path.

// Whether a sprout pressed now is Joel's own Return in `launcher`, never
// a walk from outside (`qs ipc call launcher runWith`, `launcher do`),
// which any program of his can ask, an agent's among them. The agents
// daemon can't tell those apart, since the shell asks for both, so this
// is their only guard and fails closed: no launcher, or one that doesn't
// say it isn't walked, is no Return of his (Y2YBTS and ISXEG8 on #5456).
function fromJoel(launcher) {
    return launcher !== null && launcher !== undefined && launcher.cataloguing === false;
}

// The ask that sprouts `text`, trimmed, in `folder` when one is given, or
// null when nothing is said or the folder isn't a whole path (refused
// says which).
function ask(text, folder) {
    if (refused(text, folder, null) !== null) {
        return null;
    }
    const where = String(folder ?? "");
    return Object.assign({ ask: "sprout", text: String(text).trim() },
        where !== "" ? { folder: where } : {});
}

// A token's digits, letters and digits alike, so it stays short.
const TOKEN_BASE = 36;

// How many tokens one moment can tell apart.
const TOKEN_CHANCES = 2 ** 32;

// A token naming a start to the agents daemon, which keeps it by that
// token until told (starts.rs, letters, digits and dashes): the moment
// `now` and a `chance` in [0, 1), so two asked in one moment differ.
function token(now, chance) {
    return `${Math.floor(now).toString(TOKEN_BASE)}-${Math.floor(chance * TOKEN_CHANCES)
        .toString(TOKEN_BASE)}`;
}

// How long a start this shell never asked stands kept before it's told.
// While a switch hands the shell over, the one that asked is still up
// beside the new one, and says told as it hears the end, a round trip;
// this is room for a busy shell to, so Joel isn't told it twice.
const SETTLE_MS = 5000;

// How long a shell waits on the daemon's answer to a start before taking
// it as none: the daemon says the node's silence within 50 s (sprout.rs's
// PATIENCE), so a minute with no answer at all, as from a daemon that
// went as it asked, is no answer.
const PATIENCE_MS = 60000;

// How long from its asking a start's own shell, if it's still up, has
// surely told it: its patience, a shut launcher's watch's pace (10 s,
// Agents' shutPace) and the moment. Each shell comes back to a daemon
// started again in its own time, so another may hear the record well
// before the asker does, and waits this out before telling it.
const ASKER_ANSWERS_MS = PATIENCE_MS + 10000 + SETTLE_MS;

// How long a told the daemon didn't take waits to be said again: a shell
// held while a switch hands over is taken for the shell only once it's
// the unit's main process, up to 70 s on (shell-hands-over.sh), and one
// dropped with the daemon is said again as its watch answers anyway.
const TOLD_AGAIN_MS = 30000;

// Of the starts the agents daemon keeps ended and untold (`kept`, each
// `{ token, ended }`), by how this shell stands on each (`known`, a
// token's "asking", "ended" or "told") and when it first saw each it
// never heard the end of (`seen`, by token): those to tell, as a shell
// started since or a start a restart cut off has it, once seen
// SETTLE_MS before `now` and asked (`at`) ASKER_ANSWERS_MS before it;
// those still `waiting` that out; those
// `answered`, still asked here, whose answer the daemon dropped as it
// went, so its record is the answer; and every token to say told, all
// but those still asked or waited on.
function untold(kept, known, seen, now) {
    // Own keys alone, so a prototype's name is never known; Qt's engine has no Object.hasOwn.
    const own = (of, named) => Object.keys(of).includes(named);
    const askerHad = one => !Number.isFinite(one.at) || now - one.at >= ASKER_ANSWERS_MS;
    const waitedOut = one => own(seen, one.token) && now - seen[one.token] >= SETTLE_MS
        && askerHad(one);
    const unheard = kept.filter(one => !own(known, one.token));
    const tell = unheard.filter(waitedOut);
    const waiting = unheard.filter(one => !waitedOut(one)).map(one => one.token);
    const said = kept.filter(one => (own(known, one.token) ? known[one.token] !== "asking"
        : waitedOut(one))).map(one => one.token);
    const answered = kept.filter(one => own(known, one.token) && known[one.token] === "asking");
    return { tell, told: said, waiting, answered };
}

// Of the starts to `tell`, those the daemon still keeps by its answer to
// a get of their keys (`records`, or null when it didn't answer): its
// asker, up beside this shell while a switch hands over, may have told
// one since this shell's copy was served, and a get names only what's
// still held, so none is told twice. With no answer, none is told yet.
function stillKept(tell, records) {
    if (!Array.isArray(records))
        return [];
    const keys = records.map(record => record?.key);
    return tell.filter(one => keys.includes(keptKey(one.token)));
}

// The key the agents daemon serves a kept start under (starts.rs).
function keptKey(token) {
    return `start:agents:${token}`;
}

// Why `ask` would ask nothing, in `said`'s words (`empty` or
// `unplaced`), or null when it asks; with `said` null, the key alone.
function refused(text, folder, said) {
    const where = String(folder ?? "");
    let why = null;
    if (String(text ?? "").trim() === "") {
        why = "empty";
    } else if (where !== "" && !where.startsWith("/")) {
        why = "unplaced";
    }
    if (why === null || said === null) {
        return why;
    }
    return String(said[why] ?? "");
}

// How it ended, from the daemon's `answer` (modules/agents/src/sprout.rs):
// `{ started: true, machine, held }` once the node started it, `held` the
// id its holder keeps it by ("" where the node gave none), else
// `{ started: false, why }` in the node's or the daemon's words, or
// `said`'s where no daemon answered (null) or its answer can't be read.
// A node the daemon heard fall silent (code `unanswered`) is unanswered,
// in the daemon's words, since it may yet start.
function ended(answer, said) {
    if (answer === null || answer === undefined) {
        return { started: false, unanswered: true, why: String(said.unanswered ?? "") };
    }
    const begun = answer.sprouted;
    if (typeof begun?.machine === "string" && begun.machine !== "") {
        return { started: true, machine: begun.machine,
            held: typeof begun.held === "string" ? begun.held : "" };
    }
    const why = answer.error?.message;
    if (typeof why === "string" && why !== "") {
        return answer.error.code === "unanswered" ? { started: false, unanswered: true, why }
            : { started: false, why };
    }
    return { started: false, why: String(said.unread ?? "") };
}

// The session among `running` (Sessions.running) that the sprout held by
// `held` on `machine` started, or null before it's up: here by its holder's
// id, and elsewhere, which keeps no holder's id here, only on the machine the
// node named, by the words that attach it, whose last names that id. A
// session here whose attach words name it is another's, never this one.
function sproutedOf(running, held, machine) {
    if (held === "") {
        return null;
    }
    const attaches = one => Array.isArray(one.attach) && one.attach[one.attach.length - 1] === held;
    return (
        running.find(one =>
            one.here === false ? one.machine === machine && attaches(one) : one.held === held
        ) ?? null
    );
}

// Whether Joel still waits on the act `key` in `launcher`: open, on its row
// or held in its ask, where Return leaves him with the field emptied, and no
// letter typed since the answer, when its `lettersAt` was `letters`. Only
// then may its window take the keys; otherwise what he types next, a new try
// in the ask included, could go into an agent's prompt.
function waitsOn(launcher, key, letters) {
    const on = row => row?.key === key;
    return (
        launcher?.open === true &&
        launcher.lettersAt === letters &&
        (on(launcher.asking) || on(launcher.selected))
    );
}

// How `openings` ({key, held, machine, since}) stand at `now`, `found`
// giving each one's session or null: `late` have waited `patience`
// milliseconds or more, and open nothing even if up by now, so a window
// never comes past the minute it's said to; `up` came up within it, each
// with its `session`; `waiting` are neither yet.
function settled(openings, found, now, patience) {
    const late = [];
    const up = [];
    const waiting = [];
    for (const one of openings) {
        const session = found(one);
        if (now - one.since >= patience) {
            late.push(one);
        } else if (session) {
            up.push(Object.assign({}, one, { session }));
        } else {
            waiting.push(one);
        }
    }
    return { late, up, waiting };
}

// What the act that ran says of how it ended, in `said`'s words: one the node
// started with no holder's id can't be found to open, so it promises none.
function told(how, said) {
    if (!how.started) {
        return how.why;
    }
    const words = how.held === "" ? said.unheld : said.started;
    return String(words ?? "").replace("{machine}", how.machine);
}

// Where how a start ended is said beyond its own row, which Joel may have
// left: a start refused at 17:11 on 18 November was said only there, in a
// launcher he'd shut five seconds after his Return, and so it vanished
// ("That can never happen, things I send in must be seen through"). Every
// end is journalled, without his words, so the next one missed can be
// found; one unanswered may yet start, so it isn't journalled as not.
// One that opens no window is `told` Joel in `said`'s words (Agents'
// sproutTell says where); one held to open says itself as its session
// comes up, or is told then (Agents' openSprouted).
function reach(how, said) {
    const tell = how.started !== true || how.held === "";
    return { journal: journalled(how), told: tell ? told(how, said) : null };
}

// The journal's line for how a start ended, in the machine's words or
// the why, never Joel's.
function journalled(how) {
    if (how.started) {
        return `new work started on ${how.machine}`;
    }
    if (how.unanswered) {
        return "new work's start went unanswered; it may still start";
    }
    return `new work wasn't started: ${how.why}`;
}

// How long a tell stands in the foot before a close counts as having read
// it: the keys that shut the launcher (Escape, then :q or ZZ) are a
// leaving, not a reading, so a close soon after the tell notices it.
const READ_MS = 10_000;

// Whether what the foot told, `footed` ({ keys, at }, the keys taken and
// the time as it was told), is noticed as the launcher shuts with
// `keysTaken` and `now`: when no key came since, or it shut within
// READ_MS of the tell. Told twice, a notice beside a read foot, costs less
// than an end he never sees.
function noticedAtShut(footed, keysTaken, now) {
    if (footed === null || footed === undefined) {
        return false;
    }
    return keysTaken === footed.keys || now - footed.at < READ_MS;
}

// A notice's body for the ends told, `ends`, one to a line and as plain
// text, since a notice's body is markup and the node writes the why: one
// notice, so the notices daemon folds none of them away unseen.
function noticeBody(ends) {
    return ends.map(text => String(text).replace(/&/gu, "&amp;").replace(/</gu, "&lt;")
        .replace(/>/gu, "&gt;")).join("\n");
}
