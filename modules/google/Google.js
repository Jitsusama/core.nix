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

// The account as the program keeps it, gathered from the daemon's
// records, held by key. A record carries no place, so each kind is put
// back in its order by its own fields: meetings by when they start, mail
// newest first, documents by kind and then last looked at first. Until
// the daemon answers and says how the account stands, as on a machine
// nobody signed in to Google on, there's no account; nor once it stops
// answering, since what it held may not be Joel's to see any more.
function gathered(held, answering) {
    const account = answering ? held?.[ACCOUNT] : undefined;
    if (typeof account?.account?.standing !== "string") {
        return stateOf({ account: "none", meetings: [] });
    }
    const of = kind => Object.values(held)
        .filter(record => record?.kind === kind && record[kind])
        .map(record => record[kind]);
    const rank = kind => {
        const at = DOCUMENTS.indexOf(kind);
        return at < 0 ? DOCUMENTS.length : at;
    };
    return stateOf({
        account: account.account.standing,
        meetings: of("meeting").sort((one, other) => one.start - other.start),
        mail: of("mail").sort((one, other) => other.at - one.at),
        unread: account.account.unread,
        documents: of("file").sort((one, other) => rank(one.kind) - rank(other.kind)
            || (other.viewed ?? 0) - (one.viewed ?? 0)),
        held: of("held").sort((one, other) => one.start - other.start),
        lately: of("lately").sort((one, other) => other.at - one.at),
        heard: account.heard,
        fetched: account.account.fetched,
        meet: account.account.meet,
        drive: account.account.drive
    });
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
        unread: typeof said.unread === "number" ? said.unread : null,
        documents: Array.isArray(said.documents) ? said.documents : [],
        held: Array.isArray(said.held) ? said.held : [],
        lately: Array.isArray(said.lately) ? said.lately : [],
        heard: said.heard ?? null,
        fetched: said.fetched ?? null,
        meet: { granted: said.meet?.granted === true, made: said.meet?.made ?? null },
        drive: {
            listings: Array.isArray(said.drive?.listings) ? said.drive.listings : [],
            downloads: Array.isArray(said.drive?.downloads) ? said.drive.downloads : [],
            uploads: Array.isArray(said.drive?.uploads) ? said.drive.uploads : [],
            places: placesOf(said.drive?.places)
        }
    };
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

// Everyone the account's meetings and mail name, as People's Google
// source says them (PersonSource.qml): each who holds or is asked to a
// meeting, and each who sent mail that wants Joel, by their address, with
// the name they're given. One with no address can't be one person across
// sources, so is left out; who's named more than once is said once, by
// the first name given. Each says when those things are, a meeting from
// its start to its end and mail when it came, so People lists whoever
// there's been something with, the last first, and leaves someone only
// asked to a meeting to come to be found by typing.
function people(meetings, mail) {
    const found = new Map();
    const add = (email, name, span) => {
        const address = String(email ?? "").trim().toLowerCase();
        if (address === "") {
            return;
        }
        const had = found.get(address);
        if (had) {
            had.spans.push(span);
        } else {
            found.set(address, {
                emails: [address], name: name ?? "", spans: [span]
            });
        }
    };
    for (const meeting of meetings ?? []) {
        const span = { from: meeting.start, to: meeting.end };
        add(meeting.organizer?.email, meeting.organizer?.name, span);
        for (const guest of meeting.guests ?? []) {
            add(guest.email, guest.name, span);
        }
    }
    for (const one of mail ?? []) {
        add(one.address, one.from, { from: one.at, to: one.at });
    }
    return [...found.values()];
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
