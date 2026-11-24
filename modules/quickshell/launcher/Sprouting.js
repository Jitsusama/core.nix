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

// The most characters new work a link offers runs to, as mycelium-link's
// MOST_WORDS: a few sentences, so a page can't fill the field with a book.
const OFFERED_MOST = 1000;

// The control characters, which a line of words never needs: C0, then
// DEL and C1, by their codes.
const LAST_C0 = 0x1f;
const FIRST_C1 = 0x7f;
const LAST_C1 = 0x9f;

// `word` with every control character left out.
function uncontrolled(word) {
    return Array.prototype.filter.call(word, unit => {
        const code = unit.charCodeAt(0);
        return code > LAST_C0 && (code < FIRST_C1 || code > LAST_C1);
    }).join("");
}

// Unicode's format characters (general category Cf), as mycelium-link's
// FORMATTING has them, first and last of each run: they draw nothing yet
// can turn text around (U+202E), hide in it (U+200B) or join it.
const FORMATTING = [
    [0x00ad, 0x00ad], [0x0600, 0x0605], [0x061c, 0x061c], [0x06dd, 0x06dd],
    [0x070f, 0x070f], [0x0890, 0x0891], [0x08e2, 0x08e2], [0x180e, 0x180e],
    [0x200b, 0x200f], [0x202a, 0x202e], [0x2060, 0x2064], [0x2066, 0x206f],
    [0xfeff, 0xfeff], [0xfff9, 0xfffb], [0x110bd, 0x110bd], [0x110cd, 0x110cd],
    [0x13430, 0x1343f], [0x1bca0, 0x1bca3], [0x1d173, 0x1d17a],
    [0xe0001, 0xe0001], [0xe0020, 0xe007f]
];

// What draws nothing or a blank yet isn't Cf, as mycelium-link's BLANK has
// it: Unicode's other default ignorables (Hangul fillers, variation
// selectors and the rest) and the braille blank, U+2800, which the shell
// draws full width. Kept, a run of them pushes a page's words out of sight
// between the start Joel reads and the end Return waits for.
const BLANK = [
    [0x034f, 0x034f], [0x115f, 0x1160], [0x17b4, 0x17b5], [0x180b, 0x180d],
    [0x180f, 0x180f], [0x2065, 0x2065], [0x2800, 0x2800], [0x3164, 0x3164],
    [0xfe00, 0xfe0f], [0xffa0, 0xffa0], [0xfff0, 0xfff8], [0xe0000, 0xe0000],
    [0xe0002, 0xe001f], [0xe0080, 0xe0fff]
];

// Whether `code` is a format character or draws a blank. A run is read by index, since
// Qt's engine makes an iterator and its results for each pair taken apart (Wheres.js).
function unseen(code) {
    return [FORMATTING, BLANK].some(runs => runs.some(run => code >= run[0] && code <= run[1]));
}

// The first code point past the 16 bits one UTF-16 half holds.
const FIRST_PAIRED = 0x10000;

// `text` with every format or blank character left out, read by code point, so a
// tag character (a pair of halves) goes whole: Qt's engine walks a string
// by its halves, even in Array.from.
function unformatted(text) {
    let kept = "";
    for (let at = 0; at < text.length;) {
        const code = text.codePointAt(at);
        const width = code >= FIRST_PAIRED ? 2 : 1;
        if (!unseen(code)) {
            kept += text.slice(at, at + width);
        }
        at += width;
    }
    return kept;
}

// The next line character, which Rust's is_whitespace counts a space and
// JavaScript's \s doesn't, so both readers fold it the same.
const NEXT_LINE = /\u0085/g;

// The spaces between a line's words.
const SPACES = /\s+/;

// A character written as a pair of UTF-16 halves, as an emoji is.
const PAIRED = /[\uD800-\uDBFF][\uDC00-\uDFFF]/g;

// The shape of a mesh address, as mycelium-link's `addressed` reads one.
const ADDRESS = /^[A-Z0-9]{1,12}$/;

