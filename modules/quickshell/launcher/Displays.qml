// Displays, a collection: the displays niri drives, the built-in one first,
// each with its size, refresh rate and scale in the footer. Return moves
// focus to one, or turns on one that's off; the right arrow offers its
// scale, refresh rate, size and rotation, and turning it off while another
// is on. niri forgets these choices when its configuration is next read,
// so they last the session, and what lasts belongs in the configuration.
// Everything goes over niri's socket, asked again whenever the launcher
// opens or the workspaces change, as they do when a display comes or goes,
// since niri sends no event for a display itself. What a display's EDID
// says of itself, who made it and when, its size, native mode, refresh
// range, colour depth, colours and HDR, desk-facts writes to a file
// (modules/desk-facts), and the right arrow lists after the choices,
// Return copying one.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "../Fuzzy.js" as Fuzzy

Provider {
    id: provider

    what: "display"

    collection: Words.collection("displays")

    function glyph(code) {
        return String.fromCodePoint(code);
    }

    // What niri says of each display, by its connector's name.
    property var outputs: ({})

    function refresh() {
        Niri.request("Outputs", answer => provider.outputs = answer.Outputs);
    }

    Component.onCompleted: refresh()

    Connections {
        target: Niri

        function onWorkspacesChanged() {
            provider.refresh();
        }
    }

    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (provider.launcher.open)
                provider.refresh();
        }
    }

    // Tells niri what to change, then asks again how things stand.
    function change(output, action) {
        Niri.request({ Output: { output: output.name, action: action } }, () => provider.refresh());
    }

    function builtIn(output) {
        return output.name.startsWith("eDP");
    }

    // Its maker and model, leaving out a model that's only a number.
    function named(output) {
        if (builtIn(output))
            return "Built-in Display";
        const model = /^0x[0-9a-f]+$/i.test(output.model) ? "" : output.model;
        return `${output.make} ${model}`.trim() || output.name;
    }

    function hertz(mode) {
        return Math.round(mode.refresh_rate / 1000);
    }

    function mode(output) {
        return output.current_mode === null ? null : output.modes[output.current_mode];
    }

    function described(output) {
        const now = mode(output);
        if (now === null || output.logical === null)
            return "off";
        const scale = Math.round(output.logical.scale * 100) / 100;
        return `${now.width} \u00d7 ${now.height} \u00b7 ${hertz(now)} Hz \u00b7 scale ${scale}`;
    }

    readonly property var shown: Object.values(outputs).sort((a, b) =>
        Number(builtIn(b)) - Number(builtIn(a)) || a.name.localeCompare(b.name))
    readonly property var lit: shown.filter(output => output.logical !== null)

    readonly property string focusedOutput: {
        Niri.revision;
        const workspace = Niri.workspace();
        return workspace ? workspace.output : "";
    }

    function made(output) {
        const on = output.logical !== null;
        const focused = lit.length > 1 && output.name === focusedOutput;
        return member({
            key: `display:${output.name}`,
            verb: on ? "focus" : "turn on",
            name: named(output),
            keywords: output.name,
            glyph: glyph(builtIn(output) ? 0xf0322 : 0xf0379),
            hue: "machine",
            side: !on ? "off" : focused ? "focused" : "",
            note: described(output),
            run: on ? () => Niri.act({ FocusMonitor: { output: output.name } })
                : () => provider.change(output, "On"),
            verbs: () => provider.verbs(output)
        });
    }

    readonly property var all: shown.map(output => made(output))
    onAllChanged: changed()

    function choice(key, name, glyphCode, inUse, run) {
        return thing({
            key: `verb:display-${key}`,
            name: name,
            glyph: glyph(glyphCode),
            hue: "action",
            side: inUse ? "in use" : "",
            run: run
        });
    }

    // What desk-facts knows of each display, by its connector's name; one
    // it says nothing of, or no file at all, has no facts.
    property var facts: ({})

    FileView {
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/desk/displays.json`
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        onLoaded: provider.facts = provider.parsed(text())
        onLoadFailed: provider.facts = ({})
    }

    function factRows(output) {
        return (facts[output.name] ?? []).map(fact => copyRow({
            key: `fact:displays:${output.name}:${fact.name}`,
            name: fact.value,
            side: fact.name,
            glyph: glyph(builtIn(output) ? 0xf0322 : 0xf0379)
        }));
    }

    function verbs(output) {
        return choices(output).concat(factRows(output));
    }

    function choices(output) {
        if (output.logical === null)
            return [choice("on", "Turn it on", 0xf0425, false, () => change(output, "On"))];
        const now = mode(output);
        const rows = [];
        const scale = output.logical.scale;
        for (const each of [1, 1.25, 1.5, 1.75, 2, 2.5, 3])
            rows.push(choice(`scale-${each}`, `Scale to ${Math.round(each * 100)}%`, 0xf06ed,
                Math.abs(each - scale) < 0.01,
                () => change(output, { Scale: { scale: { Specific: each } } })));
        const rates = output.modes.filter(m => m.width === now.width && m.height === now.height);
        if (rates.length > 1)
            for (const each of rates)
                rows.push(choice(`rate-${hertz(each)}`, `Refresh at ${hertz(each)} Hz`, 0xf095b,
                    each === now, () => change(output, { Mode: { mode: { Specific: {
                        width: each.width,
                        height: each.height,
                        refresh: each.refresh_rate / 1000
                    } } } })));
        const sizes = [];
        for (const each of output.modes)
            if (!sizes.some(size => size.width === each.width && size.height === each.height))
                sizes.push(each);
        if (sizes.length > 1)
            for (const each of sizes)
                rows.push(choice(`size-${each.width}x${each.height}`,
                    `Use ${each.width} \u00d7 ${each.height}`, 0xf0a24,
                    each.width === now.width && each.height === now.height,
                    () => change(output, { Mode: { mode: { Specific: {
                        width: each.width,
                        height: each.height,
                        refresh: null
                    } } } })));
        // niri turns "90" counter-clockwise.
        const turns = [
            { transform: "Normal", name: "Rotate upright" },
            { transform: "90", name: "Rotate a quarter turn left" },
            { transform: "180", name: "Rotate upside down" },
            { transform: "270", name: "Rotate a quarter turn right" }
        ];
        for (const each of turns)
            rows.push(choice(`turn-${each.transform}`, each.name, 0xf0475,
                output.logical.transform === each.transform,
                () => change(output, { Transform: { transform: each.transform } })));
        if (output.vrr_supported) {
            const vrr = { vrr: !output.vrr_enabled, on_demand: false };
            rows.push(choice("vrr", output.vrr_enabled ? "Stop variable refresh"
                : "Use variable refresh", 0xf095b, false, () => change(output, { Vrr: { vrr } })));
        }
        // The last display on stays on, or nothing would show the launcher.
        if (lit.length > 1)
            rows.push(choice("off", "Turn it off", 0xf0d90, false, () => change(output, "Off")));
        return rows;
    }

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
        const each = shown.length === 1 ? "display" : "displays";
        return lit.length === shown.length ? `${shown.length} ${each}`
            : `${lit.length} of ${shown.length} on`;
    }
}
