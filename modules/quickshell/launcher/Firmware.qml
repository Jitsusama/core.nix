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
// What fwupd says comes from the machine daemon's firmware records
// (modules/machine), watched once Firmware is asked since the launcher
// opened, until it closes. The daemon asks fwupd once a minute while
// watched and at once each time fwupd says something changed, as when a
// device is unplugged from the charger, so nothing here asks fwupd
// anything but to install. A machine without fwupd names no Firmware.
//
// A device with an update offered holds Install first (VKCF3Z,
// 2026-10-30), which asks once, saying what it ends, since firmware can't
// be taken back; a second Return installs that device's update alone and
// never restarts, the restart being Joel's, through Power. fwupd's own
// polkit rules say whether a password is asked: a signed release from
// LVFS goes in without one. Nothing installs by itself. An update fwupd
// won't put in now says why in fwupd's own words, after a dot, and
// nothing is dimmed (VKCF3Z, 2026-11-08).
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "Firmware.js" as Fwupd
import "Machine.js" as Machine

Provider {
    id: provider

    // The words it answers whole, and by their start (Provider.qml).
    questions: Words.answers("firmware")

    // The device whose update is going in, or "": fwupd installs one at a
    // time, so a second Return can't start a second.
    property string installing: ""
    // How each install since the launcher opened ended, by device:
    // { outcome, reason }, outcome "installed" or "failed", so its row
    // stays to say so until Firmware is opened again, by the Lists rule.
    property var installs: ({})
    // Whether Firmware has been asked since the launcher opened, so the
    // daemon asks fwupd only then.
    property bool askedSinceOpen: false

    readonly property string fwupdmgr: settings.fwupdmgr ?? ""
    // Whether anyone can see what it says, so a change fwupd says is
    // answered at once.
    readonly property bool shown: launcher !== null && (launcher.open || launcher.cataloguing)

    function answer(typed) {
        if (fwupdmgr === "" || asks(typed) === "")
            return null;
        if (!askedSinceOpen)
            Qt.callLater(() => provider.askedSinceOpen = true);
        // A daemon that isn't there says nothing, so Firmware is left out
        // rather than waiting on it.
        if (daemon.state === "not answering")
            return null;
        if (!daemon.answered)
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
            live: provider,
            note: ""
        };
    }

    function told() {
        const records = Object.values(daemon.revision >= 0 ? daemon.held : {});
        const devices = Fwupd.devicesOf(records);
        if (devices.length === 0)
            return null;
        const host = Fwupd.hostOf(records);
        return {
            headline: Fwupd.standing(devices),
            warns: devices.some(device => device.offered !== null),
            glyph: Words.glyph("firmware"),
            rows: devices.map(device => provider.deviceRow(device))
                .concat(host?.security ? [provider.securityRow(host)] : []),
            live: provider,
            note: ""
        };
    }

    // What holds an offered update back, in fwupd's own words, or "".
    function heldSaid(device) {
        const held = Fwupd.heldBy(device);
        return held === "" ? "" : Words.of("firmware:held").said?.[held] ?? held;
    }

    function deviceRow(device) {
        const held = heldSaid(device);
        return thingRow({
            thing: true,
            key: `firmware:${device.id}`,
            name: device.name,
            // The version and what's offered are said once, on the side,
            // and what holds an update back after a dot (VKCF3Z,
            // 2026-11-08), going first where the row is too narrow.
            side: held === "" ? Fwupd.versionOf(device) : `${Fwupd.versionOf(device)} \u00b7 ${held}`,
            sideItems: held === "" ? [] : [{ said: Fwupd.versionOf(device), keep: 2 },
                { said: held, keep: 1 }],
            note: "",
            warns: device.offered !== null,
            glyph: Words.glyph("firmware"),
            drawer: () => provider.drawerOf(device)
        }, () => provider.inside(device));
    }

    // fwupd's rating of the machine's own firmware, "HSI:3!", as fwupd
    // writes it, without the version it appends; a "!" says something
    // running lowers it. The drawer says what it rated and which fwupd.
    function securityRow(host) {
        const id = "firmware:security";
        const level = String(host.security).replace(/\s*\(v[^)]*\)\s*$/, "");
        const said = Words.of(id).said ?? {};
        const facts = [];
        if (host.product)
            facts.push({ label: Words.name("firmware:product"), value: host.product });
        if (host.fwupd)
            facts.push({ label: Words.name("firmware:rated"), value: `fwupd ${host.fwupd}` });
        return thingRow({
            thing: true,
            key: `${id}`,
            name: Words.name(id),
            side: level,
            note: level.endsWith("!") ? said.running : said.level,
            glyph: Words.glyph("firmware"),
            drawer: () => ({ title: Words.name(id), facts: facts, body: "" })
        }, () => [copyRow({
            key: `${id}:level`,
            name: Words.name(id),
            side: level,
            copy: level,
            hue: "machine",
            glyph: ""
        })]);
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
    // it waits for the restart or has gone in. One fwupd won't put in now
    // says why in fwupd's words, undimmed, and Return does nothing; one
    // that didn't install says so and can be tried again.
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
        const held = heldSaid(device);
        if (held !== "")
            return noted({ run: () => {} }, held);
        const final = device.restarts ? notes["final-at-restart"] : notes.final;
        const sure = () => [thing({
            key: `${id}:sure:${device.id}`,
            // The step that asks Joel to be sure, so its Return does it (Launcher.lands).
            sure: true,
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

    // An install ended: fwupd says what changed, which the daemon reads
    // at once, so the row says whether it waits for the restart or has
    // gone in.
    function installEnded(code) {
        const ended = Object.assign({}, installs);
        ended[installing] = code === 0 ? { outcome: "installed", reason: "" }
            : { outcome: "failed", reason: Fwupd.reason(installerErr.text) };
        installs = ended;
        installing = "";
        answered();
    }

    // What the drawer shows of a device: what it is and where it stands,
    // what's offered and what holds it back, then its identifiers in the
    // Name face, and the notes of what's offered. A fact fwupd doesn't
    // know is left out rather than shown as coming.
    function drawerOf(device) {
        const facts = [];
        const add = (id, value, extra) => {
            if (value !== null && value !== undefined && value !== "")
                facts.push(Object.assign({ label: Words.name(id), value: String(value) }, extra ?? {}));
        };
        const about = device.about;
        const release = device.release;
        add("firmware:vendor", device.vendor);
        add("firmware:version", device.version);
        add("firmware:lowest", about.lowest);
        add("firmware:bootloader", about.bootloader);
        add("firmware:branch", about.branch);
        if (device.updated !== null)
            add("firmware:last-updated", dated(device.updated));
        add("firmware:offered", device.offered);
        add("firmware:urgency", release?.urgency);
        if (release?.size)
            add("firmware:size", Machine.bytes(release.size));
        if (release?.released)
            add("firmware:released", dated(release.released));
        if (release?.minutes)
            add("firmware:takes", String(Words.of("firmware:takes").said?.minutes ?? "")
                .replace("{count}", release.minutes));
        const held = Words.of("firmware:held").said ?? {};
        add("firmware:held", device.problems.map(problem => held[problem] ?? problem).join(" \u00b7 "));
        add("firmware:none", device.offered === null ? about.none : null);
        add("firmware:failed", device.failed);
        if (about.flashes !== null)
            add("firmware:flashes", about.flashes);
        if (about.battery !== null)
            add("firmware:battery", `${about.battery}%`);
        add("firmware:protocols", about.protocols.join(" \u00b7 "));
        add("firmware:plugin", about.plugin, { named: true });
        add("firmware:serial", about.serial, { named: true });
        const ended = installs[device.id] ?? null;
        const failed = ended?.outcome === "failed" && ended.reason !== ""
            ? `${Words.of("verb:firmware:install").said?.failed ?? ""}: ${ended.reason}` : "";
        return { title: device.name, facts: facts,
            body: [failed, release?.summary ?? "", device.notes].filter(part => part !== "").join("\n\n") };
    }

    // A day, as Quests says one: "29 Oct", or "29 Oct 2025" in another year.
    function dated(seconds) {
        const when = new Date(seconds * 1000);
        const thisYear = when.getFullYear() === new Date().getFullYear();
        return Qt.formatDate(when, thisYear ? "d MMM" : "d MMM yyyy");
    }

    // What fwupd says changed is answered at once while anyone can see it.
    Connections {
        target: daemon

        function onHeard(keys) {
            if (provider.shown)
                provider.answered();
        }
    }

    // Closed, nothing is kept, so the next opening asks afresh.
    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (!provider.launcher.open) {
                provider.installs = ({});
                provider.askedSinceOpen = false;
            }
        }
    }

    Daemon {
        id: daemon

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
        kind: ["firmware"]
        live: provider.launcher !== null && provider.launcher.open && provider.askedSinceOpen
        keptThroughGap: false
        answersWhole: true
    }

    Process {
        id: installer
        stderr: StdioCollector {
            id: installerErr
        }
        onExited: code => provider.installEnded(code)
    }
}