// New work a mycelium: link offers (mycelium-link's sprout), as `qs ipc
// call launcher newWork` hands it: its words after "words:" and the cell
// it came from after "from:". Any program of Joel's can make that call,
// so it's read again here: { words, from }, the words one plain line of
// at most OFFERED_MOST characters, and from { address, name } only where
// `nameOf` names a session here by that address, else null; or null
// where no plain words are offered. It only fills the field: starting
// still waits on his own Return (fromJoel).
function offered(words, from, nameOf) {
    const said = String(words ?? "");
    if (!said.startsWith("words:")) {
        return null;
    }
    // Format characters go first: \s counts U+FEFF a space, where
    // mycelium-link drops it from inside a word.
    const plain = unformatted(said.slice("words:".length).replace(NEXT_LINE, " "))
        .split(SPACES)
        .map(uncontrolled)
        .filter(word => word !== "")
        .join(" ");
    // Counted by characters, as mycelium-link's chars() counts them: QML's
    // engine counts a string's halves, so a pair of them is one here.
    if (plain === "" || plain.replace(PAIRED, "_").length > OFFERED_MOST) {
        return null;
    }
    const cell = String(from ?? "");
    const address = cell.startsWith("from:") ? cell.slice("from:".length) : "";
    const name = ADDRESS.test(address) && typeof nameOf === "function"
        ? nameOf(address) : null;
    return {
        words: plain,
        from: typeof name === "string" && name !== "" ? { address, name } : null
    };
}

// What the foot says over new work a link offered, from the words
// `said` (agents.json's Start New Work's): that a page offered it, and
// which session it says it's from where it named one here, by its
// quest's name, which any page may name, so it's never said as sure.
// The name goes in as it is, a `$` in it included.
function offeredSaid(offer, said) {
    const words = said ?? {};
    return offer.from === null ? String(words.offered ?? "")
        : String(words.offeredFrom ?? "").replace("{cell}", () => offer.from.name);
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
    const notHeard = kept.filter(one => !own(known, one.token));
    const tell = notHeard.filter(waitedOut);
    const waiting = notHeard.filter(one => !waitedOut(one)).map(one => one.token);
    const said = kept.filter(one => (own(known, one.token) ? known[one.token] !== "asking"
        : waitedOut(one))).map(one => one.token);
    const answered = kept.filter(one => own(known, one.token) && known[one.token] === "asking");
    return { tell, told: said, waiting, answered };
}

