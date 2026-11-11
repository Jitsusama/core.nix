// What the launcher's Firmware makes of the machine daemon's firmware
// records, fwupd's own answers, in plain JavaScript so a check can run it
// in Qt's own engine, as the shell does: which devices are listed, in
// what order and by what names, which has an update offered and what
// holds one back, when each was last updated, and an update's release
// notes as words. Firmware.qml watches the daemon and draws the rows;
// nothing here asks anything.
.pragma library
.import "Order.js" as Order

// A device fwupd keeps from people, as it does each region of the BIOS's
// flash chip and the Secure Boot keys, which it updates only as a whole.
const HIDDEN = "updatable-hidden";
// An update that finishes only at a restart, as the BIOS's does.
const AT_RESTART = "needs-reboot";
// What fwupd names a vendor it doesn't know.
const UNKNOWN = "Unknown";
// What fwupd's history says of an update that's in but finishes at a
// restart: pending, as a BIOS's capsule is until the restart flashes it,
// or needing a reboot.
const AT_THE_RESTART = ["pending", "needs-reboot"];
// What it says of one that didn't go in.
const FAILED = ["failed", "failed-transient"];
// The characters release notes escape, and what each is.
const ESCAPED = [["&lt;", "<"], ["&gt;", ">"], ["&quot;", "\""], ["&#39;", "'"],
    ["&apos;", "'"], ["&nbsp;", " "], ["&amp;", "&"]];

// Each device worth listing from the daemon's firmware records: a named
// device with a version that fwupd doesn't hide, those with an update
// first, then in fwupd's order, each { id, name, vendor, version,
// offered, notes, updated, restarts, waits, problems, failed, release,
// about }:
// - `waits` is the version installed to finish at the next restart,
//   which is offered too until then;
// - `problems` what fwupd says keeps it from updating now, by fwupd's
//   names, as "require-ac-power" while it's on battery;
// - `failed` fwupd's words for why the last install didn't go in;
// - `release` what's offered: { urgency, size, released, minutes,
//   summary }, or null;
// - `about` what fwupd says of the device itself: { lowest, bootloader,
//   branch, flashes, battery, protocols, plugin, serial, none }, `none`
//   being fwupd's words for why it offers nothing.
// Two devices of one name are numbered, as two USB-C controllers of one
// dock are, so neither reads as the other.
function devicesOf(records) {
    const shown = (records ?? []).filter(record => record.kind === "firmware"
        && record.key !== "firmware:host" && (record.name ?? "") !== ""
        && (record.version ?? "") !== "" && !(record.flags ?? []).includes(HIDDEN));
    const made = shown.map(deviceOf);
    const ordered = made.filter(device => device.offered !== null)
        .concat(made.filter(device => device.offered === null));
    return numbered(ordered);
}

// One device as Firmware shows it, from its record.
function deviceOf(record) {
    const flags = record.flags ?? [];
    const last = record.last_update ?? null;
    const waits = waitsFor(record, last);
    const update = record.update ?? null;
    return {
        id: record.id,
        name: record.name,
        // fwupd says "Unknown" of a vendor it doesn't know, as of the
        // Secure Boot keys', which is no vendor to show.
        vendor: record.vendor === UNKNOWN ? "" : record.vendor ?? "",
        version: record.version,
        offered: update?.version ?? waits,
        notes: words(update?.description ?? ""),
        updated: last?.updated_at_s ?? null,
        restarts: flags.includes(AT_RESTART),
        waits,
        problems: record.problems ?? [],
        failed: last !== null && FAILED.includes(last.state) ? last.error ?? "" : null,
        release: releaseOf(update),
        about: aboutOf(record)
    };
}

// The version an update installed waits for the restart to flash, or
// null. Once the restart has flashed it, that version is the device's
// own, and nothing waits.
function waitsFor(record, last) {
    if (last === null || !AT_THE_RESTART.includes(last.state)) {
        return null;
    }
    const version = last.version ?? "";
    return version !== "" && version !== record.version ? version : null;
}

// What's offered, as its drawer says it, or null with nothing offered.
function releaseOf(update) {
    if (update === null) {
        return null;
    }
    return {
        // fwupd's "unknown" is no urgency to show.
        urgency: update.urgency === "unknown" ? null : update.urgency ?? null,
        size: update.size_b ?? null,
        released: update.released_at_s ?? null,
        minutes: update.install_minutes ?? null,
        summary: update.summary ?? ""
    };
}

// What fwupd says of the device itself.
function aboutOf(record) {
    return {
        lowest: record.version_lowest ?? null,
        bootloader: record.version_bootloader ?? null,
        branch: record.branch ?? null,
        flashes: record.flashes_left ?? null,
        battery: record.battery_pct ?? null,
        protocols: record.protocols ?? [],
        plugin: record.plugin ?? null,
        serial: record.serial ?? null,
        none: record.no_update ?? null
    };
}

