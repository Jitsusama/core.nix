// The launcher's things found by what they mean as well as by their
// letters: "dim the screen" finds Brightness. The meaning daemon
// (modules/meaning) holds each thing's meaning, made on the NPU from its
// name and the words it's known by, and is told them as the launcher
// opens (Meaning.js says which); typed into, the launcher asks `meant`,
// and the nearest few things not already found are added below
// everything typing found, each noting "found by its meaning" in the
// foot. It lists no row of its own and never ranks a guess over what was
// typed. Capture's Recent asks `takenBySight` alike, for the screenshots
// and camera pictures that look like what's typed, which it lists below
// those found by their names and words (launcher/Capture.qml).
//
// Nothing is asked on the shell's thread but a line down a socket: each
// text is asked once, the last few answers kept, and an answer coming
// later says the launcher found more, which adds the rows below without
// moving the selection or a row above (Launcher.qml's foundMore). The
// connection is held only while the launcher shows.
pragma ComponentBehavior: Bound

import qs
import Quickshell
import QtQuick
import "Meaning.js" as Meaning

Provider {
    id: provider

    // When the text now asked was first asked, so an answer that comes as
    // soon as the key is ranked as its own search.
    property real askedAt: 0
    // How soon that is, in milliseconds: the daemon answers from what it
    // holds, in a few on the NPU.
    readonly property int freshFor: 50
    // The most rows meaning adds, so what it finds never outnumbers what
    // was typed (Y2YBTS, 2026-11-07).
    readonly property int most: 3
    // How many the daemon is asked for: more than are shown, since some
    // are already found by their letters and left out.
    readonly property int asksFor: 8
    // The last few searches, newest last: { kind, words, records, whole },
    // records null while asked, of things or of pictures. One answered
    // before every thing had its meaning, or every picture was looked at
    // (whole false), is shown, and asked again a moment after.
    property var searches: []
    readonly property int keeps: 8
    readonly property int againAfter: 1000
    // How long the things are left to settle while the launcher is open
    // before they're told again (the Timer `settled`).
    readonly property int settlesIn: 500
    // Searches waiting for the connection, { kind, words }, asked once
    // it's made.
    property var pending: []
    // The things last told, as text, so they're told again only when they
    // change, never as a session's row does.
    property string toldAs: ""

    // The rows found by what's typed's meaning, best first, none of them
    // in `shown` (a set of identities), or none while it's asked.
    function meant(letters, shown) {
        const kept = provider.searched("thing", letters);
        if (kept === null)
            return [];
        return Meaning.rows(kept.records, provider.launcher?.keyed ?? {}, shown,
            { most: provider.most, note: Words.field.byMeaning ?? "" });
    }

    // The pictures that look like what's typed, nearest first, as paths,
    // none of them in `listed` (a set of paths), or none while they're
    // asked. Each is what the model saw in it, and nothing is said of it
    // but that it was found by its meaning.
    function takenBySight(letters, listed) {
        const kept = provider.searched("picture", letters);
        return kept === null ? [] : Meaning.paths(kept.records, listed, provider.most);
    }

    // What's typed's search of a kind, as kept, or null while it's asked,
    // asking it now when it wasn't.
    function searched(kind, letters) {
        const words = Meaning.asked(letters);
        if (!Meaning.enough(words))
            return null;
        const kept = provider.searches.find(search => search.kind === kind
            && search.words === words);
        if (!kept) {
            provider.searches = provider.searches
                .concat([{ kind: kind, words: words, records: null, whole: false }])
                .slice(-provider.keeps);
            provider.askedAt = Date.now();
            provider.request(kind, words);
            return null;
        }
        return kept.records === null ? null : kept;
    }

    function request(kind, words) {
        provider.pending = provider.pending
            .filter(asked => asked.kind !== kind || asked.words !== words)
            .concat([{ kind: kind, words: words }])
            .slice(-provider.keeps);
        reader.active = true;
        provider.send();
    }

    // Sends what waits for the connection. It isn't called `ask`, which
    // is Provider's own, for a URL.
    function send() {
        const daemon = reader.item as Daemon;
        if (!daemon || daemon.wire === null)
            return;
        const asking = provider.pending;
        provider.pending = [];
        for (const asked of asking)
            daemon.find({ ask: "search", kind: asked.kind, words: asked.words,
                limit: provider.asksFor },
                answer => provider.heard(asked.kind, asked.words, answer));
    }

    // A search's answer kept, unless the daemon wasn't there to give one,
    // when it's let go to be asked again. A daemon that looks at no
    // pictures says so, and is answered by nothing from then on. It isn't
    // called `answered`, which is Provider's own signal.
    function heard(kind, text, answer) {
        const other = search => search.kind !== kind || search.words !== text;
        const refused = answer?.error?.code === "unknown-kind";
        if (answer === null || (answer.error && !refused)) {
            provider.searches = provider.searches.filter(other);
            return;
        }
        const records = (answer.records ?? []).filter(record => typeof record?.key === "string");
        const whole = refused || answer.whole === true;
        provider.searches = provider.searches.map(search => other(search) ? search
            : { kind: kind, words: text, records: records, whole: whole });
        if (Meaning.asked(provider.launcher?.typed ?? "") !== text)
            return;
        provider.foundMore(Date.now() - provider.askedAt < provider.freshFor);
        if (!whole)
            again.restart();
    }

    // An answer given before every thing had its meaning, or every picture
    // was looked at, is asked again a moment later, while it's still what's
    // typed, so what the daemon learns meanwhile is added below without a
    // key.
    Timer {
        id: again

        interval: provider.againAfter
        onTriggered: {
            const words = Meaning.asked(provider.launcher?.typed ?? "");
            if (!provider.launcher?.open)
                return;
            for (const kept of provider.searches)
                if (kept.words === words && kept.records !== null && !kept.whole)
                    provider.request(kept.kind, words);
        }
    }

    // The things told to the daemon, when they've changed since.
    function tell() {
        const daemon = reader.item as Daemon;
        if (!daemon || daemon.wire === null || !provider.launcher)
            return;
        const telling = Meaning.telling(provider.launcher.choices);
        if (telling.as === provider.toldAs)
            return;
        provider.toldAs = telling.as;
        daemon.ask({ ask: "know", kind: "thing", things: telling.things }, null);
    }

    // The things change while the launcher is open as windows and notices
    // come and go, often several at once; walking them takes the shell's
    // thread about 5 ms over a thousand in Qt's engine (2026-11-08), so a
    // run of changes is told once, after it settles, never between keys.
    Timer {
        id: settled

        interval: provider.settlesIn
        onTriggered: provider.tell()
    }

    // The connection is made as the launcher opens, so the things are told
    // before the first key, and let go as it closes, with every answer:
    // the things may change while it's closed, and asking again costs the
    // daemon a few milliseconds.
    Connections {
        target: provider.launcher

        function onOpenChanged(): void {
            if (provider.launcher.open) {
                reader.active = true;
                Qt.callLater(provider.tell);
                return;
            }
            reader.active = false;
            settled.stop();
            provider.pending = [];
            provider.searches = [];
        }

        function onChoicesChanged(): void {
            if (provider.launcher.open)
                settled.restart();
        }
    }

    LazyLoader {
        id: reader

        active: false

        Daemon {
            socket: `${Quickshell.env("XDG_RUNTIME_DIR") ?? ""}/daemons/meaning.sock`
            kind: "thing"
            // No record is watched: a search's answer is given apart.
            watching: ({ keys: [] })
            // A new connection may be a new daemon, which knows nothing.
            onWireChanged: {
                provider.toldAs = "";
                provider.tell();
                provider.send();
            }
        }
    }
}
