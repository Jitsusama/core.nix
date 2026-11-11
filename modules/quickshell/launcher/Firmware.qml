// The machine's firmware, answered in the field by Firmware's words
// ("firmware", "update" or "updates", as words.json lists them) and listed
// last in Machine, after Lag, since it's the computer and its devices
// (VKCF3Z, 2026-10-29). The headline says whether anything is offered,
// "up to date" or "1 update ready", the latter in the warning colour; the
// rows are each device fwupd knows, those with an update first, then in
// fwupd's order, each saying its version, holding its Version a step in,
// and showing in the drawer everything fwupd says of it and of what's
// offered, with the release's notes; then Security, fwupd's rating of
// the machine's own firmware.
//
// The rows and the headline are the machine daemon's (modules/machine's
// firmware_rows.rs), served ready (ReadyRows.qml), so nothing is gathered
// or worded here: the daemon asks fwupd once a minute while Firmware's
// rows can be seen (Seen.js) and at once each time fwupd says something
// changed, as when a device is unplugged from the charger. Installing,
// fixing a security attribute or putting its fix back, and restarting
// into the firmware's setup are the daemon's acts too, asked on the same
// socket, so the shell spawns nothing, and each row says how its act
// stands as the daemon serves it. A machine without fwupd names no
// Firmware.
//
// A device with an update offered holds Install first (VKCF3Z,
// 2026-10-30), which steps into the one row asking if Joel's sure, since
// firmware can't be taken back; its Return installs that device's update
// alone and never restarts, the restart being Joel's, through Power.
// fwupd's own polkit rules say whether a password is asked: a signed
// release from LVFS goes in without one. Nothing installs by itself. An
// act done with is dimmed and asks nothing more.
import qs
import Quickshell
import QtQuick
import "Seen.js" as Seen

Provider {
    id: provider

    // The words it answers whole, and by their start (Provider.qml).
    questions: Words.answers("firmware")

    // The fwupdmgr the daemon installs with, named here only to say
    // whether this machine has fwupd at all.
    readonly property string fwupdmgr: settings.fwupdmgr ?? ""
    // Whether Firmware's rows can be seen: its answer, or words typed that
    // ask it, a row of it, or Machine's place holding its row, so the
    // daemon asks fwupd only then and a change fwupd says is answered at
    // once.
    readonly property bool seen: Seen.seen(launcher, provider, ["firmware"],
        typed => fwupdmgr !== "" && Seen.asks(questions, plainly(typed)))

    // Firmware's rows and headline as the machine daemon serves them,
    // watched only while they can be seen, and held meanwhile.
    ReadyRows {
        id: ready

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
        place: "firmware"
        beside: ["head"]
        live: provider.seen
        paced: provider.launcher
        maker: provider
        // In the machine's ink, as Machine's are, where a row's own hue
        // doesn't warn or ask.
        make: (row, key) => ready.thing(row, key, { hue: row.hue || "machine" })
        drawn: provider.launcher?.open ?? false
    }
    // What the daemon serves, for a reader told of each change it hears.
    readonly property alias served: ready

    // Whether fwupd has answered the daemon: its headline served, saying
    // nothing where fwupd lists no device. A place first watched after
    // the daemon starts is answered before fwupd is asked.
    readonly property bool fwupdAnswered: ready.placings >= 0 && !!ready.held["head:firmware"]

    // Whether the daemon hasn't yet answered this watch whole, or fwupd
    // the daemon, so a terminal's show asks again for the figures rather
    // than printing what it held before (Provider.stillReading).
    stillReading: provider.seen && ready.link !== null && ready.state !== "not answering"
        && (!ready.link.answeredThisWatch || !fwupdAnswered)

    function answer(typed) {
        if (fwupdmgr === "" || asks(typed) === "")
            return null;
        // A daemon that isn't there says nothing, so Firmware is left out
        // rather than waiting on it.
        if (ready.state === "not answering")
            return null;
        if (!(ready.link?.answered ?? false) || !fwupdAnswered)
            return waitingAnswer();
        return told();
    }

    // While fwupd first answers: held in its place, its words to come, as
    // the processor's telemetry is (VKCF3Z, 2026-10-29).
    function waitingAnswer() {
        return {
            headline: "\u2026",
            parted: true,
            rows: [],
            asking: true,
            glyph: Words.glyph("firmware"),
            note: "",
            live: provider
        };
    }

    // Firmware's answer: its headline and its rows, a step in each holding
    // what's inside it; none on a machine where fwupd lists no device.
    function told() {
        const head = ready.held["head:firmware"]?.head ?? {};
        const rows = ready.rows();
        if (rows.length === 0)
            return null;
        return {
            headline: head.said ?? "",
            warns: head.hue === "warning",
            glyph: Words.glyph("firmware"),
            rows: rows,
            note: "",
            // Its answer and each row are Firmware's, so the daemon is
            // watched while one is stepped into (Seen.js), and an act's
            // end is heard there.
            live: provider
        };
    }

    // The rows' keys as last answered, so a device coming or going is
    // said where Machine holds Firmware's place; after that, what fwupd
    // says changed is answered at once while anyone can see it.
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
            } else if (provider.seen) {
                provider.answered();
            }
        }
    }
}
