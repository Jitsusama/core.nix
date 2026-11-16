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

    // Whether the passwords daemon answers the asks, and the connection
    // Passwords holds to it, once it has one.
    readonly property bool overPasswords: settings.over === "passwords"
    readonly property var passwordsLink: overPasswords
        ? (launcher?.sources ?? []).find(source => source.asksLink)?.asksLink ?? null : null

    // When each ask's line, by its row's key, was first presented selected
    // this opening, and its digest then (Shown.js's noted); let go as the
    // launcher shuts, so a line is read again each opening.
    property var seen: ({})

    // The selected row, if it's an ask's, noted from the next frame niri
    // presents, the first that draws it selected; again when its line
    // changes. Nothing is noted where frames can't be timed, so no yes
    // there says it was read, nor while the launcher is shut, when no
    // frame draws it. Every other line's frame is let go of first, so one
    // left and come back to counts from its return.
    function noteSelected() {
        if (!(launcher?.open ?? false))
            return;
        const key = launcher?.selected?.shownKey;
        provider.seen = Shown.kept(provider.seen, key);
        if (typeof key !== "string")
            return;
        const digest = ready.served(key)?.digest;
        if (typeof digest !== "string" || seen[key]?.digest === digest)
            return;
        launcher.presentedAwait(KeyTimes.now(), at => {
            if (launcher.open)
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
        function onOpenChanged() {
            if (!provider.launcher.open)
                provider.seen = {};
            else
                provider.noteSelected();
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
        return ready.thing(served, key, {
            thing: true,
            key: served.thing,
            // The record's key, by which a frame presenting it is noted.
            shownKey: key,
            waiting: served.state === "waiting",
            asked: served.attention ?? null,
            // A waiting ask arms on its first Return, saying what a second
            // does; an answered one has no act to arm.
            arms: served.state === "waiting" ? Words.of("verb:release").said?.arms ?? "" : "",
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
