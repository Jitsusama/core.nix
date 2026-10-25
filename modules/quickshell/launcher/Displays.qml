// Displays, a collection: the displays niri drives, the built-in one first,
// each with its size, refresh rate and scale in the footer. Return moves
// focus to one, or turns on one that's off; the right arrow lists its
// Scale, Refresh, Resolution and Rotation as parts saying how each is set,
// each stepping into its choices (which what's typed there finds too, so
// "scale 150" needs no step in), and turning it off while another is on.
// "colour depth", "refresh rate" and the like ask the focused display's
// part, or each display's while several are on. niri forgets these choices when its configuration is next read,
// so they last the session, and what lasts belongs in the configuration.
// Everything goes over niri's socket, asked again whenever the launcher
// opens or the workspaces change, as they do when a display comes or goes,
// since niri sends no event for a display itself. What a display's EDID
// says of itself, who made it and when, its size, native mode, refresh
// range, colour depth, colours and HDR, desk-facts writes to a file
// (modules/desk-facts), and the right arrow lists after the settings,
// named first as parts are (Colour depth "10 bit"), Return copying one.
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

    // What desk-facts says of a display, each named first as a part is:
    // Colour depth "10 bit", Colours, then who made it and when. Its input
    // says the colour depth and the link as one ("10 bit, DisplayPort"),
    // which reads as two parts, the depth first, since it's what's asked.
    function factRows(output) {
        const said = [];
        for (const fact of facts[output.name] ?? []) {
            const both = fact.name === "input" ? bitsAndLink.exec(fact.value) : null;
            if (both) {
                said.push({ id: "depth", value: both[1] }, { id: "input", value: both[2] });
            } else {
                said.push({ id: fact.name.toLowerCase(), value: fact.value });
            }
        }
        const first = said.filter(fact => askedFirst.includes(fact.id));
        return first.concat(said.filter(fact => !askedFirst.includes(fact.id))).map(fact => copyRow({
            key: `fact:displays:${output.name}:${fact.id}`,
            name: partName(fact.id),
            side: fact.value,
            copy: fact.value
        }));
    }

    // "10 bit, DisplayPort", as desk-facts says a display's input.
    readonly property var bitsAndLink: /^(\d+ bits?), (.+)$/
    // The facts asked of a display most, which lead the rest.
    readonly property var askedFirst: ["depth", "colours", "hdr"]

    // A part's name, as the vocabulary says it (displays:depth is "Colour
    // depth"), or the fact's own, capitalised, for one it doesn't know.
    function partName(id) {
        const named = Words.name(`displays:${id}`);
        return named !== `displays:${id}` ? named : id.charAt(0).toUpperCase() + id.slice(1);
    }

    // A display's right arrow: turning it on while it's off; while it's
    // on, each of its settings as a part saying how it's set, whose step
    // in offers the others (and whose choices what's typed finds from
    // here, so "scale 150" is Scale to 150%), then turning it off while
    // another is on, then what its EDID says.
    function verbs(output) {
        if (output.logical === null)
            return [choice("on", "Turn it on", 0xf0425, false, () => change(output, "On"))]
                .concat(factRows(output));
        const rows = setUp(output);
        // The last display on stays on, or nothing would show the launcher.
        if (lit.length > 1)
            rows.push(choice("off", "Turn it off", 0xf0d90, false, () => change(output, "Off")));
        return rows.concat(factRows(output));
    }

    // A setting as a part: its name, how it's set, and the choices a step
    // in, or only how it's set when there's nothing else to choose.
    function setting(output, id, side, glyphCode, choices) {
        const fields = {
            key: `displays:${output.name}:${id}`,
            name: partName(id),
            side: side
        };
        if (choices.length === 0)
            return copyRow(Object.assign(fields, { copy: "", hue: "machine", glyph: "" }));
        return holderRow(Object.assign(fields, { glyph: glyph(glyphCode), offers: true }),
            () => choices);
    }

    // How niri turns a display, as a part says it and as its choice does;
    // niri turns "90" counter-clockwise.
    readonly property var turns: [
        { transform: "Normal", side: "upright", name: "Rotate upright" },
        { transform: "90", side: "a quarter turn left", name: "Rotate a quarter turn left" },
        { transform: "180", side: "upside down", name: "Rotate upside down" },
        { transform: "270", side: "a quarter turn right", name: "Rotate a quarter turn right" }
    ]

    readonly property var scales: [1, 1.25, 1.5, 1.75, 2, 2.5, 3]

    function percent(scale) {
        return `${Math.round(scale * 100)}%`;
    }

    // How it's set: Scale, Refresh, Resolution, Rotation.
    function setUp(output) {
        const now = mode(output);
        const scale = output.logical.scale;
        const rows = [setting(output, "scale", percent(scale), 0xf06ed, scales.map(each =>
            choice(`scale-${each}`, `Scale to ${percent(each)}`, 0xf06ed,
                Math.abs(each - scale) < 0.01,
                () => change(output, { Scale: { scale: { Specific: each } } }))))];
        if (now !== null) {
            const rates = output.modes.filter(m => m.width === now.width && m.height === now.height);
            const refreshes = rates.length < 2 ? [] : rates.map(each =>
                choice(`rate-${hertz(each)}`, `Refresh at ${hertz(each)} Hz`, 0xf095b,
                    each === now, () => change(output, { Mode: { mode: { Specific: {
                        width: each.width,
                        height: each.height,
                        refresh: each.refresh_rate / 1000
                    } } } })));
            if (output.vrr_supported) {
                const vrr = { vrr: !output.vrr_enabled, on_demand: false };
                refreshes.push(choice("vrr", output.vrr_enabled ? "Stop variable refresh"
                    : "Use variable refresh", 0xf095b, false,
                    () => change(output, { Vrr: { vrr } })));
            }
            const variable = output.vrr_enabled ? ", variable" : "";
            rows.push(setting(output, "refresh", `${hertz(now)} Hz${variable}`, 0xf095b,
                refreshes));
            const sizes = [];
            for (const each of output.modes)
                if (!sizes.some(size => size.width === each.width && size.height === each.height))
                    sizes.push(each);
            rows.push(setting(output, "resolution", `${now.width} \u00d7 ${now.height}`, 0xf0a24,
                sizes.length < 2 ? [] : sizes.map(each =>
                    choice(`size-${each.width}x${each.height}`,
                        `Use ${each.width} \u00d7 ${each.height}`, 0xf0a24,
                        each.width === now.width && each.height === now.height,
                        () => change(output, { Mode: { mode: { Specific: {
                            width: each.width,
                            height: each.height,
                            refresh: null
                        } } } })))));
        }
        const turned = turns.find(each => each.transform === output.logical.transform);
        rows.push(setting(output, "rotation", turned ? turned.side : "flipped", 0xf0475,
            turns.map(each => choice(`turn-${each.transform}`, each.name, 0xf0475,
                output.logical.transform === each.transform,
                () => change(output, { Transform: { transform: each.transform } })))));
        return rows;
    }

    // Asked of a display by a part's words, "colour depth" or "refresh
    // rate": the focused one's, or each one's when several are on.
    readonly property var partsAsked: {
        const out = {};
        for (const id of ["scale", "refresh", "resolution", "rotation", "depth", "colours", "hdr"])
            for (const question of Words.answers(`displays:${id}`))
                out[question] = id;
        return out;
    }
    questions: Object.keys(partsAsked)

    function answer(typed) {
        const id = partsAsked[asks(typed)];
        if (id === undefined || lit.length === 0)
            return null;
        const partOf = output => verbs(output).find(row => row.name === partName(id)) ?? null;
        const focused = lit.find(output => output.name === focusedOutput) ?? lit[0];
        const part = partOf(focused);
        if (part === null)
            return null;
        const others = lit.filter(output => output !== focused && partOf(output) !== null);
        return {
            headline: part.side,
            copy: "",
            glyph: Theme.glyph.display,
            note: named(focused),
            rows: others.length === 0 ? (part.verbs ? part.verbs() : [])
                : others.map(output => copyRow({
                    key: `displays:${output.name}:${id}:said`,
                    name: named(output),
                    side: partOf(output).side,
                    copy: "",
                    glyph: ""
                }))
        };
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
