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
// tray counts it.
pragma ComponentBehavior: Bound
import qs
import Quickshell
import QtQuick
import "../Fuzzy.js" as Fuzzy

Provider {
    id: provider

    what: "ask"

    collection: Words.collection("asks")

    // The daemon's rows of Asks, watched whether or not the launcher's
    // open, so an ask put up while it's shut is in For You as it opens.
    ReadyRows {
        id: ready
        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/secret-asks.sock`
        place: "asks"
        beside: ["summary"]
        make: (row, key) => provider.made(row, key)
        maker: provider
        drawn: provider.launcher?.open ?? false
    }

    readonly property string summaryKey: "summary:asks"

    // Newest first, as the daemon serves them.
    readonly property var all: ready.rows()

    // An ask's row, from the one the daemon serves, with what For You
    // reads of it: whether it waits on Joel, and since when.
    function made(served, key) {
        return ready.thing(served, key, {
            thing: true,
            key: served.thing,
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
