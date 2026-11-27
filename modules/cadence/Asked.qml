// Asked For in the launcher: what Joel asked the agents for in his own
// words that hasn't landed, one view of them all (WCJQSG, 2026-11-26,
// d903dead, loop 6). Typing "asked" or "promised" answers with its
// headline, "2 late of 9", and a row for each, late ones first; each
// says when it's due and how its probe stands, its note the quest that
// answers for it, and its drawer when it was asked, how often its date
// moved and the change it rides on. Return copies that change, or else
// his words, to say to its owner.
//
// The rows and the headline are cadence-spec's (./spec), served ready
// over its socket beside Spec's (ReadyRows.qml), so nothing is read,
// worded or ordered here (Joel, 2026-11-10: "the daemons need to be the
// data layer"). The daemon reads the asks only while Asked For can be
// seen (Seen.js), and at most once a minute.
import qs
import Quickshell
import QtQuick
import "Seen.js" as Seen
import "ReadyRows.js" as Ready

Provider {
    id: provider

    // The words it answers whole, and by their start (Provider.qml).
    questions: Words.answers("agents:asked-for")

    // Whether its rows can be seen: its answer, or words typed that ask
    // it, or a row of it, so the daemon reads the asks only then.
    readonly property bool seen: Seen.seen(launcher, provider, [],
        typed => Seen.asks(questions, plainly(typed)))

    ReadyRows {
        id: ready

        socket: `${Quickshell.env("XDG_RUNTIME_DIR") ?? ""}/daemons/cadence-spec.sock`
        place: "asked"
        beside: ["head"]
        live: provider.seen
        paced: provider.launcher
        maker: provider
        drawn: provider.launcher?.open ?? false
        make: (row, key) => ready.thing(row, key, { hue: row.hue || "agent" })
    }
    // What the daemon serves, for a reader told of each change it hears.
    readonly property alias served: ready

    // Whether the daemon has served its headline, saying nothing where
    // no ask is open.
    readonly property bool headed: ready.placings >= 0 && !!ready.held["head:asked"]

    // Whether the daemon hasn't yet answered this watch whole, so a
    // terminal's show asks again rather than printing what it held before
    // (Provider.stillReading).
    stillReading: provider.seen && ready.link !== null && ready.state !== "not answering"
        && (!ready.link.answeredThisWatch || !headed)

    function answer(typed) {
        if (asks(typed) === "" || ready.state === "not answering")
            return null;
        if (!(ready.link?.answered ?? false) || !headed)
            return {
                headline: "\u2026",
                parted: true,
                rows: [],
                asking: true,
                glyph: Words.glyph("agents:asked-for"),
                note: "",
                live: provider
            };
        const head = ready.held["head:asked"]?.head ?? {};
        return {
            headline: head.said ?? "",
            warns: head.glyph === "failed",
            glyph: Ready.glyphOf(head.glyph ?? "", Theme.glyph) || Words.glyph("agents:asked-for"),
            rows: ready.rows(),
            note: "",
            // Its answer and each row are Asked For's, so the daemon is
            // watched while one is stepped into (Seen.js).
            live: provider
        };
    }

    // A watch's rows coming or going are answered again while seen, and
    // once its whole answer is heard, as Firmware's are.
    Connections {
        target: ready

        function onHeard(changed, gone) {
            if (provider.seen)
                provider.answered();
        }
    }
    Connections {
        target: ready.link

        function onAnsweredChanged() {
            if (ready.link.answered && provider.seen)
                provider.answered();
        }
    }
}
