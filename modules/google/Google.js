// What the shell reads of the Google account the google program keeps
// (src/state.rs), from the one line it leaves in google.json. Google.qml
// holds what's read, and the checks run it in Qt's own engine
// (tests/mail.qml).
.pragma library

// What the program said, or null when the line isn't its state, so a
// half-written line or a stray one changes nothing. Mail, the unread
// count and the documents are optional, as a program from before them
// doesn't say them.
function heard(line) {
    let said;
    try {
        said = JSON.parse(line);
    } catch (_error) {
        return null;
    }
    if (!said || typeof said.account !== "string" || !Array.isArray(said.meetings)) {
        return null;
    }
    return {
        account: said.account,
        meetings: said.meetings,
        mail: Array.isArray(said.mail) ? said.mail : [],
        unread: typeof said.unread === "number" ? said.unread : null,
        documents: Array.isArray(said.documents) ? said.documents : [],
        heard: said.heard ?? null
    };
}

// Everyone the account's meetings and mail name, as People's Google
// source says them (PersonSource.qml): each who holds or is asked to a
// meeting, and each who sent mail that wants Joel, by their address, with
// the name they're given. All of them are recent, since the meetings run
// from today to a week on and the mail is what waits on him now. One with
// no address can't be one person across sources, so is left out; who's
// named more than once is said once, by the first name given.
function people(meetings, mail) {
    const found = new Map();
    const add = (email, name) => {
        const address = String(email ?? "").trim().toLowerCase();
        if (address !== "" && !found.has(address)) {
            found.set(address, { emails: [address], name: name ?? "", recent: true });
        }
    };
    for (const meeting of meetings ?? []) {
        add(meeting.organizer?.email, meeting.organizer?.name);
        for (const guest of meeting.guests ?? []) {
            add(guest.email, guest.name);
        }
    }
    for (const one of mail ?? []) {
        add(one.address, one.from);
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
