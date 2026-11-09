// What the launcher's Firmware makes of the machine daemon's firmware
// records, fwupd's own answers, in plain JavaScript so a check can run it
// in Qt's own engine, as the shell does: which devices are listed, in
// what order and by what names, which has an update offered and what
// holds one back, when each was last updated, and an update's release
// notes as words. Firmware.qml watches the daemon and draws the rows;
// nothing here asks anything.
.pragma library

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
        fwupd: host.fwupd ?? null };
}

// What holds back an update that's offered, by fwupd's name for it, or
// "": the first problem fwupd says, once nothing waits for the restart,
// since an update waiting there is no update held back.
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

// The last line fwupdmgr said of why an install didn't go in, as its
// error says it, or "" where it said nothing.
function reason(text) {
    const lines = String(text).split("\n").map(line => line.trim()).filter(line => line !== "");
    return lines.length === 0 ? "" : lines[lines.length - 1];
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