// The machine's own, as fwupd rates it: { security, product, fwupd }, or
// null before fwupd has answered.
function hostOf(records) {
    const host = (records ?? []).find(record => record.key === "firmware:host") ?? null;
    if (host === null) {
        return null;
    }
    return { security: host.security ?? null, product: host.product ?? null,
        fwupd: host.fwupd ?? null, attributes: attributesOf(host.attributes) };
}

// fwupd's highest level, whose protections are each of those levels' and
// none above.
const TOP_LEVEL = 5;
// The version fwupd appends to its rating, " (v2.1.9)".
const APPENDED = /\s*\(v[^)]*\)\s*$/;
// A rating's level and its "!".
const RATING = /^HSI:(\d)(!?)/;

// fwupd's rating as it writes it, "HSI:1!", without the version it
// appends, and what it says: { code, level, lowered }, `lowered` where
// something running lowers it, the "!"; or null where fwupd gave none.
function ratingOf(security) {
    if (!security) {
        return null;
    }
    const code = String(security).replace(APPENDED, "");
    const read = RATING.exec(code);
    return { code, level: read ? Number(read[1]) : null, lowered: read?.[2] === "!" };
}

// What fixes a failing attribute, by fwupd's flag for it: a setting in
// the firmware's setup, one in the system, or an update from its maker.
const FIXES = [["action-config-fw", "setup"], ["action-config-os", "system"],
    ["action-contact-oem", "maker"]];

// Each attribute the level is made of, one another obsoletes left out as
// fwupd's own count leaves it: { id, name, about, level, result,
// passesAs, passed, missing, runtimeIssue, canFix, canUndo, fix, setting,
// kernel, plugin }, named by fwupd's summary, `level` null for one about
// what's running, `runtimeIssue` where failing it lowers the level (the
// "!"), `canFix` and `canUndo` where fwupd can change it itself and put
// it back, `fix` a FIXES name or "", `setting` the firmware setting it
// reads and `kernel` the kernel's, each { current, target }, the
// setting's with its `id`.
function attributesOf(said) {
    return (said ?? [])
        .filter(one => !(one.flags ?? []).includes("obsoleted"))
        .map(one => {
            const flags = one.flags ?? [];
            return {
                id: one.id,
                name: one.summary || one.name || one.id,
                about: one.description ?? "",
                level: one.level ?? null,
                result: one.result ?? "unknown",
                passesAs: one.passes_as ?? null,
                passed: one.passed === true,
                missing: flags.includes("missing-data"),
                runtimeIssue: flags.includes("runtime-issue"),
                canFix: flags.includes("can-fix"),
                canUndo: flags.includes("can-undo"),
                fix: FIXES.find(([flag]) => flags.includes(flag))?.[1] ?? "",
                setting: one.bios_setting ?? null,
                kernel: one.kernel ?? null,
                plugin: one.plugin ?? ""
            };
        });
}

// What the next level needs, as fwupd counts it: each attribute at that
// level that hasn't passed, one it couldn't read included. None past the
// top level, or where fwupd gave no level.
function neededFor(attributes, level) {
    if (level === null || level >= TOP_LEVEL) {
        return [];
    }
    return attributes.filter(one => one.level === level + 1 && !one.passed);
}

// What's running that lowers the level, the "!": each attribute fwupd
// says is a runtime issue that hasn't passed, as fwupd adds its "!"
// (fu_security_attrs_calculate_hsi), not one only about updates or
// attestation.
function loweredBy(attributes) {
    return attributes.filter(one => one.runtimeIssue && !one.passed);
}

// Every attribute as a step in lists them: each failing one first, so
// what's wrong is at the top whatever its level, then each passing one,
// each part by level with what's running last, fwupd's order kept within
// a level (VKCF3Z, 2026-11-09).
function listed(attributes) {
    return Order.sorted(attributes, Order.then(Order.smaller(one => Number(one.passed)),
        Order.smaller(one => one.level ?? TOP_LEVEL + 1)));
}

// What blocks an update that's offered, by fwupd's name for it, or "":
// the first problem fwupd says, once nothing waits for the restart,
// since an update waiting there is blocked by nothing.
function heldBy(device) {
    if (device.offered === null || device.waits !== null) {
        return "";
    }
    return device.problems.find(problem => problem !== "update-pending") ?? "";
}

// The devices with each name that's shared numbered in the order listed,
// "USB-C Controller 1" and "USB-C Controller 2"; a name alone stays as it is.
function numbered(list) {
    const counts = {};
    for (const device of list) {
        counts[device.name] = (counts[device.name] ?? 0) + 1;
    }
    const seen = {};
    return list.map(device => {
        if (counts[device.name] < 2) {
            return device;
        }
        seen[device.name] = (seen[device.name] ?? 0) + 1;
        return Object.assign({}, device, { name: `${device.name} ${seen[device.name]}` });
    });
}

