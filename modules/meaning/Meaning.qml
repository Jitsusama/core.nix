// The launcher's things found by what they mean as well as by their
// letters: "dim the screen" finds Brightness. The meaning daemon
// (modules/meaning) holds each thing's meaning, made on the NPU from its
// name and the words it's known by, and is told them as the launcher
// opens (Meaning.js says which); typed into, the launcher asks `meant`,
// and the nearest few things not already found are added below
// everything typing found, each noting "found by its meaning" in the
// foot. It lists no row of its own and never ranks a guess over what was
// typed.
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
    // The last few searches, newest last: { words, records, whole },
    // records null while asked. One answered before every thing had its
    // meaning (whole false) is shown, and asked again a moment after.
    property var searches: []
    readonly property int keeps: 8
    readonly property int againAfter: 1000
    // Texts waiting for the connection, asked once it's made.
    property var pending: []
    // The things last told, as text, so they're told again only when they
    // change, never as a session's row does.
    property string toldAs: ""

    // The rows found by what's typed's meaning, best first, none of them
    // in `shown` (a set of identities), or none while it's asked.
    function meant(letters, shown) {
        const words = Meaning.asked(letters);
        if (!Meaning.enough(words))
            return [];
        const kept = provider.searches.find(search => search.words === words);
        if (!kept) {
            provider.searches = provider.searches
                .concat([{ words: words, records: null, whole: false }])
                .slice(-provider.keeps);
            provider.askedAt = Date.now();
            provider.request(words);
            return [];
        }
        if (kept.records === null)
            return [];
        return Meaning.rows(kept.records, provider.launcher?.keyed ?? {}, shown,
            { most: provider.most, note: Words.field.byMeaning ?? "" });
    }

    function request(words) {
        provider.pending = provider.pending.filter(text => text !== words).concat([words])
            .slice(-provider.keeps);
        reader.active = true;
        provider.send();
    }

    // The texts waiting sent, once there's a connection. It isn't called
    // `ask`, which is Provider's own.
    function send() {
        const daemon = reader.item as Daemon;
        if (!daemon || daemon.wire === null)
            return;
        const asking = provider.pending;
        provider.pending = [];
        for (const text of asking)
            daemon.find({ ask: "search", kind: "thing", words: text, limit: provider.asksFor },
                answer => provider.heard(text, answer));
    }

    // A text's answer kept, unless the daemon wasn't there to give one,
    // when it's let go to be asked again. It isn't called `answered`,
    // which is Provider's own signal.
    function heard(text, answer) {
        if (answer === null || answer.error) {
            provider.searches = provider.searches.filter(search => search.words !== text);
            return;
        }
        const records = (answer.records ?? []).filter(record => typeof record?.key === "string");
        provider.searches = provider.searches.map(search => search.words !== text ? search
            : { words: text, records: records, whole: answer.whole === true });
        if (Meaning.asked(provider.launcher?.typed ?? "") !== text)
            return;
        provider.foundMore(Date.now() - provider.askedAt < provider.freshFor);
        if (answer.whole !== true)
            again.restart();
    }

    // An answer given before every thing had its meaning is asked again a
    // moment later, while it's still what's typed, so what the daemon
    // learns meanwhile is added below without a key.
    Timer {
        id: again

        interval: provider.againAfter
        onTriggered: {
            const words = Meaning.asked(provider.launcher?.typed ?? "");
            const kept = provider.searches.find(search => search.words === words);
            if (provider.launcher?.open && kept && kept.records !== null && !kept.whole)
                provider.request(words);
        }
    }

    // The things told to the daemon, when they've changed since.
    function tell() {
        const daemon = reader.item as Daemon;
        if (!daemon || daemon.wire === null || !provider.launcher)
            return;
        const things = Meaning.things(provider.launcher.choices);
        const as = JSON.stringify(things);
        if (as === provider.toldAs)
            return;
        provider.toldAs = as;
        daemon.ask({ ask: "know", kind: "thing", things: things }, null);
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
            provider.pending = [];
            provider.searches = [];
        }

        function onChoicesChanged(): void {
            if (provider.launcher.open)
                Qt.callLater(provider.tell);
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
