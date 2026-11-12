// The battery and what feeds it, answered in the field: "battery", "power",
// "charger", "plugged in" or "energy". The headline is its charge and how
// long it has; the rows are the detail a status line leaves out, each a
// part named first: the Charger, holding each USB-C port and what it
// gives; the Charge, as a bar; what the machine is Drawing or Charging
// at, and the Voltage, each a spark of the last reads; the Energy it
// holds; its Health; the Cycles; the Charge type and Charge limit, a
// setting where Joel's account may write it, its choices a step in; its
// Temperature; and its Model, and when it was Made. A part has its own
// words too, "battery health" or "charge limit", which answer it alone.
// The power profile is the session's to show and change (Session.qml),
// and its row comes beside these. Every name and word is the vocabulary's
// (launcher/words.json, Words.qml).
//
// The rows and the headline are the machine daemon's (modules/machine's
// power_rows.rs), served ready (ReadyRows.qml), so nothing is gathered,
// worded or sampled here: the daemon reads the supplies once a second
// while Power's rows can be seen (Seen.js), and once as the shell starts
// and as UPower says a device came or went (Daemon.qml's refresh). A
// charge limit chosen is asked of the daemon on the rows' own socket, its
// row drawn at once as the choice says it becomes, until the daemon tells
// it so.
import qs
import Quickshell
import Quickshell.Services.UPower
import QtQuick
import "Seen.js" as Seen

Provider {
    id: provider

    examples: ["battery", "charger"]

    // The parts asked for alone, by their own words, as the part's name.
    readonly property var partsAsked: {
        const out = {};
        for (const id of Words.of("power").parts)
            for (const question of Words.answers(id))
                out[question] = Words.name(id);
        return out;
    }

    // The words it answers whole, and by their start (Provider.qml).
    // "charger" asks it whole, so the charger's old names do too.
    questions: Words.started("power").concat(Object.keys(partsAsked),
        Words.olds(Words.of("power:charger")))

    // Whether Power's rows can be seen: its answer, or words typed that
    // ask it, a row of it, its own place, or Machine's holding the Power
    // row that says how it stands, so the daemon reads the supplies only
    // then.
    readonly property bool seen: Seen.seen(launcher, provider, ["energy", "power"],
        typed => Seen.asks(questions, plainly(typed)))

    // Power's rows and headline as the machine daemon serves them, watched
    // only while they can be seen, and held meanwhile.
    ReadyRows {
        id: ready

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
        place: "power"
        beside: ["head"]
        live: provider.seen
        paced: provider.launcher
        maker: provider
        // A part holding more in the machine's ink, as Machine's are, and
        // a value in the voice's, where it doesn't warn.
        make: (row, key) => ready.thing(row, key, {
            hue: row.hue || ((row.acts ?? []).some(one => one.step === "rows") ? "machine"
                : "voice")
        })
        drawn: provider.launcher?.open ?? false
    }
    // What the daemon serves, for a reader told of each change it hears.
    readonly property alias served: ready

    // The supplies are read once though nothing shows them, as the shell
    // starts and as UPower says a device came or went, so Machine's Power
    // row and Now's Battery say how it stands while the launcher is closed.
    // UPower says so without being polled.
    readonly property int upowerDevices: UPower.devices.values.length
    onUpowerDevicesChanged: ready.link?.refresh()

    Connections {
        target: ready

        function onLinkChanged() {
            ready.link?.refresh();
        }
    }

    // Whether the daemon hasn't yet answered this watch whole, so a
    // terminal's show asks again for the figures rather than printing
    // what it held before (Provider.stillReading).
    stillReading: provider.seen && ready.link !== null && !ready.link.answeredThisWatch
        && ready.state !== "not answering"

    // The headline as served, or null on a machine with no battery.
    function head() {
        return ready.held["head:power"]?.head ?? null;
    }

    function answer(typed) {
        const question = asks(typed);
        if (question === "")
            return null;
        const whole = told();
        if (whole === null)
            return null;
        const part = partsAsked[question];
        if (part === undefined)
            return whole;
        const key = ready.keys.find(each => (ready.served(each)?.depth ?? 0) === 0
            && ready.served(each)?.name === part);
        if (key === undefined)
            return null;
        const row = ready.rowAt(key);
        return {
            headline: row.side,
            copy: row.copies?.() ?? row.side,
            graph: row.graph,
            glyph: Theme.glyph.charge,
            rows: ready.inside(key),
            note: row.note !== "" ? row.note : whole.note,
            live: provider
        };
    }

    // Power's answer: its headline and its own rows, a step in each holding
    // what's inside it; none until the daemon has served the battery's.
    function told() {
        const said = head();
        if (said === null)
            return null;
        return {
            headline: said.said ?? "",
            graph: said.graph,
            glyph: Theme.glyph.charge,
            rows: ready.rows(),
            note: "",
            live: provider
        };
    }

    // How Power stands, said on its row where Machine holds it (Launcher's
    // standsOf): the charge and how long it has, as the headline says it,
    // then the power profile in use, as the session says it (Session.qml),
    // so the row reads "36%" and "6 h 41 min to full" and then "Full
    // Performance", a dot between. A machine with no battery says its
    // profile alone.
    function stands() {
        const session = (launcher?.sources ?? []).find(one => one.settings.name === "session");
        return [head()?.said ?? "", session?.profile?.side ?? ""]
            .filter(part => part !== "").join(" \u00b7 ");
    }

    // A row of its own as it reads now, by its key, as Now's Battery
    // borrows the Energy's drawer (Piece.qml's drawnBy) and an act's row is
    // looked for as it becomes (Launcher.qml's nowRow); undefined before
    // the daemon has served it.
    function rowNow(key) {
        const served = `row:${key}`;
        return ready.served(served) ? ready.rowAt(served) : undefined;
    }

    // While its answer is shown, what the daemon changes is answered at
    // once, so its figures and sparks move.
    readonly property bool shown: launcher !== null && launcher.open && launcher.held.length === 0
        && launcher.answer?.live === provider

    // Whether its answer is stepped into: held as a step rather than
    // being the launcher's answer, as Machine.qml's are.
    readonly property bool steppedIn: launcher !== null && launcher.open
        && launcher.held.some(step => step.row?.live === provider)

    // The rows' keys as last answered, so a battery or a port coming or
    // going is said: the launcher keeps what's inside each place until its
    // source says it changed, and Machine's Power row says the charge too,
    // so the places held are said again with it. Unsaid, Battery's Health
    // was never found from the root (the net's goes-places run,
    // 2026-11-07). After that, a shown answer moves with each change, and
    // so does one stepped into, as Machine › Power › Battery is: it kept
    // the charge it was stepped into with, since a new charge moves no key
    // (Y2YBTS, 2026-11-12 [495f1a1b]). Power's figures change about once
    // a minute, so saying each again costs little, where Heat's change
    // every second.
    property string placedAnswered: ""

    Connections {
        target: ready

        function onHeard(changed, gone) {
            const placed = ready.keys.join("\n");
            const moved = placed !== provider.placedAnswered;
            provider.placedAnswered = placed;
            if (moved) {
                provider.answered();
                provider.launcher?.placesRead?.();
            } else if (provider.shown || provider.steppedIn) {
                provider.answered();
            }
        }
    }
}
