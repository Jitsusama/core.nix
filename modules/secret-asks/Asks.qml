// Asks, a collection inside Passwords: each agent's ask for a secret, as
// the secret-asks daemon serves it (src/row.rs) over one connection to its
// socket in the ready-row form (ReadyRows.qml), so nothing here words or
// sorts an ask, and the daemon is what decides one. The first Return on a
// waiting ask arms it in place, the foot saying "↵ again releases", and a
// second with no key between does its first act, Release It, as Log Out's
// does (Arming.js; VKCF3Z, 2026-11-12), asking the daemon with the digest
// it served the row with, so a yes for an ask that changed since is
// refused there and drawn again. r on it refuses it, its words' key for
// Refuse It, never y, which copies, nor Escape; the right arrow lists both.
// Each act keeps the launcher open, so Joel sees the row become releasing,
// ran or refused, and a refusal is said in the daemon's words. A waiting
// ask is also one of For You's things to do (`forYou`, which Now.qml
// gathers with every source's), so home's On row leads with it and the
// tray counts it. A standing yes, which lets the same ask run for a week
// (src/grant.rs), is a row too, never waiting, so it neither arms nor
// counts in For You, and its Return does its one act, Forget It.
//
// Where the passwords daemon answers the asks (`over` is "passwords"), they
// come over Passwords' own connection to its daemon, the shell's child, and
// no socket is opened. Each yes there says how long its line was on the
// screen (Shown.js), which that daemon needs before it releases anything.
pragma ComponentBehavior: Bound
import qs
import Quickshell
import QtQuick
import KeyTimes
import "../Fuzzy.js" as Fuzzy
import "Shared.js" as Shared
import "Shown.js" as Shown

