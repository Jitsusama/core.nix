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
// (modules/machine), watched while Firmware's rows can be seen (Seen.js).
// Every word is the vocabulary's (launcher/words.json). The daemon asks fwupd once a minute while
// watched and at once each time fwupd says something changed, as when a
// device is unplugged from the charger, so nothing here asks fwupd
// anything. Installing, fixing a security attribute or putting its fix
// back, and restarting into the firmware's setup are the daemon's acts
// too, asked on the same socket, so the shell spawns nothing. A machine
// without fwupd names no Firmware.
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
import QtQuick
import "Firmware.js" as Fwupd
import "Machine.js" as Machine
import "Seen.js" as Seen

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
    // The change fwupd is making, "fix:" or "undo:" and the attribute,
    // or "", and how each change since the launcher opened ended, keyed
    // the same way, so a fix's end never reads as its undo's; a restart
    // into firmware setup that didn't go is kept there too, as "setup:".
    property string changing: ""
    property var changes: ({})
    // The attribute whose restart into firmware setup is being asked of
    // logind, or "".
    property string restarting: ""

    // The fwupdmgr the daemon installs with, named here only to say
    // whether this machine has fwupd at all.
    readonly property string fwupdmgr: settings.fwupdmgr ?? ""
    // Whether Firmware's rows can be seen: its answer, or words typed that
    // ask it, a row of it, or Machine's place holding its row, so the
    // daemon asks fwupd only then and a change fwupd says is answered at
    // once.
    readonly property bool seen: Seen.seen(launcher, provider, ["firmware"],
        typed => fwupdmgr !== "" && Seen.asks(questions, plainly(typed)))

    // Whether the daemon hasn't yet answered this watch whole, so a
    // terminal's show asks again for the figures rather than printing
    // what it held before (Provider.stillReading).
    stillReading: daemon.live && !daemon.answeredThisWatch && daemon.state !== "not answering"

    // Firmware's own words, for Firmware.js.
    readonly property var said: Words.of("firmware").said ?? {}

    function answer(typed) {
        if (fwupdmgr === "" || asks(typed) === "")
            return null;
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
            note: "",
            live: provider
        };
    }

    function told() {
        const records = Object.values(daemon.revision >= 0 ? daemon.held : {});
        const devices = Fwupd.devicesOf(records);
        if (devices.length === 0)
            return null;
        const host = Fwupd.hostOf(records);
        return {
            headline: Fwupd.standing(devices, provider.said),
            warns: devices.some(device => device.offered !== null),
            glyph: Words.glyph("firmware"),
            rows: devices.map(device => provider.deviceRow(device))
                .concat(host?.security ? [provider.securityRow(host)] : []),
            note: "",
            // Its answer and each row are Firmware's, so the daemon is
            // watched while one is stepped into (Seen.js), and an install's
            // history is heard there.
            live: provider
        };
    }

    // What holds an offered update back, in fwupd's own words, or "".
    function heldSaid(device) {
        const held = Fwupd.heldBy(device);
        return held === "" ? "" : Words.of("firmware:held").said?.[held] ?? held;
    }

    function deviceRow(device) {
        const held = heldSaid(device);
        const version = Fwupd.versionOf(device, provider.said);
        return thingRow({
            thing: true,
            key: `firmware:${device.id}`,
            name: device.name,
            // The version and what's offered are said once, on the side,
            // and what holds an update back after a dot (VKCF3Z,
            // 2026-11-08), going first where the row is too narrow.
            side: held === "" ? version : `${version} \u00b7 ${held}`,
            sideItems: held === "" ? [] : [{ said: version, keep: 2 }, { said: held, keep: 1 }],
            note: "",
            warns: device.offered !== null,
            glyph: Words.glyph("firmware"),
            drawer: () => provider.drawerOf(device),
            live: provider
        }, () => provider.inside(device));
    }

    // fwupd's rating of the machine's own firmware, "HSI:1!", as fwupd
    // writes it, a name from the world, without the version it appends;
    // its note says the level in words, and that what's running lowers it
    // where there's a "!" (VKCF3Z [55ed47a4], 2026-11-09, which rules
    // this row's note though it steps in). The drawer says what the next
    // level needs and what's running that lowers it, in our words, then
    // what it rated and which fwupd, its words saying the level once as a
    // sentence, under an eyebrow saying where and what kind it is. A step
    // in lists every attribute the level is made of (attributeRow), each
    // failing one first, then the rating, which Return copies, wearing
    // Security Level's mark so it doesn't read as a heading (VKCF3Z,
    // 2026-11-09).
    function securityRow(host) {
        const id = "firmware:security";
        const rating = Fwupd.ratingOf(host.security);
        const said = Words.of(id).said ?? {};
        const meaning = rating.level === null ? "" : said[String(rating.level)] ?? "";
        const attributes = host.attributes;
        const results = Words.of("firmware:result").said ?? {};
        const failing = list => list.map(one => `${one.name} ${resultOf(one, results)}`)
            .join(" \u00b7 ");
        const needed = Fwupd.neededFor(attributes, rating.level);
        const lowered = rating.lowered ? Fwupd.loweredBy(attributes) : [];
        const facts = [];
        if (needed.length > 0)
            facts.push({ label: Words.name("firmware:needed", "", { level: rating.level + 1 }),
                value: failing(needed) });
        if (lowered.length > 0)
            facts.push({ label: Words.name("firmware:lowered"), value: failing(lowered) });
        if (host.product)
            facts.push({ label: Words.name("firmware:product"), value: host.product });
        if (host.fwupd)
            facts.push({ label: Words.name("firmware:rated"), value: `fwupd ${host.fwupd}` });
        const form = rating.lowered ? "sentence-lowered" : "sentence";
        const sentence = meaning === "" ? "" : String(said[form] ?? "")
            .replace("{level}", rating.level).replace("{meaning}", meaning);
        return thingRow({
            thing: true,
            key: `${id}`,
            name: Words.name(id),
            side: rating.code,
            note: [meaning, rating.lowered ? said.lowered : ""].filter(Boolean).join(" \u00b7 "),
            glyph: Words.glyph("firmware"),
            live: provider,
            drawer: () => ({ eyebrow: said.eyebrow ?? "", title: Words.name(id), facts: facts,
                body: sentence })
        }, () => Fwupd.listed(attributes).map(attributeRow).concat([copyRow({
            key: `${id}:level`,
            name: Words.name(id),
            side: rating.code,
            copy: rating.code,
            hue: "machine",
            glyph: Words.glyph("firmware"),
            // A figure wears no mark unless it's marked (Choice.qml).
            marked: true
        })]));
    }

    // What fwupd found of an attribute in our words, "not enabled", and
    // that it couldn't read what it needed where it says so.
    function resultOf(attribute, results) {
        const found = results[attribute.result] ?? attribute.result;
        return attribute.missing ? `${found} \u00b7 ${results.missing}` : found;
    }

    // One attribute of the security level, as fwupd names it: its side
    // its group, the level it counts at or what's running, muted on every
    // row, then what was found, warning where it fails; its mark the theme's for a pass, or
    // its warning mark in warning ink for a shortfall (VKCF3Z,
    // 2026-11-09). Its drawer, under an eyebrow saying its group and that
    // it's a check, "level 2 \u00b7 check", says what passing would find, what fixes it,
    // the firmware setting or the kernel's it reads, what that's set to
    // and what passing needs, and fwupd's description in its words, the
    // setting, plugin and attribute laid out as identifiers, at their
    // dots. A step in holds what takes Joel to its fix (actsOn).
    function attributeRow(attribute) {
        const results = Words.of("firmware:result").said ?? {};
        const groups = Words.of("firmware:group").said ?? {};
        const fixes = Words.of("firmware:fixed-in").said ?? {};
        const facts = [];
        const fact = (id, value, identifies) => {
            if (value)
                facts.push({ label: Words.name(id), value: value, identifies: !!identifies });
        };
        if (!attribute.passed && attribute.passesAs)
            fact("firmware:passes-as", results[attribute.passesAs] ?? attribute.passesAs);
        if (!attribute.passed)
            fact("firmware:fixed-in", fixes[attribute.fix] ?? "");
        fact("firmware:setting", attribute.setting?.id ?? "", true);
        fact("firmware:set-to", attribute.setting?.current ?? "");
        if (!attribute.passed)
            fact("firmware:should-be", attribute.setting?.target ?? "");
        fact("firmware:kernel-set-to", attribute.kernel?.current ?? "");
        if (!attribute.passed)
            fact("firmware:kernel-should-be", attribute.kernel?.target ?? "");
        fact("firmware:plugin", attribute.plugin, true);
        fact("firmware:attribute", attribute.id, true);
        const group = attribute.level === null ? groups.running
            : String(groups.level ?? "").replace("{level}", attribute.level);
        const result = resultOf(attribute, results);
        // A thing, so its drawer shows as it's selected, as a port's does.
        return thingRow({
            thing: true,
            key: `firmware:security:${attribute.id}`,
            name: attribute.name,
            side: `${group} \u00b7 ${result}`,
            sideItems: [{ said: group, keep: 1, hue: "muted" }, { said: result, keep: 2 }],
            note: "",
            warns: !attribute.passed,
            hue: attribute.passed ? "machine" : "warning",
            glyph: attribute.passed ? Theme.glyph.complete : Theme.glyph.caution,
            live: provider,
            drawer: () => ({ eyebrow: [group, groups.kind].filter(Boolean).join(" \u00b7 "),
                title: attribute.name, facts: facts,
                body: attribute.about })
        }, () => provider.actsOn(attribute));
    }

    // What an attribute holds a step in: Secure It where it fails and fwupd
    // can change it itself, Undo It where it passes and fwupd can put
    // it back, Reboot Into Firmware Setup where it fails and a firmware
    // setting fixes it, then fwupd's id for it, which Return copies and
    // its documentation goes by.
    function actsOn(attribute) {
        const acts = [];
        if (!attribute.passed && attribute.canFix)
            acts.push(changeRow(attribute, "fix"));
        if (attribute.passed && attribute.canUndo)
            acts.push(changeRow(attribute, "undo"));
        if (!attribute.passed && attribute.fix === "setup")
            acts.push(setupRow(attribute));
        return acts.concat([copyRow({
            key: `firmware:security:${attribute.id}:id`,
            name: Words.name("firmware:attribute"),
            side: attribute.id,
            copy: attribute.id,
            hue: "machine",
            glyph: ""
        })]);
    }

    // fwupd fixing an attribute itself, or putting a fix back, as
    // fwupdmgr's security-fix and security-undo do: asked once, saying
    // what it changes, then done with Joel's second Return, fwupd's polkit
    // rules saying whether a password is asked. The row stays to say how
    // it ended.
    function changeRow(attribute, how) {
        const id = `verb:firmware:${how}`;
        const notes = Words.of(id).said ?? {};
        const keyed = `${how}:${attribute.id}`;
        const ended = changes[keyed] ?? null;
        const row = {
            key: `${id}:${attribute.id}`,
            name: Words.name(id),
            verb: Words.verb(id),
            glyph: Words.glyph(id),
            hue: "action",
            stays: true
        };
        const standing = note => changing === keyed ? notes.changing : note;
        const noted = (fields, note) => thing(Object.assign(row, fields, {
            note: note,
            state: () => standing(note)
        }));
        if (changing === keyed)
            return noted({ run: () => {} }, notes.changing);
        if (ended?.outcome === "changed")
            return noted({ gone: true }, notes.changed);
        const sure = () => [thing({
            key: `${id}:sure:${attribute.id}`,
            // The step that asks Joel to be sure, so its Return does it (Launcher.lands).
            sure: true,
            name: String(Words.of(id).sure ?? "").replace("{attribute}", attribute.name),
            note: notes.final,
            verb: Words.verb(id),
            glyph: Words.glyph(id),
            hue: "alert",
            stays: true,
            state: () => standing(notes.final),
            run: () => provider.change(attribute, how)
        })];
        const failed = ended?.outcome === "failed";
        return noted({ verbs: sure, targets: sure },
            failed ? [notes.failed, ended.reason].filter(Boolean).join(" \u00b7 ") : notes.what);
    }

    // Restarting into the firmware's own setup, where the setting that
    // fixes an attribute is changed, saying which and to what where fwupd
    // says: asked once, since it closes every window, then done with
    // Joel's second Return, as Power's Reboot is. logind refuses it where
    // the firmware can't be asked to open its setup, and the row then
    // says logind's words for why.
    function setupRow(attribute) {
        const id = "verb:firmware:setup";
        const notes = Words.of(id).said ?? {};
        const setting = attribute.setting;
        const ended = changes[`setup:${attribute.id}`] ?? null;
        const asked = setting?.target ? String(notes.set ?? "").replace("{setting}", setting.id)
            .replace("{target}", setting.target) : notes.find;
        const note = ended?.outcome === "failed"
            ? [notes.failed, ended.reason].filter(Boolean).join(" \u00b7 ") : asked;
        const sure = () => [thing({
            key: `${id}:sure:${attribute.id}`,
            // The step that asks Joel to be sure, so its Return does it (Launcher.lands).
            sure: true,
            name: Words.of(id).sure ?? "",
            note: notes.final,
            verb: Words.verb(id),
            glyph: Words.glyph(id),
            hue: "alert",
            run: () => provider.restart(attribute)
        })];
        return thing({
            key: `${id}:${attribute.id}`,
            name: Words.name(id),
            note: note,
            verb: Words.verb(id),
            glyph: Words.glyph(id),
            hue: "action",
            stays: true,
            verbs: sure,
            targets: sure
        });
    }

    // Has logind restart into the firmware's setup, unless it's already
    // asked to.
    function restart(attribute) {
        if (restarting !== "")
            return;
        restarting = attribute.id;
        const minutes = Fwupd.changePatience();
        daemon.ask({ ask: "act", act: "firmware-setup", patience_s: minutes * 60 },
            answer => provider.restartEnded(Fwupd.acted(answer, minutes)));
    }

    // logind refused the restart, or took it, in which case nothing here
    // is left to see it.
    function restartEnded(run) {
        if (run.code !== 0) {
            const ended = Object.assign({}, changes);
            ended[`setup:${restarting}`] = Fwupd.ended(run,
                Words.of("verb:firmware:setup").said, "restarted");
            changes = ended;
        }
        restarting = "";
        answered();
    }

    // Has fwupd fix an attribute itself, or put a fix back, unless a
    // change is going in, through the daemon, which asks fwupd over the
    // system bus as fwupdmgr's security-fix and security-undo do. Neither
    // restarts; the restart stays Joel's.
    function change(attribute, how) {
        if (changing !== "")
            return;
        changing = `${how}:${attribute.id}`;
        const minutes = Fwupd.changePatience();
        daemon.ask({ ask: "act", act: `firmware-${how}`, attribute: attribute.id,
            patience_s: minutes * 60 },
            answer => provider.changeEnded(Fwupd.acted(answer, minutes)));
        answered();
    }

    // A change ended: fwupd says what changed, which the daemon reads at
    // once, so the attribute says what it found now. One stopped for
    // taking too long says how long it was waited for.
    function changeEnded(run) {
        const how = changing.slice(0, changing.indexOf(":"));
        const ended = Object.assign({}, changes);
        ended[changing] = Fwupd.ended(run, Words.of(`verb:firmware:${how}`).said, "changed");
        changes = ended;
        changing = "";
        answered();
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
        return noted({ verbs: sure, targets: sure },
            failed ? [notes.failed, ended.reason].filter(Boolean).join(" \u00b7 ") : "");
    }

    // What Install asks, as every act that can't be taken back does:
    // "Install 1.13.0 on System Firmware Now", noted that it can't be
    // undone, and that it finishes at a restart where it does.
    function sureOf(device) {
        return String(Words.of("verb:firmware:install").sure ?? "")
            .replace("{version}", device.offered).replace("{device}", device.name);
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
        const minutes = Fwupd.installPatience(device);
        daemon.ask({ ask: "act", act: "firmware-install", device: device.id,
            patience_s: minutes * 60 },
            answer => provider.installEnded(Fwupd.acted(answer, minutes)));
        answered();
    }

    // An install ended: fwupd says what changed, which the daemon reads
    // at once, so the row says whether it waits for the restart or has
    // gone in. One stopped for taking too long says how long it was
    // waited for, and the device says what fwupd went on to do.
    function installEnded(run) {
        const ended = Object.assign({}, installs);
        ended[installing] = Fwupd.ended(run, Words.of("verb:firmware:install").said,
            "installed");
        installs = ended;
        installing = "";
        answered();
    }

    // What the drawer shows of a device: who made it in its eyebrow, "from
    // Dell \u00b7 firmware"; what it is and where it stands, what's offered
    // and what holds it back, leaving the versions to the row's side
    // (VKCF3Z, 2026-11-08); then its identifiers in the Name face, and the
    // notes of what's offered. A fact fwupd doesn't know is left out
    // rather than shown as coming.
    function drawerOf(device) {
        const facts = [];
        const add = (id, value, extra) => {
            if (value !== null && value !== undefined && value !== "")
                facts.push(Object.assign({ label: Words.name(id), value: String(value) },
                    extra ?? {}));
        };
        const about = device.about;
        const release = device.release;
        add("firmware:lowest", about.lowest);
        add("firmware:bootloader", about.bootloader);
        add("firmware:branch", about.branch);
        if (device.updated !== null)
            add("firmware:last-updated", dated(device.updated));
        add("firmware:urgency", release?.urgency);
        if (release?.size)
            add("firmware:size", Machine.bytes(release.size));
        if (release?.released)
            add("firmware:released", dated(release.released));
        if (release?.minutes)
            add("firmware:takes", String(Words.of("firmware:takes").said?.minutes ?? "")
                .replace("{count}", release.minutes));
        const held = Words.of("firmware:held").said ?? {};
        add("firmware:held", device.problems.map(problem => held[problem] ?? problem)
            .join(" \u00b7 "));
        add("firmware:none", device.offered === null ? about.none : null);
        add("firmware:failed", device.failed);
        if (about.flashes !== null)
            add("firmware:flashes", about.flashes);
        if (about.battery !== null)
            add("firmware:battery", `${about.battery}%`);
        add("firmware:protocols", about.protocols.join(" \u00b7 "));
        add("firmware:plugin", about.plugin, { identifies: true });
        add("firmware:serial", about.serial, { identifies: true });
        const ended = installs[device.id] ?? null;
        const failed = ended?.outcome === "failed" && ended.reason !== ""
            ? `${Words.of("verb:firmware:install").said?.failed ?? ""} \u00b7 ${ended.reason}` : "";
        const eyebrows = Words.of("firmware:vendor").said ?? {};
        const eyebrow = device.vendor
            ? String(eyebrows.from ?? "").replace("{vendor}", device.vendor) : eyebrows.alone ?? "";
        const body = [failed, release?.summary ?? "", device.notes].filter(part => part !== "");
        return { eyebrow: eyebrow, eyebrowNames: !!device.vendor, title: device.name, facts: facts,
            body: body.join("\n\n") };
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
            if (provider.seen)
                provider.answered();
        }
    }

    // Closed, nothing is kept, so the next opening asks afresh.
    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (!provider.launcher.open) {
                provider.installs = ({});
                provider.changes = ({});
            }
        }
    }

    Daemon {
        id: daemon

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
        kind: ["firmware"]
        live: provider.seen
        paced: provider.launcher
        keptThroughGap: false
        answersWhole: true
    }
}
