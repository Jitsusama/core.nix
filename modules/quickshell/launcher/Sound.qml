// Sound, a collection: the devices sound goes out of and comes in through,
// outputs then inputs, the one in use first in each, every glyph saying
// what kind of device it is and the footer its volume. Return uses one, or
// mutes or unmutes the one in use, and the right arrow offers the rest.
// Everything comes from PipeWire through Quickshell, so nothing is asked
// and nothing is spawned.
import qs
import Quickshell
import Quickshell.Services.Pipewire
import QtQuick
import "../Fuzzy.js" as Fuzzy

Provider {
    id: provider

    collection: ({
        noun: "devices",
        title: "Sound",
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
        .filter(node => node.audio !== null && !node.isStream
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

    // What it is, from what PipeWire knows of it: over Bluetooth, through a
    // display, worn, or the machine's own.
    function kind(node) {
        const said = `${node.name} ${named(node)} ${node.properties["device.form-factor"] ?? ""}`
            .toLowerCase();
        const bluetooth = said.includes("bluez");
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

    readonly property var all: ordered(outputs).concat(ordered(inputs)).map(node => made(node))
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
}
