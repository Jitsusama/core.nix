// The machine's firmware, answered in the field by Firmware's words
// ("firmware", "update" or "updates", as words.json lists them) and listed
// last in Machine, after Lag, since it's the computer and its devices
// (VKCF3Z, 2026-10-29). The headline says whether anything is offered,
// "up to date" or "1 update ready", the latter in the warning colour; the
// rows are each device fwupd knows, those with an update first, then in
// fwupd's order, each saying its version, holding its Version a step in,
// and showing in the drawer its vendor, its version, when it was last
// updated and what's offered, with the release's notes.
//
// fwupd is asked as the answer is wanted and again while it stays wanted,
// a moment apart, never between: nothing is remembered from one opening
// of the launcher to the next, so it never shows a version that was. Its
// three answers, the devices, what LVFS offers them and what's been
// installed, are worked into rows by Firmware.js. A machine without fwupd
// has no fwupdmgr, and so no answer. Nothing here installs anything.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "Firmware.js" as Fwupd

Provider {
    id: provider

    // The words it answers whole, and by their start (Provider.qml).
    questions: Words.answers("firmware")

    // How old fwupd's answers may be and still be what's shown, since
    // they change only as a device comes or goes or an update goes in.
    readonly property int fresh: 10000

    // fwupd's three answers as each command's --json said them, while the
    // launcher is open, or null before they're all in.
    property var said: null
    property real saidAt: 0
    // Whether fwupd wouldn't answer, so the answer is left out rather
    // than waiting on it.
    property bool failed: false
    // How many of the three commands are still to answer.
    property int waiting: 0

    readonly property string fwupdmgr: settings.fwupdmgr ?? ""
    // Whether anyone can see what it says: fwupd is asked only then, so a
    // version is never remembered from an earlier opening.
    readonly property bool shown: launcher !== null && (launcher.open || launcher.cataloguing)

    function answer(typed) {
        if (fwupdmgr === "" || asks(typed) === "")
            return null;
        if (shown && (said === null || Date.now() - saidAt > fresh))
            look();
        if (said === null)
            return failed ? null : waitingAnswer();
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
            note: ""
        };
    }

    function told() {
        const devices = Fwupd.devices(said.listed, said.offered, said.history);
        if (devices.length === 0)
            return null;
        return {
            headline: Fwupd.standing(devices),
            warns: devices.some(device => device.offered !== null),
            glyph: Words.glyph("firmware"),
            rows: devices.map(device => provider.deviceRow(device)),
            note: ""
        };
    }

    function deviceRow(device) {
        return thingRow({
            thing: true,
            key: `firmware:${device.id}`,
            name: device.name,
            side: device.version,
            note: Fwupd.versionOf(device),
            warns: device.offered !== null,
            glyph: Words.glyph("firmware"),
            drawer: () => provider.drawerOf(device)
        }, () => provider.inside(device));
    }

    // What a device holds a step in: its Version, which Return copies.
    function inside(device) {
        return [copyRow({
            key: `firmware:${device.id}:version`,
            name: Words.name("firmware:version"),
            side: device.version,
            copy: device.version,
            hue: "machine",
            glyph: ""
        })];
    }

    // What the drawer shows of a device: what it is and where it stands,
    // then the notes of what's offered. A fact fwupd doesn't know is left
    // out rather than shown as coming.
    function drawerOf(device) {
        const facts = [];
        if (device.vendor !== "")
            facts.push({ label: Words.name("firmware:vendor"), value: device.vendor });
        facts.push({ label: Words.name("firmware:version"), value: device.version });
        if (device.updated !== null)
            facts.push({ label: Words.name("firmware:last-updated"), value: dated(device.updated) });
        if (device.offered !== null)
            facts.push({ label: Words.name("firmware:offered"), value: device.offered });
        return { title: device.name, facts: facts, body: device.notes };
    }

    // A day, as Quests says one: "29 Oct", or "29 Oct 2025" in another year.
    function dated(seconds) {
        const when = new Date(seconds * 1000);
        const thisYear = when.getFullYear() === new Date().getFullYear();
        return Qt.formatDate(when, thisYear ? "d MMM" : "d MMM yyyy");
    }

    // Asks fwupd all three at once, unless it's already being asked.
    function look() {
        if (waiting > 0)
            return;
        failed = false;
        waiting = 3;
        devicesAsked.running = true;
        offeredAsked.running = true;
        historyAsked.running = true;
    }

    // One command's answer in; once the last is, the answer is made again.
    function heard() {
        waiting -= 1;
        if (waiting > 0)
            return;
        // Answered after the launcher closed, it's left unsaid, or the next
        // opening would show it as though just asked.
        if (!shown)
            return;
        const listed = parsedOr(devicesOut.text, null);
        if (listed === null) {
            failed = true;
            said = null;
        } else {
            // With nothing offered, fwupd says so in words, not JSON.
            said = {
                listed: listed,
                offered: parsedOr(offeredOut.text, {}),
                history: parsedOr(historyOut.text, {})
            };
            saidAt = Date.now();
        }
        answered();
    }

    function parsedOr(text, otherwise) {
        try {
            return JSON.parse(text);
        } catch (error) {
            return otherwise;
        }
    }

    // Closed, nothing is kept, so the next opening asks afresh.
    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (!provider.launcher.open)
                provider.said = null;
        }
    }

    Process {
        id: devicesAsked
        command: [provider.fwupdmgr, "get-devices", "--json"]
        stdout: StdioCollector {
            id: devicesOut
            onStreamFinished: provider.heard()
        }
    }

    Process {
        id: offeredAsked
        command: [provider.fwupdmgr, "get-updates", "--json"]
        stdout: StdioCollector {
            id: offeredOut
            onStreamFinished: provider.heard()
        }
    }

    Process {
        id: historyAsked
        command: [provider.fwupdmgr, "get-history", "--json"]
        stdout: StdioCollector {
            id: historyOut
            onStreamFinished: provider.heard()
        }
    }
}