// How many updates are offered, as Firmware's row says it: "up to date",
// or "1 update ready", in Firmware's words, `said`.
function standing(list, said) {
    const count = list.filter(device => device.offered !== null).length;
    if (count === 0) {
        return said.current ?? "";
    }
    return count === 1 ? said.one ?? "" : String(said.many ?? "").replace("{count}", count);
}

// A device's version as its row says it, how it stands first and the
// version it runs after a dot (VKCF3Z, 2026-11-08): "1.12.0", "1.13.0
// ready, from 1.12.0", or "1.13.0 at the restart, from 1.12.0" once it's
// installed to finish there, each comma a dot as drawn, in Firmware's
// words, `said`.
function versionOf(device, said) {
    const from = (way, version) => String(way ?? "").replace("{version}", version)
        .replace("{was}", device.version);
    if (device.waits !== null) {
        return from(said.restart, device.waits);
    }
    return device.offered === null ? device.version : from(said.ready, device.offered);
}

// A sentence's closing full stop.
const CLOSING_STOP = /\.$/u;
// A word opened by two capitals, as BIOS and UEFI are, which keeps its
// first. Named as letters, since Qt's engine has no \p{Lu}: fwupd's lines
// are in English.
const CAPITALS = /^[A-Z]{2}/u;

// The last line fwupdmgr, fwupd or logind said of why what it was asked
// didn't go, or "" where it said nothing, as a note says it, after a
// dot: its first letter lowered, unless it opens a word in capitals,
// and no closing full stop (VKCF3Z, 2026-11-09).
function reason(text) {
    const lines = String(text).split("\n").map(line => line.trim()).filter(line => line !== "");
    if (lines.length === 0) {
        return "";
    }
    const last = lines[lines.length - 1].replace(CLOSING_STOP, "");
    return CAPITALS.test(last) ? last : last.charAt(0).toLowerCase() + last.slice(1);
}

// The least an install is waited for, in minutes: long enough for a
// polkit prompt Joel is answering and an update fwupd says nothing of
// the length of.
const INSTALL_PATIENCE = 30;
// A security attribute's fix or undo is one call fwupd answers at once
// (fu_util_security_fix in fwupd 2.1.9), so two minutes is a polkit
// prompt left unanswered, not a change still going in.
const CHANGE_PATIENCE = 2;

// How many minutes an install of this device's update is waited for
// before fwupdmgr is stopped: twice what fwupd says it takes, and never
// under INSTALL_PATIENCE. Stopping fwupdmgr stops only the waiting; the
// daemon carries on with what it was asked, and says what changed.
function installPatience(device) {
    const minutes = Number(device?.release?.minutes ?? 0);
    return Math.max(INSTALL_PATIENCE, Number.isFinite(minutes) ? 2 * minutes : 0);
}

// How many minutes a fix or undo is waited for.
function changePatience() {
    return CHANGE_PATIENCE;
}

// How a firmware act the machine daemon answered ran, as `ended` reads
// it: { code, stopped, err }, code 0 where it was done; the minutes it
// was waited for where the daemon stopped waiting at its patience, or 0;
// and the words fwupdmgr, fwupd or logind said of why not. No answer at
// all, from a daemon that went, is one that didn't go, saying nothing.
function acted(answer, minutes) {
    if (answer?.done === true) {
        return { code: 0, stopped: 0, err: "" };
    }
    const error = answer?.error ?? {};
    return { code: 1, stopped: error.code === "stopped" ? minutes : 0,
        err: String(error.message ?? "") };
}

// How a firmware act ended, as its row keeps it until Firmware is
// opened again: `done` ("installed" or "changed") where it ended by
// itself with nothing wrong; otherwise "failed", saying it stopped
// waiting after so many minutes where its patience ran out, else why in
// the last line of what was said. `run` is { code, stopped, err }, as
// `acted` gives it; `said` is its act's words, holding "stopped".
function ended(run, said, done) {
    if (run.code === 0 && run.stopped === 0) {
        return { outcome: done, reason: "" };
    }
    const why = run.stopped > 0
        ? String(said?.stopped ?? "").replace("{count}", run.stopped)
        : reason(run.err);
    return { outcome: "failed", reason: why };
}

// Release notes as words: fwupd gives them as a little HTML, paragraphs
// and lists, so each becomes a line of its own and nothing else is kept.
function words(html) {
    let text = String(html)
        .replace(/<\s*li[^>]*>/gi, "\n")
        .replace(/<\s*\/\s*(p|ul|ol|li)\s*>/gi, "\n")
        .replace(/<\s*(br|p|ul|ol)[^>]*>/gi, "\n")
        .replace(/<[^>]*>/g, "");
    for (const [escaped, plain] of ESCAPED) {
        text = text.split(escaped).join(plain);
    }
    return text.split("\n").map(line => line.replace(/\s+/g, " ").trim())
        .filter(line => line !== "").join("\n");
}
