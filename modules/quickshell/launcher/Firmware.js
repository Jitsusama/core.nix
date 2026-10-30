// What the launcher's Firmware makes of fwupd's answers, in plain
// JavaScript so a check can run it in Qt's own engine, as the shell does:
// which devices are listed, in what order and by what names, which has an
// update offered, when each was last updated, and an update's release
// notes as words. Firmware.qml asks fwupd and draws the rows; nothing here
// asks it anything.
.pragma library

// A device fwupd keeps from people, as it does each region of the BIOS's
// flash chip and the Secure Boot keys, which it updates only as a whole.
const HIDDEN = "updatable-hidden";
// An update that finishes only at a restart, as the BIOS's does.
const AT_RESTART = "needs-reboot";
// What fwupd names a vendor it doesn't know.
const UNKNOWN = "Unknown";
// A device fwupd installs to only on the charger, as the BIOS.
const ON_THE_CHARGER = "require-ac";
// What fwupd's history says of an update that went in.
const INSTALLED = 2;
// What it says of one that's in but finishes at a restart: pending, as a
// BIOS's capsule is until the restart flashes it, or needing a reboot.
const AT_THE_RESTART = [1, 4];
// The characters release notes escape, and what each is.
const ESCAPED = [["&lt;", "<"], ["&gt;", ">"], ["&quot;", "\""], ["&#39;", "'"],
    ["&apos;", "'"], ["&nbsp;", " "], ["&amp;", "&"]];

// Each device worth listing from fwupd's devices, offered updates and
// history, as each command's `--json` says them: a named device with a
// version that fwupd doesn't hide, those with an update first, then in
// fwupd's order, each { id, name, vendor, version, offered, notes,
// updated, restarts, waits, charger }: `waits` is the version installed
// to finish at the next restart, which is offered too until then, and
// `charger` whether fwupd installs to it only on the charger. Two devices of one name are numbered, as two USB-C
// controllers of one dock are, so neither reads as the other.
function devices(listed, offered, history) {
    const shown = (listed?.Devices ?? []).filter(device => (device.Name ?? "") !== ""
        && (device.Version ?? "") !== "" && !(device.Flags ?? []).includes(HIDDEN));
    const updates = new Map((offered?.Devices ?? [])
        .filter(device => (device.Releases ?? []).length > 0)
        .map(device => [device.DeviceId, device.Releases[0]]));
    const installed = new Map((history?.Devices ?? [])
        .filter(entry => entry.UpdateState === INSTALLED)
        .map(entry => [entry.DeviceId, Number(entry.Modified ?? entry.Created ?? 0)]));
    const waiting = new Map((history?.Devices ?? [])
        .filter(entry => AT_THE_RESTART.includes(entry.UpdateState))
        .map(entry => [entry.DeviceId, entry.Releases?.[0] ?? null])
        .filter(([, release]) => (release?.Version ?? "") !== ""));
    const made = shown.map(device => {
        const waits = waiting.get(device.DeviceId) ?? null;
        // Once the restart has flashed it, the version it waited for is
        // the device's own, and nothing waits.
        const pending = waits !== null && waits.Version !== device.Version ? waits : null;
        const release = updates.get(device.DeviceId) ?? pending;
        return {
            id: device.DeviceId,
            name: device.Name,
            // fwupd says "Unknown" of a vendor it doesn't know, as of the
            // Secure Boot keys', which is no vendor to show.
            vendor: device.Vendor === UNKNOWN ? "" : device.Vendor ?? "",
            version: device.Version,
            offered: release ? release.Version ?? null : null,
            notes: release ? words(release.Description ?? "") : "",
            updated: installed.get(device.DeviceId) || null,
            restarts: (device.Flags ?? []).includes(AT_RESTART),
            waits: pending ? pending.Version : null,
            charger: (device.Flags ?? []).includes(ON_THE_CHARGER)
        };
    });
    const ordered = made.filter(device => device.offered !== null)
        .concat(made.filter(device => device.offered === null));
    return numbered(ordered);
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
// or "1 update ready".
function standing(list) {
    const count = list.filter(device => device.offered !== null).length;
    if (count === 0) {
        return "up to date";
    }
    return count === 1 ? "1 update ready" : `${count} updates ready`;
}

// A device's version as its row says it, with what's offered after it:
// "1.12.0", "1.12.0, 1.13.0 ready", or "1.12.0, 1.13.0 at the restart" once
// it's installed to finish there.
function versionOf(device) {
    if (device.waits !== null) {
        return `${device.version}, ${device.waits} at the restart`;
    }
    return device.offered === null ? device.version : `${device.version}, ${device.offered} ready`;
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
