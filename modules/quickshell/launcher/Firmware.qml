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
// has no fwupdmgr, and so no answer.
//
// A device with an update offered holds Install first (VKCF3Z,
// 2026-10-30), which asks once, saying what it ends, since firmware can't
// be taken back; a second Return installs that device's update alone and
// never restarts, the restart being Joel's, through Power. fwupd's own
// polkit rules say whether a password is asked: a signed release from
// LVFS goes in without one. Nothing installs by itself.
import qs
import Quickshell
import Quickshell.Io
import Quickshell.Services.UPower
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
    // The device whose update is going in, or "": fwupd installs one at a
    // time, so a second Return can't start a second.
    property string installing: ""
    // How each install since the launcher opened ended, by device:
    // { outcome, reason }, outcome "installed" or "failed", so its row
    // stays to say so until Firmware is opened again, by the Lists rule.
    property var installs: ({})

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

    // What a device holds a step in: Install while an update is offered,
    // or since one went in while shown, then its Version, which Return
    // copies.
    function inside(device) {
        const rows = device.offered !== null || installs[device.id] !== undefined
            || installing === device.id ? [installRow(device)] : [];
        return rows.concat([copyRow({
            key: `firmware:${device.id}:version`,
            name: Words.name("firmware:version"),
            side: device.version,
            copy: device.version,
            hue: "machine",
            glyph: ""
        })]);
    }

    // Install, as it stands: ready, asking once; installing, when Return
    // does nothing more; or dimmed in place once there's nothing to do, as
    // it waits for the restart, has gone in, or needs the charger while on
    // battery. One that didn't install says so and can be tried again.
    function installRow(device) {
        const id = "verb:firmware:install";
        const notes = Words.of(id).said ?? {};
        const ended = installs[device.id] ?? null;
        const version = device.offered ?? device.version;
        const row = {
            key: `${id}:${device.id}`,
            name: Words.name(id, "", { version: version }),
            verb: Words.verb(id),
            glyph: Words.glyph(id),
            hue: "action",
            stays: true
        };
        // The foot says the row's note, unless an install going in or one
        // that didn't says more.
        const noted = (fields, note) => thing(Object.assign(row, fields, {
            note: note,
            state: () => provider.standingOf(device, note)
        }));
        if (installing === device.id)
            return noted({ run: () => {} }, notes.installing);
        if (device.waits !== null)
            return noted({ gone: true }, notes.waits);
        if (device.offered === null || ended?.outcome === "installed")
            return noted({ gone: true }, notes.installed);
        if (device.charger && UPower.onBattery)
            return noted({ gone: true }, notes.charger);
        const final = device.restarts ? notes["final-at-restart"] : notes.final;
        const sure = () => [thing({
            key: `${id}:sure:${device.id}`,
            name: sureOf(device),
            note: final,
            verb: Words.verb(id),
            glyph: Words.glyph(id),
            hue: "alert",
            stays: true,
            state: () => provider.standingOf(device, final),
            run: () => provider.install(device)
        })];
        const failed = ended?.outcome === "failed";
        return noted({ verbs: sure, targets: sure }, failed ? notes.failed : "");
    }

    // What Install asks, as every act that can't be taken back does:
    // "Install 1.13.0 on System Firmware Now", noted that it can't be
    // undone, and that it finishes at a restart where it does.
    function sureOf(device) {
        return String(Words.of("verb:firmware:install").sure ?? "").replace("{version}", device.offered)
            .replace("{device}", device.name);
    }

    // What the foot says of an install: "installing" while it goes in,
    // "didn't install" after one that didn't, or else the row's own note,
    // as the warning that it can't be undone or "restart to finish".
    function standingOf(device, note) {
        const notes = Words.of("verb:firmware:install").said ?? {};
        if (installing === device.id)
            return notes.installing;
        return installs[device.id]?.outcome === "failed" ? notes.failed : note;
    }

    // Puts in the update offered to this device alone, without fwupd's
    // asking about a restart, unless another is going in.
    function install(device) {
        if (installing !== "" || device.offered === null)
            return;
        installing = device.id;
        installer.command = [fwupdmgr, "update", device.id, "--assume-yes", "--no-reboot-check"];
        installer.running = true;
        answered();
    }

    // An install ended: what fwupd says now is asked again while shown,
    // so the row says whether it waits for the restart or has gone in.
    function installEnded(code) {
        const ended = Object.assign({}, installs);
        ended[installing] = code === 0 ? { outcome: "installed", reason: "" }
            : { outcome: "failed", reason: Fwupd.reason(installerErr.text) };
        installs = ended;
        installing = "";
        if (shown) {
            said = null;
            look();
        }
        answered();
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
        const ended = installs[device.id] ?? null;
        const failed = ended?.outcome === "failed" && ended.reason !== ""
            ? `${Words.of("verb:firmware:install").said?.failed ?? ""}: ${ended.reason}` : "";
        return { title: device.name, facts: facts,
            body: [failed, device.notes].filter(part => part !== "").join("\n\n") };
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
            if (!provider.launcher.open) {
                provider.said = null;
                provider.installs = ({});
            }
        }
    }

    // Plugged in or out while shown, an Install that needs the charger
    // lights or dims in place.
    Connections {
        target: UPower

        function onOnBatteryChanged() {
            if (provider.shown && provider.said !== null)
                provider.answered();
        }
    }

    Process {
        id: installer
        stderr: StdioCollector {
            id: installerErr
        }
        onExited: code => provider.installEnded(code)
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