Provider {
    id: provider

    what: "ask"

    collection: Words.collection("asks")

    // The daemon's rows of Asks, watched whether or not the launcher's
    // open, so an ask put up while it's shut is in For You as it opens.
    ReadyRows {
        id: ready
        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/secret-asks.sock`
        // Over Passwords' connection, never one of its own while waiting
        // for it, so the asks folder is only ever the child's.
        over: provider.passwordsLink
        active: provider.overPasswords ? provider.passwordsLink !== null
            : !!Quickshell.env("XDG_RUNTIME_DIR")
        stamp: (key, ask) => provider.stamped(key, ask)
        place: "asks"
        beside: ["summary"]
        make: (row, key) => provider.made(row, key)
        maker: provider
        drawn: provider.launcher?.open ?? false
    }

    readonly property string summaryKey: "summary:asks"

    // Whether each ask hides what it asks for: while niri says a call or
    // any other program streams the screen, a recording of one window
    // aside, and until niri has said which casts there are (Shared.js, as
    // Copies hides a copy). Joel shows people his desktop through the
    // launcher, and an ask names the secret, the tree and the why, so each
    // row then says only that it's hidden. It has no act, since a yes for
    // a line he can't read would release what he never saw, and no frame
    // of it is timed, so shown again it counts nothing until his next
    // move. It still waits in For You, hidden there too.
    readonly property bool hiding: Shared.hidden(Niri.castsHeard, Niri.casts,
        Recorder.windowSession)
    onHidingChanged: {
        forget();
        ready.remade();
        changed();
    }

    // Whether the passwords daemon answers the asks, and the connection
    // Passwords holds to it, once it has one.
    readonly property bool overPasswords: settings.over === "passwords"
    readonly property var passwordsLink: overPasswords
        ? (launcher?.sources ?? []).find(source => source.asksLink)?.asksLink ?? null : null

    // When each ask's line, by its row's key, was first presented selected
    // this opening, and its digest then (Shown.js's noted); let go as the
    // launcher shuts, so a line is read again each opening.
    property var seen: ({})
    // Counted up each time the selection or the opening moves, so a frame
    // asked for before the move notes nothing: it drew a line since left.
    property int noting: 0

    // The line, by its row's key, that one of Joel's keys moving over the
    // rows armed, so it's timed from its next frame. Every opening may be
    // asked from outside, as any process running as him may ask one while
    // his own keys land there, typed into another window, and those keys
    // are mostly Returns, letters, arrows and Escape: a Return, a step into
    // a place, or an Up or Down on the find, where a terminal's history
    // would take it, times nothing, so only a move of his over the rows does
    // (Shown.js's keyed; ISXEG8, 2026-11-17). The cost is two keys more on
    // an ask he lands on from the find, Escape and j, said in the refusal
    // of a yes that comes without them.
    property string armed: ""
    // The keys taken since Asks last read them, by name, oldest first; how
    // many keys the line had taken when the last ask from outside came, so
    // a key read after that ask arms nothing it selected; and how many rows
    // were stepped into when the keys were last read, so a key stepping
    // into the armed ask's verbs is known.
    property var taken: []
    property int keysAtOutside: 0
    property int depth: 0
    // Whether the keys came over the rows by an Escape of his, read since
    // the opening and the last ask from outside. A Return that lands, the
    // command line, or an ask from outside while they're already there
    // puts them over the rows too, as a terminal's keys could, so a move
    // arms only after the Escape the refusal names.
    property bool escaped: false

    // Every line's time started again, and any frame asked for let go.
    function forget() {
        provider.armed = "";
        provider.noting += 1;
        provider.seen = {};
    }

    // The keys of his since the last read, once the launcher has done what
    // they do: a move over the rows arms the line selected after it, Return
    // leaves every line as it was, as does one key stepping into the armed
    // ask's verbs, whose Release It is that ask's yes, and any other starts
    // every line's time again, as does an ask from outside that came after
    // them.
    function keyed() {
        const names = provider.taken;
        provider.taken = [];
        const held = launcher?.held ?? [];
        const deeper = held.length > provider.depth;
        provider.depth = held.length;
        if ((launcher?.keysTaken ?? 0) === provider.keysAtOutside) {
            provider.forget();
            return;
        }
        const over = launcher?.keysOverRows ?? false;
        const moved = (launcher?.keyMoved ?? false) && over && provider.escaped;
        provider.escaped = over && (provider.escaped || names.includes("Escape"));
        const does = Shown.keyed(names, moved);
        const stepped = held[held.length - 1]?.row?.shownKey;
        if (does === "passes" || deeper && names.length === 1 && provider.armed !== ""
            && stepped === provider.armed)
            return;
        if (does === "resets") {
            provider.forget();
            return;
        }
        const key = launcher?.selected?.shownKey ?? stepped;
        provider.armed = typeof key === "string" ? key : "";
        provider.noteSelected();
    }

    // The selected row, if it's an ask's, noted from the next frame niri
    // presents, the first that draws it selected; again when its line
    // changes. Nothing is noted where frames can't be timed, so no yes
    // there says it was read, nor while the launcher is shut, when no
    // frame draws it. Every other line's frame is let go of first, so one
    // left and come back to counts from its return.
    function noteSelected() {
        provider.noting += 1;
        const asked = provider.noting;
        if (!(launcher?.open ?? false))
            return;
        const selected = launcher?.selected ?? null;
        // One of an ask's verbs, listed by the right arrow, keeps its ask's
        // line counted from when it was shown, since Release It there is
        // that ask's yes; a frame of the verbs never draws the line anew.
        const stepped = launcher?.held?.[launcher.held.length - 1]?.row?.shownKey;
        if (typeof selected?.shownKey !== "string" && typeof stepped === "string") {
            provider.seen = Shown.kept(provider.seen, stepped);
            return;
        }
        const key = selected?.shownKey;
        provider.seen = Shown.kept(provider.seen, key);
        if (typeof key !== "string")
            return;
        if (provider.armed !== key) {
            provider.seen = {};
            return;
        }
        // The digest of the row as it's drawn, which a frame presents, not
        // of the ask as served since, before the row is made again.
        const digest = selected.digest;
        if (typeof digest !== "string" || seen[key]?.digest === digest)
            return;
        launcher.presentedAwait(KeyTimes.now(), at => {
            if (launcher.open && provider.noting === asked)
                provider.seen = Shown.noted(provider.seen, key, digest, at);
        });
    }

    // A verb's ask as it's sent, a yes saying how long its line was shown,
    // or null for a yes walked from outside the launcher, which is never
    // sent. Only the passwords child reads the time; the experiments' own
    // daemon is sent the ask as served.
    function stamped(key, ask) {
        const said = Shown.stamped(ask, seen, key, {
            now: KeyTimes.now(),
            opening: launcher?.openingsDrawn ?? 0,
            cataloguing: launcher?.cataloguing ?? true
        });
        return overPasswords || said === null ? said : ask;
    }

    Connections {
        target: provider.launcher
        enabled: provider.overPasswords
        function onSelectedChanged() {
            provider.noteSelected();
        }
        // An ask still selected as the launcher opens again changes no
        // selection, so its first frame is noted here or its yes would
        // go with no time shown.
        // Every opening may be asked from outside, so none arms a line.
        function onOpenChanged() {
            provider.armed = "";
            provider.escaped = false;
            provider.depth = (provider.launcher.held ?? []).length;
            if (!provider.launcher.open) {
                provider.noting += 1;
                provider.seen = {};
            } else {
                provider.noteSelected();
            }
        }
        function onHeldChanged() {
            provider.noteSelected();
        }
        // Read once the key's done what it does, as its name comes before
        // its count and whether it moved after (Prompt.qml's modalKey).
        // Its name is read now, before the next key's replaces it, as keys
        // typed fast can come in one turn and be read together.
        function onKeysTakenChanged() {
            provider.taken = provider.taken.concat([provider.launcher.keyTaken]);
            Qt.callLater(provider.keyed);
        }
        // An ask from outside may select any line, so none stays his.
        function onToldFromOutsideChanged() {
            provider.keysAtOutside = provider.launcher.keysTaken;
            provider.escaped = false;
            provider.forget();
        }
    }

    // Newest first, as the daemon serves them.
    readonly property var all: ready.rows()
    // Said as what they show moves, so the launcher open on Asks searches
    // it again and a yes is drawn ran there and then; without it, the row
    // read waiting until Asks was next opened (5GLTPH, 2026-11-15).
    onAllChanged: {
        changedTo(all);
        if (overPasswords && (launcher?.open ?? false))
            noteSelected();
    }

    // An ask's row, from the one the daemon serves, with what For You
    // reads of it: whether it waits on Joel, and since when.
    function made(served, key) {
        const hidden = Words.name("asks:hidden");
        const drawn = !hiding ? served : Object.assign({}, served, {
            name: hidden,
            note: "",
            acts: [],
            keywords: [],
            drawer: { eyebrow: "", title: hidden, body: "", facts: [] }
        });
        return ready.thing(drawn, key, {
            thing: true,
            key: served.thing,
            // The record's key, by which a frame presenting it is noted.
            shownKey: key,
            // The ask as this row draws it, which a frame of it presents.
            digest: hiding ? null : served.digest,
            waiting: served.state === "waiting",
            asked: served.attention ?? null,
            // A waiting ask arms on its first Return, saying what a second
            // does; an answered one has no act to arm.
            arms: served.state === "waiting" && !hiding
                ? Words.of("verb:release").said?.arms ?? "" : "",
            // What it is and where it's from, which For You filters by.
            forYou: { kind: "do", source: "asks" }
        });
    }

    // Each waiting ask as For You weighs a thing to do: it blocks the
    // agent that asked, so it's among what needs Joel now.
    readonly property var forYou: all.filter(row => row.waiting === true).map(row => ({
        row,
        thing: { kind: "todo", blocks: 1, ready: true, since: row.asked }
    }))

    function search(typed) {
        const words = Fuzzy.words(typed);
        const need = Fuzzy.letters(words);
        const found = [];
        for (const row of poolFor(all, typed)) {
            if ((row.prepared.mask & need) !== need)
                continue;
            const m = Fuzzy.match(row.prepared, words, false);
            if (m.score >= 0)
                found.push({ score: m.score, grade: m.grade, row: row });
        }
        matchedIn(all, typed, found.map(f => f.row));
        // Newest first among equals, the daemon's order, as an ask is
        // wanted by when it came.
        let finest = 3;
        for (const f of found)
            finest = Math.min(finest, f.grade);
        return { rows: found.filter(f => f.grade === finest).map(f => f.row), grade: finest };
    }

    function everything() {
        return all;
    }

    // How many wait, as the daemon says it.
    function counted() {
        void ready.placings;
        return String(ready.held[summaryKey]?.summary?.said ?? countedAs(all.length));
    }
}
