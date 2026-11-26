.pragma library

// The notice Start New Work gives of how starts ended, when Joel left the
// launcher before its foot could tell him (Agents.qml's sproutNoticed and
// sproutNotify). The notices daemon folds a newer notice from one program
// under one summary into the older, so each notice leads with what the one
// it folds away said, and only one stands for all of them.

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
// text, since a notice's body is markup and the node writes the why, and
// each backslash doubled, as notify-send reads escapes (g_strcompress), so
// the history keeps the ends' words: one notice, which no fold hides.
function noticeBody(ends) {
    return ends.map(text => String(text).replace(/\\/gu, "\\\\").replace(/&/gu, "&amp;")
        .replace(/</gu, "&lt;").replace(/>/gu, "&gt;")).join("\n");
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
// though the daemon folds it into the next all the same; an earlier send
// on its way says less, so never stands for this one. One never heard, as
// when notify-send failed, leads on until one leading with its ends is
// heard; then null, when `noticed` says what leads.
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
