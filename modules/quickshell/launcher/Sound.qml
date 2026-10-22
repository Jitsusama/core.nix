// Sound, a collection: the devices sound goes out of and comes in through,
// outputs then inputs, the one in use first in each, every glyph saying
// what kind of device it is and the footer its volume. Return uses one, or
// mutes or unmutes the one in use, and the right arrow offers the rest.
//
// It answers too: "volume" or "what's playing" says how loud the output in
// use is and through what, then the input, and each program playing; with
// a number, "volume 40", Return sets the output to it, and unmutes it,
// since a level asked for is one to be heard.
//
// Everything comes from PipeWire through Quickshell, so nothing is asked
// and nothing is spawned.
import qs
import Quickshell.Bluetooth
import Quickshell.Services.Pipewire
import QtQuick
import "../Fuzzy.js" as Fuzzy

Provider {
    id: provider

    what: "sound"

    collection: ({
        noun: "devices",
        title: "Sound",
        words: ["audio", "speakers", "headphones", "headset", "microphone", "mic", "output", "input"],
        glyph: glyph(0xf057e),
        ask: "which device?",
        empty: "No sound devices"
    })

    function glyph(code) {
        return String.fromCodePoint(code);
    }

    // A display plugged in besides the machine's own panel, which niri gives
    // workspaces of its own. PipeWire lists a sound card's display outputs
    // whether or not anything is plugged in, so they wait for one.
    readonly property bool displayed: Object.values(Niri.workspaces)
        .some(workspace => workspace.output && !workspace.output.startsWith("eDP"))

    function display(node) {
        return /hdmi|displayport/i.test(`${node.name} ${node.description}`);
    }

    // Hardware, not a program's stream, and not a filter that puts itself in
    // front of a device, as the speaker's tuning does.
    readonly property var nodes: Pipewire.nodes.values
        .filter(node => !!node.audio && !node.isStream
            && (node.type & PwNodeType.Audio) && (node.isSink || (node.type & PwNodeType.Source)))
    readonly property var devices: nodes
        .filter(node => node.properties["filter.smart"] !== "true" && (displayed || !display(node)))
    readonly property var outputs: devices.filter(node => node.isSink)
    readonly property var inputs: devices.filter(node => !node.isSink)

    // A node's volume is heard only once something binds it.
    PwObjectTracker {
        objects: provider.nodes
    }

    function inUse(node) {
        return node === (node.isSink ? Pipewire.defaultAudioSink : Pipewire.defaultAudioSource);
    }

    // Its own short name, rather than the sound card's long one before it.
    function named(node) {
        return node.nickname || node.description || node.name;
    }

    // What it is, from what PipeWire knows of it, and for a Bluetooth device
    // what BlueZ says it is, since PipeWire keeps that on the device rather
    // than its sound: over Bluetooth, through a display, worn, or the
    // machine's own.
    function kind(node) {
        // WirePlumber's stand-in for a headset's microphone says whose it is
        // only in its name.
        const address = node.properties["api.bluez5.address"]
            ?? (/^bluez_input\.([0-9A-F:]{17})$/.exec(node.name) || ["", ""])[1];
        const accessory = address
            ? Bluetooth.devices.values.find(device => device.address === address) : null;
        const said = [node.name, named(node), node.properties["device.form-factor"] ?? "",
            accessory ? accessory.icon : ""].join(" ").toLowerCase();
        const bluetooth = !!address || said.includes("bluez");
        if (!node.isSink)
            return said.includes("headset") ? 0xf02ce : 0xf036c;
        if (display(node))
            return 0xf0f5f;
        if (said.includes("head"))
            return bluetooth ? 0xf0970 : 0xf02cb;
        return bluetooth ? 0xf09a2 : 0xf04c3;
    }

    function use(node) {
        if (node.isSink)
            Pipewire.preferredDefaultAudioSink = node;
        else
            Pipewire.preferredDefaultAudioSource = node;
    }

    function mute(node) {
        node.audio.muted = !node.audio.muted;
    }

    function made(node) {
        const used = inUse(node);
        const muted = node.audio.muted;
        const note = `${node.isSink ? "output" : "input"} · ${Math.round(node.audio.volume * 100)}%`
            + (muted ? " · muted" : "");
        return member({
            key: `sound:${node.name}`,
            verb: !used ? "use" : muted ? "unmute" : "mute",
            name: named(node),
            glyph: glyph(muted ? (node.isSink ? 0xf0581 : 0xf036d) : kind(node)),
            hue: "machine",
            side: used ? "in use" : "",
            note: note,
            run: used ? () => provider.mute(node) : () => provider.use(node),
            verbs: () => provider.verbs(node)
        });
    }

    function verbs(node) {
        return (inUse(node) ? [] : [thing({
            key: "verb:sound-use",
            name: node.isSink ? "Play sound through it" : "Listen through it",
            glyph: glyph(kind(node)),
            hue: "action",
            run: () => provider.use(node)
        })]).concat([thing({
            key: "verb:sound-mute",
            name: node.audio.muted ? "Unmute it" : "Mute it",
            glyph: glyph(node.audio.muted ? 0xf057e : 0xf0581),
            hue: "action",
            run: () => provider.mute(node)
        })]);
    }

    // In use first, then by name, outputs before inputs.
    function ordered(nodes) {
        return nodes.slice().sort((a, b) => Number(inUse(b)) - Number(inUse(a))
            || named(a).localeCompare(named(b)));
    }

    // A device going away can be read once more before the lists drop it.
    readonly property var all: ordered(outputs).concat(ordered(inputs))
        .filter(node => !!node.audio).map(node => made(node))
    onAllChanged: changed()

    function search(typed) {
        const words = Fuzzy.words(typed);
        const need = Fuzzy.letters(words);
        const found = [];
        for (const row of all) {
            if ((row.prepared.mask & need) !== need)
                continue;
            const m = Fuzzy.match(row.prepared, words, false);
            if (m.score >= 0)
                found.push({ score: m.score, grade: m.grade, row: row });
        }
        let finest = 3;
        for (const f of found)
            finest = Math.min(finest, f.grade);
        const kept = found.filter(f => f.grade === finest).sort((a, b) => b.score - a.score);
        return { rows: kept.map(f => f.row), grade: finest };
    }

    function everything() {
        return all.slice();
    }

    function counted() {
        return `${outputs.length} out, ${inputs.length} in`;
    }

    examples: ["volume", "volume 40"]

    // How loud a device is, in words.
    function level(node) {
        return !node || !node.audio ? "none"
            : node.audio.muted ? "muted" : `${Math.round(node.audio.volume * 100)}%`;
    }

    function answer(typed) {
        const text = typed.trim().toLowerCase().replace(/\?$/, "");
        const asked = /^(?:volume|vol|sound)\s+(?:to\s+)?(\d{1,3})\s*%?$/.exec(text);
        if (asked)
            return set(Math.min(100, Number(asked[1])));
        if (!/^(volume|vol|how loud|loudness|what'?s playing|now playing|playing)$/.test(text))
            return null;
        return told();
    }

    function told() {
        const out = Pipewire.defaultAudioSink;
        const into = Pipewire.defaultAudioSource;
        const rows = [];
        // Each device by name, its level beside it, so a long name is what
        // the row is rather than what gets cut.
        if (out)
            rows.push(copyRow({
                name: named(out),
                side: `out \u00b7 ${level(out)}`,
                glyph: glyph(kind(out))
            }));
        if (into)
            rows.push(copyRow({
                name: named(into),
                side: `in \u00b7 ${level(into)}`,
                glyph: glyph(kind(into))
            }));
        // The track playing, as the music mark has it (Music.qml), then
        // each program with a stream open to play through, by the name it
        // gives; a playback stream carries PipeWire's sink flag, and may be
        // open and silent, so it says only that it's open.
        if (Music.playing)
            rows.push(copyRow({
                name: Music.artist === "" ? Music.title : `${Music.title}, ${Music.artist}`,
                side: "playing",
                glyph: Theme.glyph.playing
            }));
        const playing = Pipewire.nodes.values
            .filter(node => node.isStream && node.isSink && (node.type & PwNodeType.Audio))
            .map(node => node.properties["application.name"] ?? node.name)
            .filter((name, i, all) => name && all.indexOf(name) === i);
        for (const name of playing)
            rows.push(copyRow({ name: name, side: "has sound open", glyph: glyph(0xf057e) }));
        return {
            headline: out ? `${level(out)}, through ${named(out)}` : "nothing to play through",
            rows: rows,
            note: !Music.playing && playing.length === 0 ? "nothing playing" : "from PipeWire"
        };
    }

    function set(share) {
        const out = Pipewire.defaultAudioSink;
        if (!out || !out.audio)
            return null;
        return {
            headline: `set ${named(out)} to ${share}%`,
            verb: "set",
            run: () => {
                out.audio.volume = share / 100;
                out.audio.muted = false;
            },
            rows: told().rows,
            note: `now ${level(out)}`
        };
    }
}