// Of the starts to `tell`, those the daemon still keeps by its `answer`
// to sprout-kept, which names the tokens it keeps of those asked: its
// asker, up beside this shell while a switch hands over, may have told
// one since this shell's copy was served, so none is told twice. The
// daemon answers from the file it keeps, read before it listens, so a
// daemon just started names those it kept though it serves no record
// yet. With no answer, or a refusal, none is told yet.
function stillKept(tell, answer) {
    const kept = answer?.kept;
    if (!Array.isArray(kept)) {
        return [];
    }
    return tell.filter(one => kept.includes(one.token));
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
            : { started: false, why, refusal: String(answer.error.refusal ?? "") };
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
// or held in its ask, where Return leaves him with the field emptied, with
// no key taken and no letter given since his Return, when its `keysTaken`
// was `keys` and its `lettersAt` `letters`: an arrow moves only the keys, a
// paste only the letters. Else what he types could reach an agent's prompt.
function waitsOn(launcher, key, keys, letters) {
    const on = row => row?.key === key;
    return launcher?.open === true && launcher.keysTaken === keys &&
        launcher.lettersAt === letters && (on(launcher.asking) || on(launcher.selected));
}

// How `openings` ({key, held, machine, since, late}) stand at `now`, by
// `found` giving each one's session or null: `up` are up, however late;
// `late` waited `patience` unmarked, and wait on in `waiting` marked `late`,
// so each is said late once; `gone` waited `watch`; `waiting` wait on.
function settled(openings, found, now, { patience, watch }) {
    const stood = { late: [], up: [], gone: [], waiting: [] };
    for (const one of openings) {
        const session = found(one);
        const waited = now - one.since;
        if (session) {
            stood.up.push(Object.assign({}, one, { session }));
        } else if (waited >= watch) {
            stood.gone.push(one);
        } else if (waited >= patience && one.late !== true) {
            stood.late.push(one);
            stood.waiting.push(Object.assign({}, one, { late: true }));
        } else {
            stood.waiting.push(one);
        }
    }
    return stood;
}

// What opens a placement refusal, and a machine's reason in one that waiting
// never changes, as mayYetTake reads them.
const PLACED_NOWHERE = /^nowhere: /u;
const NEVER_TAKES = /^\S+ (?:lacks|isn't) |, (?:lacks|isn't) /u;

// The refusals Joel can meet that one act of the launcher's gets past, by
// the node's or the agents daemon's words, each with the key of that act's
// words in `said` and, for one naming the command line's act, what to cut
// from its end so only the launcher's is said. As mycelium 8f1b0e66 says
// them, in its sprout.rs's folder and its node's placement: one needing a
// repository, begun at home, or naming a folder not here is started In one
// of his repositories, or from the work's whole link, which names its own;
// one placed nowhere, whose words say each machine's reasons (needs it
// lacks, reach, room, its time), is said again once one of those can take
// it, unless each lacks what the work needs or isn't the machine it names,
// which waiting never changes (mayYetTake); one placed nowhere now, as the
// node's nowhere_now says one a machine out of reach would take, is said
// again so too. One whose node's socket is
// missing or refuses, as the agents daemon's sprout.rs says from Rust's own
// words for the two, is said again once the node runs; any other failure to
// reach it isn't a node that's down. A refusal none of these knows is told in its own words alone.
const ACTS = [
    { says: /^it needs a repository to work in, /u,
        cut: /: say which, as `[^`]*`(?:; `mycelium places` lists the places)?$/u,
        act: "actRepository" },
    { says: /^a session begun in a home folder /u, cut: /: name the repository's folder$/u,
        act: "actRepository" },
    { says: / isn't a folder here$/u, cut: null, act: "actRepository" },
    { says: /^nowhere: /u, cut: null, act: "actPlaced", only: mayYetTake },
    { says: /^nowhere now, though a machine out of reach would do: /u, cut: null,
        act: "actPlaced" },
    { says: /^the node couldn't be reached: (?:No such file or directory|Connection refused) /u,
        cut: null, act: "actNode" }
];

// Whether a placement refusal's `words` name a machine that may yet take
// the work: one whose reasons, as mycelium's placement says them
// ("skiff lacks system macos; tern is out of reach"), include none that
// waiting never changes, a need it lacks or a name it isn't. A realm that
// knows no machine has none.
function mayYetTake(words) {
    return words.replace(PLACED_NOWHERE, "").split("; ")
        .some(miss => miss !== "the realm knows no machine" && !NEVER_TAKES.test(miss));
}

// The keys of the words each known refusal's act is said in (ACTS), so a
// test can hold them to the words Joel reads.
function refusalActs() {
    return ACTS.map(one => one.act);
}

// The refusal `why` as Joel is told it, in `said`'s words: ending with the
// act that gets past it (ACTS), or as it is when none is known, its words
// are missing, or the command line's act to cut isn't where it's cut from,
// since saying both acts is worse than saying the node's alone. The
// journal keeps `why` as it was said.
function acted(why, said) {
    const words = String(why ?? "");
    const known = ACTS.find(one => one.says.test(words));
    const act = String(said?.[known?.act] ?? "");
    if (known === undefined || act === "" || (known.cut !== null && !known.cut.test(words))
        || (known.only !== undefined && !known.only(words))) {
        return words;
    }
    const cut = known.cut === null ? words : words.replace(known.cut, "");
    return `${cut}; ${act}`;
}

// Whether a start that ended `how` asks Joel where it starts: one refused
// for want of a repository, which a folder Start New Work lists gets past,
// so he's stepped back to them, his words carried to the one he picks
// (Joel, 24 November, of a link refused so: "the UI was crap").
const unplacedBy = why =>
    ACTS.find(one => one.act === "actRepository" && one.says.test(String(why ?? "")));
function asksWhere(how) {
    return how?.started === false && how.unanswered !== true
        && (how.refusal === "needs_folder" || unplacedBy(how.why) !== undefined);
}

// Whether `launcher` may step Joel back to pick where act `key` starts: he's
// still in its emptied field, no key since his Return (`letters` its
// lettersAt then), and `folders` are listed to pick from.
function stepsBack(launcher, key, letters, folders) {
    return launcher?.open === true && launcher.asking?.key === key
        && launcher.lettersAt === letters && folders > 0;
}

// How a start that asks where is told, in `said`'s words: the refusal, the
// command line's act cut, then that his words go with the folder he picks;
// as `told` says it where those words are missing or the cut isn't there.
function toldWhere(how, said) {
    const words = String(how.why ?? "");
    const known = unplacedBy(words);
    const where = String(said?.askedWhere ?? "");
    if (where === "" || (known?.cut && !known.cut.test(words))) {
        return told(how, said);
    }
    return `${known?.cut ? words.replace(known.cut, "") : words}; ${where}`;
}

// What the act that ran says of how it ended, in `said`'s words: one
// refused ends with what gets past it (acted), and one the node started
// with no holder's id can't be found to open, so it promises none.
function told(how, said) {
    if (!how.started) {
        return how.unanswered === true ? how.why : acted(how.why, said);
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

// The ends a notice of `ends` under `summary` says, given the notices
// (Notices' history, `{ id, app, summary, body, read }`, `body` its words
// plain) and those still open (`live`, by id): led by what an unread one
// of Agents' own still open under that summary says, since the notices
// daemon folds a newer notice from one program under one summary into
// the older, which it marks read, so the newer stands for both.
function noticed(ends, summary, history, live) {
    const open = (history ?? []).find(notice => notice.app === "Agents"
        && notice.summary === summary && !notice.read && notice.id in (live ?? {})
        && notice.body !== "");
    return open === undefined ? ends : [open.body].concat(ends);
}

// What Agents sent as a notice of `ends` under `summary`, with the ids its
// summary's notices had in the history then, so `unheard` knows the one
// sent from those before it.
function sentOf(ends, summary, history) {
    const ids = (history ?? []).filter(notice => notice.app === "Agents"
        && notice.summary === summary).map(notice => notice.id);
    return { ends, summary, ids };
}

// The ends of the notice Agents last sent (`sent`, from sentOf) while the
// history doesn't yet hold it: no notice of Agents' under its summary, new
// since, says just its ends, as the history keeps a body plain. Until the
// daemon's answer comes back, the history can't say the sent one is open,
// though the daemon will fold it into the next all the same; an earlier
// send still on its way says less, so it never stands for this one. One
// never heard, as when notify-send failed, leads on, since its ends reached
// no card, until one leading with them is heard; then null, when `noticed`
// says what leads.
function unheard(sent, history) {
    if (sent === null || sent === undefined) {
        return null;
    }
    const says = sent.ends.map(String).join("\n");
    const heard = (history ?? []).some(notice => notice.app === "Agents"
        && notice.summary === sent.summary && !sent.ids.includes(notice.id)
        && notice.body === says);
    return heard ? null : sent.ends;
}

// The notice of the ends `fresh` under `summary`, given the one last sent
// (`sent`) and the notices (`{ history, live }`, as `noticed` reads them):
// `{ ends, sent }`, led by the sent one the history doesn't hold yet
// (unheard), else by an unread one still open (noticed), since the daemon
// folds either away into it, and what it sends, for the next.
function sproutNotice(fresh, sent, summary, notices) {
    const waiting = unheard(sent, notices.history);
    const ends = waiting !== null ? waiting.concat(fresh)
        : noticed(fresh, summary, notices.history, notices.live);
    return { ends, sent: sentOf(ends, summary, notices.history) };
}
