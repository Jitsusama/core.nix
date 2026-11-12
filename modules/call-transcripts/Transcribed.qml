// The launcher's Transcripts: every call call-transcripts wrote down,
// newest first, each the call's one row (Transcripts.row), so it's the
// same here as in someone's Last Met or a meeting's. It's a collection,
// since a year of calls is thousands: a search with a match adds one
// row, found by a call's title or by who it was with, by name or
// address, and its word, "transcripts", lists them all.
//
// A meeting's Open the Transcript is lent from here: google's daemon
// serves it as `transcript:<meeting id>`, standing only where lent
// (ready-rows.md's lentOnly), and `lent` makes it for a meeting whose
// call was written down; `transcriptOf(event)` is that call's row, which
// Calendar.qml asks for a meeting the daemon doesn't serve yet. On a
// machine without call-transcripts nothing lends it, and no meeting
// offers one.
import qs
import QtQuick
import "../Fuzzy.js" as Fuzzy

Provider {
    id: provider

    what: "transcript"

    collection: Words.collection("transcripts")

    // Each call's row, made again only once the daemon says that call
    // changed, since it says the call whole: a call going on changes
    // every few seconds, and making every other call's row again each
    // time would cost as many as there are.
    readonly property var made: ({})

    readonly property var all: {
        const rows = Transcripts.calls.map(call => {
            const had = made[call.key];
            return had && had.call === call ? had : Transcripts.row(call, provider);
        });
        for (const key of Object.keys(made))
            delete made[key];
        for (const row of rows)
            made[row.call.key] = row;
        return rows;
    }
    onAllChanged: changedTo(all)

    // Each saying when it was as of now, since a call yesterday was today
    // when its row was made.
    function dated(rows) {
        const now = Date.now();
        for (const row of rows)
            row.note = Transcripts.noted(row.call, now);
        return rows;
    }

    function everything() {
        return dated(all.slice());
    }

    function looksThrough() {
        return all;
    }

    // Its newest call's row, for a meeting's Open the Transcript, or null.
    function transcriptOf(event) {
        const call = Transcripts.ofEvent(event);
        return call ? Transcripts.row(call, provider) : null;
    }

    // A meeting's Open the Transcript, as google's daemon serves it lent:
    // its call's transcript opened in the reader, closing the launcher, as
    // Return on the call's row does, with its summary, or its first lines,
    // in the drawer, each read from the call as it is then; null for a
    // meeting with no call written down, which then offers none.
    function lent(thing) {
        const prefix = "transcript:";
        const event = String(thing).startsWith(prefix) ? String(thing).slice(prefix.length) : "";
        const call = event === "" ? null : transcriptOf(event);
        if (!call || call.verb === "more")
            return null;
        const key = "verb:calendar:transcript";
        return provider.thing({
            key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            later: true,
            run: () => transcriptOf(event)?.run(),
            drawer: () => transcriptOf(event)?.drawer() ?? null
        });
    }

    // Which meetings' calls are written down, so only a call being written
    // down, or gone, makes the rows lent again, not each line of a call
    // going on.
    readonly property string lentRevision: Transcripts.calls
        .filter(call => call.event && call.transcript)
        .map(call => `${call.event}:${call.transcript}`)
        .join("\n")

    // A call is found by its title, or failing that by who it was with,
    // which counts for less, newest first among equals, since a call is
    // wanted by when it was.
    function search(typed) {
        const words = Fuzzy.words(typed);
        const need = Fuzzy.letters(words);
        const found = [];
        for (const row of poolFor(all, typed)) {
            const byName = (row.prepared.mask & need) === need
                ? Fuzzy.match(row.prepared, words, false) : { score: -1, grade: 3 };
            const byWho = byName.score < 0 && row.keywords
                ? Fuzzy.match(row.keywords, words, false) : { score: -1, grade: 3 };
            const best = byName.score >= 0 ? byName
                : { score: byWho.score < 0 ? -1 : byWho.score / 2, grade: byWho.grade };
            if (best.score >= 0)
                found.push({ score: best.score, grade: best.grade, row: row });
        }
        matchedIn(all, typed, found.map(each => each.row));
        let finest = 3;
        for (const each of found)
            finest = Math.min(finest, each.grade);
        const kept = found.filter(each => each.grade === finest)
            .sort((a, b) => b.score - a.score || b.row.at - a.row.at);
        return { rows: dated(kept.map(each => each.row)), grade: finest };
    }
}
