// What the shell reads of the Google account the google program keeps
// (src/state.rs), from the records its daemon serves (src/daemon.rs).
// Google.qml holds what's read, and the checks run it in Qt's own engine
// (tests/mail.qml).
.pragma library

// The account's one record, which says how it stands and when Google
// last answered.
const ACCOUNT = "account:google";

// The kinds of document, in the order the program looks them up
// (src/drive.rs's Kind::ALL), each kind's last looked at first.
const DOCUMENTS = ["document", "spreadsheet", "presentation"];

// The kinds of record the daemon keeps besides the account
// (src/daemon.rs's KINDS), each by key.
const KINDS = ["meeting", "mail", "file", "held", "lately", "call"];

// What's kept of the daemon's records: the account's, and each other
// kind's things by key, so a change remakes only the parts whose keys it
// names (Daemon.qml's heard). The account's record is told again each
// time Google answers, and every list was walked and its text made anew
// at each, 169 records about twice a minute on Joel's shell (RCTRF8,
// 2026-11-10).
function keeping() {
    const kept = { account: null };
    for (const kind of KINDS) {
        kept[kind] = {};
    }
    return kept;
}

// Every kind, as moved, for a part to be remade whole.
function everyKind() {
    const every = { account: true };
    for (const kind of KINDS) {
        every[kind] = true;
    }
    return every;
}

// Takes what `held` (Daemon.qml's) says of these keys into `kept`, and
// says which kinds moved, each true. A key's kind is how it begins, as the
// daemon makes them, so a key gone is placed without its record.
function keep(kept, held, keys) {
    const moved = {};
    for (const key of keys) {
        const kind = key.slice(0, key.indexOf(":"));
        const record = held?.[key];
        const thing = record?.kind === kind ? record[kind] : undefined;
        if (kind === "account") {
            if (key !== ACCOUNT) {
                continue;
            }
            kept.account = record ?? null;
        } else if (!KINDS.includes(kind)) {
            continue;
        } else if (thing) {
            kept[kind][key] = thing;
        } else {
            delete kept[kind][key];
        }
        moved[kind] = true;
    }
    return moved;
}

// The parts of the account as the program keeps it (stateOf's) that the
// kinds moved make, from what's kept. A record carries no place, so each
// kind is put back in its order by its own fields: meetings by when they
// start, mail newest first, documents by kind and then last looked at
// first. Until the daemon answers and says how the account stands, as on
// a machine nobody signed in to Google on, there's no account, and every
// part is none; nor once it stops answering, since what it held may not
// be Joel's to see any more.
function partsOf(kept, moved, answering) {
    if (!ready(kept, answering)) {
        return stateOf({ account: "none", meetings: [] });
    }
    const account = kept.account;
    const of = kind => Object.values(kept[kind]);
    const rank = kind => {
        const at = DOCUMENTS.indexOf(kind);
        return at < 0 ? DOCUMENTS.length : at;
    };
    const parts = {};
    if (moved.meeting) {
        parts.meetings = of("meeting").sort((one, other) => one.start - other.start);
    }
    if (moved.mail) {
        // Mail done is still a mail record, saying how it went, so what
        // wants Joel is the mail without a `done` (src/done.rs).
        parts.mail = of("mail").filter(mail => !mail.done)
            .sort((one, other) => other.at - one.at);
        parts.finished = of("mail").filter(mail => mail.done)
            .sort((one, other) => other.done.at - one.done.at);
    }
    if (moved.file) {
        parts.documents = of("file").sort((one, other) => rank(one.kind) - rank(other.kind)
            || (other.viewed ?? 0) - (one.viewed ?? 0));
    }
    if (moved.held) {
        parts.held = of("held").sort((one, other) => one.start - other.start);
    }
    if (moved.lately) {
        parts.lately = of("lately").sort((one, other) => other.at - one.at);
    }
    if (moved.call) {
        parts.calls = of("call");
    }
    if (moved.account) {
        const own = stateOf({ account: account.account.standing, meetings: [],
            unread: account.account.unread, heard: account.heard,
            fetched: account.account.fetched, meet: account.account.meet,
            drive: account.account.drive });
        for (const part of ["account", "unread", "heard", "fetched", "meet", "drive"]) {
            parts[part] = own[part];
        }
    }
    return parts;
}

// Whether there's an account to say: the daemon answers and has said how
// it stands. Until there is, every part is none, so once there is every
// part is remade, whichever kinds moved.
function ready(kept, answering) {
    return answering && typeof kept.account?.account?.standing === "string";
}

// The account as the program keeps it, gathered whole from the daemon's
// records, held by key, as partsOf says each part.
function gathered(held, answering) {
    const kept = keeping();
    keep(kept, held, Object.keys(held ?? {}));
    return stateOf(Object.assign({ meetings: [] }, partsOf(kept, everyKind(), answering)));
}

// The account in the program's shape, or null when `said` isn't one.
// Mail, the unread count, the documents and the rest are optional, each
// taken as none when it isn't said.
function stateOf(said) {
    if (!said || typeof said.account !== "string" || !Array.isArray(said.meetings)) {
        return null;
    }
    return {
        account: said.account,
        meetings: said.meetings,
        mail: Array.isArray(said.mail) ? said.mail : [],
        finished: Array.isArray(said.finished) ? said.finished : [],
        unread: typeof said.unread === "number" ? said.unread : null,
        documents: Array.isArray(said.documents) ? said.documents : [],
        held: Array.isArray(said.held) ? said.held : [],
        lately: Array.isArray(said.lately) ? said.lately : [],
        calls: Array.isArray(said.calls) ? said.calls : [],
        heard: said.heard ?? null,
        fetched: said.fetched ?? null,
        meet: { granted: said.meet?.granted === true, made: said.meet?.made ?? null },
        drive: {
            listings: Array.isArray(said.drive?.listings) ? said.drive.listings : [],
            downloads: Array.isArray(said.drive?.downloads) ? said.drive.downloads : [],
            uploads: Array.isArray(said.drive?.uploads) ? said.drive.uploads : [],
            places: placesOf(said.drive?.places),
            kept: keptOf(said.drive?.kept)
        }
    };
}

// How much Drive's copies here take and may take, in bytes (src/drive.rs's
// Kept), none from a program before it.
function keptOf(kept) {
    const bytes = value => typeof value === "number" && value >= 0 ? value : 0;
    return { held: bytes(kept?.held), most: bytes(kept?.most) };
}

// Drive's own places as the program says them (src/drive.rs's Places):
// its Recent, newest first, and Starred, Shared with Me and the shared
// drives, each { files, more }; any not said holds none, as from a
// program before them.
function placesOf(places) {
    const placed = place => ({
        files: Array.isArray(place?.files) ? place.files : [],
        more: place?.more === true
    });
    return {
        recent: Array.isArray(places?.recent) ? places.recent : [],
        starred: placed(places?.starred),
        shared: placed(places?.shared),
        drives: placed(places?.drives)
    };
}

// The parts of what the program said whose text differs from `held`, the
// text each part was last taken from, with the texts to hold now. The
// account's record changes each time Google answers, most minutes only
// to say when, and a part assigned again, even the same, remakes
// everything bound to it: People's whole list, each minute, was most of
// the closed shell's time (2026-10-30).
function changed(held, said) {
    const texts = Object.assign({}, held);
    const parts = [];
    for (const part of Object.keys(said)) {
        const text = JSON.stringify(said[part]);
        if (text !== held[part]) {
            texts[part] = text;
            parts.push(part);
        }
    }
    return { texts, parts };
}

// The line that asks the program what a Drive folder holds: "root" (My
// Drive), "shared" (Shared with Me), "drives" (the shared drives) or a
// folder's id.
function list(folder) {
    return `${JSON.stringify({ list: { folder } })}\n`;
}

// The line asking for a save the picker staged to be sent up again, by
// where it's staged.
function upload(path) {
    return `${JSON.stringify({ upload: { path } })}\n`;
}

// The line that asks the program to download a Drive file to "open",
// "pick" (for the program that asked the picker) or "save", a save going
// `to` the path Joel picked, or into Downloads when none is given.
function download(file, how, to) {
    const asked = { file, how };
    if (how === "save" && to) {
        asked.to = to;
    }
    return `${JSON.stringify({ download: asked })}\n`;
}

// What the Drive folder asked for at `since` holds, as the program last
// listed it, { folder, asked, files, more, failed }; null until it's said.
function listing(drive, folder, since) {
    return (drive?.listings ?? []).find(each => each.folder === folder && each.asked >= since)
        ?? null;
}

// How the Drive file asked for at `since` stands, as { state, path,
// failed }: "fetching" until the program says where the newest one asked
// since is, then "done" with its path, or "failed" with why; "" when none
// was asked.
function downloading(drive, file, since) {
    if (since === null) {
        return { state: "", path: "", failed: "" };
    }
    const asked = (drive?.downloads ?? []).find(each => each.file === file && each.asked >= since);
    if (!asked || !(asked.path || asked.failed)) {
        return { state: "fetching", path: "", failed: "" };
    }
    return asked.path
        ? { state: "done", path: asked.path, failed: "" }
        : { state: "failed", path: "", failed: asked.failed };
}

// The line that asks the program to make a new Meet meeting.
function make() {
    return `${JSON.stringify({ make: {} })}\n`;
}

// The ask that follows the Meet calls of these codes, those open in
// windows of their own, so the program asks Meet who's in each.
function follow(codes) {
    return `${JSON.stringify({ follow: { codes } })}\n`;
}

// How the meeting asked for at `since` stands, as { state, link }: "making"
// until the program says it made one asked since, then "made" with its
// link, or "failed"; "" when none was asked.
function making(meet, since) {
    if (since === null) {
        return { state: "", link: "" };
    }
    const made = meet?.made;
    if (!made || made.asked < since || !(made.link || made.failed)) {
        return { state: "making", link: "" };
    }
    return made.link ? { state: "made", link: made.link } : { state: "failed", link: "" };
}

// Whether a meeting or a piece of mail is with someone by one of their
// addresses: they hold or are asked to it, or they sent it.
function withThem(thing, emails) {
    const has = email => emails.includes(String(email ?? "").toLowerCase());
    return has(thing.address) || has(thing.organizer?.email)
        || (thing.guests ?? []).some(guest => has(guest.email));
}

// Whether a piece of mail Joel sent or read is with someone: they sent
// it, or it went to them or was copied to them, as mail Joel sent them is.
function mailedWith(mail, emails) {
    const has = email => emails.includes(String(email ?? "").toLowerCase());
    return has(mail.address) || (mail.others ?? []).some(has);
}

// The meetings Joel held lately, as Calendar's Recent lists them and Meet's
// finds its calls among: each over, from the month before today the
// program keeps (`held`) and from today's (`meetings`), newest first by
// when each started, as the times their rows say read down, each once.
// One he said no to is left out, since he never held it.
function heldLately(held, meetings, now) {
    const seen = new Set();
    return (held ?? []).concat(meetings ?? [])
        .filter(meeting => meeting.end <= now && meeting.answer !== "no")
        .sort((one, other) => other.start - one.start)
        .filter(meeting => !seen.has(meeting.id) && seen.add(meeting.id));
}

// The series a meeting is one of, by Google's own id for it, which the
// daemon passes on (`series`, its recurringEventId), never read off an
// instance's id: a series split at "this and following" is two, as Google
// keeps it. A meeting held once is its own.
function seriesOf(meeting) {
    return String(meeting.series ?? meeting.id ?? "");
}

// The meetings held lately as Calendar's Recent lists them: a series once,
// at its latest, with how many of it came before as `earlier`, so a daily
// meeting takes one place, not nine (VKCF3Z, ccccbe5f). `lately` is
// heldLately's, newest first, which Meet's calls are still found among.
function seriesOnce(lately) {
    const bySeries = new Map();
    for (const meeting of lately ?? []) {
        const series = seriesOf(meeting);
        const latest = bySeries.get(series);
        if (latest) {
            latest.earlier += 1;
        } else {
            bySeries.set(series, Object.assign({}, meeting, { earlier: 0 }));
        }
    }
    return [...bySeries.values()];
}

// How many of its series came before one a Recent lists, in `said`'s
// words, "8 in the past 30 days", or nothing when none did.
function earlier(count, said) {
    return count > 0 ? String(said?.earlier ?? "").replace("{count}", count) : "";
}
