// The desk light, as DeskLight.qml hears it, answered in the field:
// "desk light" or "warmth" answers how it's lit, "desk light 60" sets its
// brightness, "desk light 4000 k" its warmth, "desk light off" puts it out,
// and "warmer" or "cooler" steps its warmth. Its answer holds its
// Brightness and Warmth, each a setting with a bar, stepping brighter,
// dimmer, warmer or cooler or to a level a step in, and putting it out or
// lighting it; what's typed beside them finds their choices too, so
// "brighter" needs no step in. The bar the keys show says where it got to.
// The screen's brightness is the screen's own (Displays.qml), and a
// keyboard's light the keyboard's (Keyboards.qml).
import qs
import QtQuick
import "Numbers.js" as Numbers

Provider {
    id: provider

    examples: ["desk light"]

    // The words it answers whole, and by their start (Provider.qml), as
    // the vocabulary lists them.
    questions: Words.started("light")

    // How far a step goes: a tenth of its brightness, or about the warmth
    // between two of its own button's steps.
    readonly property int brightnessStep: 10
    readonly property int warmthStep: 500
    // The levels a step into its Brightness offers, brightest first, and
    // the warmths a step into its Warmth does, warmest first.
    readonly property var levels: [100, 90, 80, 70, 60, 50, 40, 30, 20, 10]
    readonly property var warmths: [2700, 3000, 3500, 4000, 4500, 5000, 5500, 6000, 6500]

    // Whether a light is plugged in, which changes only as one comes or
    // goes, so its options aren't made again at every report it gives.
    readonly property bool plugged: DeskLight.light !== null

    // Its options, which :set sets as its rows do (Provider.qml's
    // setOptions): desklight lights it or puts it out, deskbrightness is
    // its share and deskwarmth its warmth in kelvin, as its Warmth row
    // says it, so Warmer takes from it. There are none with no light.
    setOptions: !plugged ? ({}) : ({
        desklight: {
            kind: "switch",
            now: () => DeskLight.light?.on ?? false,
            set: to => provider.done(() => provider.turn(to))
        },
        deskbrightness: {
            kind: "number",
            least: 1,
            most: 100,
            now: () => DeskLight.light?.percent ?? 0,
            set: to => provider.done(() => provider.brighten(to))
        },
        deskwarmth: {
            kind: "number",
            least: DeskLight.warmest,
            most: DeskLight.coolest,
            now: () => DeskLight.light?.kelvin ?? DeskLight.warmest,
            set: to => provider.done(() => provider.warm(to))
        }
    })

    // Does what an option's set asks, which says nothing, since the light's
    // own report says how it's lit.
    function done(change) {
        change();
        return "";
    }

    function answer(typed) {
        const text = typed.trim().toLowerCase();
        const desk = /^(?:desk (?:light|lamp)|lamp|litra)\s+(?:to\s+)?(?:(\d{1,3})\s*%?|(\d{4})\s*k|(on|off))$/
            .exec(text);
        if (desk)
            return setDesk(desk[1], desk[2], desk[3]);
        if (text === "warmer" || text === "cooler")
            return stepDesk(text);
        if (asks(text) === "")
            return null;
        return deskTold();
    }

    // A choice a step into a setting, as the vocabulary names it, keyed by
    // what it chooses too, so each level is a row of its own to hold as it
    // becomes chosen.
    function choice(key, fill, chosen, glyph, run) {
        return thing({
            key: `${key}:${Object.values(fill).join(":")}`,
            verb: Words.verb(key),
            name: Words.name(key, "", fill),
            glyph: glyph,
            hue: "action",
            side: chosen ? "chosen" : "",
            stays: true,
            run: run
        });
    }

    // A setting as a part: its name, how it's set and a bar, its choices a
    // step in and found from beside it by what's typed (`offers`).
    function setting(fields, choices) {
        return holderRow(Object.assign({ offers: true }, fields), choices);
    }

    // An act on the desk light, as the vocabulary says it, in the state
    // it's in, done as it is when it's run.
    function deskStep(key, state, run) {
        return thing({
            key: key,
            verb: Words.verb(key, state),
            name: Words.name(key, state),
            // Its words' state, so the : line names the command for it.
            wordState: state,
            glyph: Theme.glyph.lamp,
            hue: "action",
            stays: true,
            run: run
        });
    }

    // Inside the desk light: its Brightness and its Warmth, each a setting
    // stepping it or setting a level, then putting it out or lighting it,
    // as it's lit now, or as `light` says it will be.
    function deskParts(light = DeskLight.light) {
        if (light === null)
            return [];
        const span = DeskLight.coolest - DeskLight.warmest;
        const brightness = setting({
            key: "light:desk:brightness",
            name: Words.name("light:desk:brightness"),
            side: light.on ? `${light.percent}%` : `out, ${light.percent}% once lit`,
            glyph: Theme.glyph.brightness,
            graph: level(light.on ? light.percent : 0, 100)
        }, () => provider.brightnesses());
        // Warmth fills as the light warms, from its coolest to its warmest.
        const warmth = setting({
            key: "light:desk:warmth",
            name: Words.name("light:desk:warmth"),
            side: kelvins(light.kelvin),
            glyph: Theme.glyph.lamp,
            graph: level(DeskLight.coolest - light.kelvin, span)
        }, () => provider.warmthChoices());
        return [brightness, warmth, deskStep("verb:turn", light.on ? "" : "off",
            () => provider.turn(!light.on))];
    }

    // A step into the desk light's Brightness: a step brighter or dimmer,
    // then each level, the one it's at chosen. A step goes from where it
    // stands when it's run.
    function brightnesses(light = DeskLight.light) {
        const at = () => DeskLight.light.percent;
        return [
            deskStep("verb:brighter", "", () => provider.brighten(at() + brightnessStep)),
            deskStep("verb:dimmer", "", () => provider.brighten(at() - brightnessStep))
        ].concat(levels.map(each => choice("light:desk:level", { n: each },
            light.percent === each, Theme.glyph.brightness, () => provider.brighten(each))));
    }

    // A step into its Warmth: a step warmer or cooler, then each warmth.
    function warmthChoices(light = DeskLight.light) {
        const at = () => DeskLight.light.kelvin;
        return [
            deskStep("verb:warmer", "", () => provider.warm(at() - warmthStep)),
            deskStep("verb:cooler", "", () => provider.warm(at() + warmthStep))
        ].concat(warmths.map(each => choice("light:desk:kelvin", { k: each },
            light.kelvin === each, Theme.glyph.lamp, () => provider.warm(each))));
    }

    // Every row that says how the light is lit, made as `light` lights it:
    // its parts and each of their steps' choices.
    function rowsOf(light) {
        return deskParts(light).concat(brightnesses(light), warmthChoices(light));
    }

    // How a row of the light's reads now, for the launcher to tell when
    // what it was to become has come, in a step shown or not.
    function rowNow(key) {
        if (DeskLight.light === null)
            return null;
        return rowsOf(DeskLight.light).find(row => row.key === key) ?? null;
    }

    // Its rows as `changes` will light it drawn at once, and only those it
    // changes, since litra says so only once the light has answered (Joel,
    // 2026-11-07: "it should show right away"); the answer is asked again
    // so a step holding them shows it. `act` names what was asked, for the
    // foot to say if it never comes.
    function becomesLit(changes, act) {
        if (DeskLight.light === null)
            return;
        const reads = row => provider.says([row], [], true);
        const now = new Map(rowsOf(DeskLight.light).map(row => [row.key, reads(row)]));
        const holds = rowsOf(Object.assign({}, DeskLight.light, changes))
            .filter(row => now.get(row.key) !== reads(row))
            .map(row => ({ key: row.key, row: row }));
        provider.becomesAll(holds, Words.field.becoming?.objects?.deskLight ?? "", act);
        provider.answered();
    }

    function turn(on) {
        becomesLit({ on: on }, on ? "on" : "off");
        DeskLight.turn(on);
    }

    // A share, lighting it if it's out, as DeskLight takes it.
    function brighten(percent) {
        becomesLit({ on: true, percent: Math.max(0, Math.min(100, Math.round(percent))) }, "set");
        DeskLight.brighten(percent);
    }

    // A warmth, in the hundreds litra rounds to, as DeskLight takes it.
    function warm(kelvin) {
        becomesLit({ kelvin: Math.max(DeskLight.warmest, Math.min(DeskLight.coolest,
            Math.round(kelvin / 100) * 100)) }, "set");
        DeskLight.warm(kelvin);
    }

    // A warmth as the theme says a figure, grouped by thousands.
    function kelvins(kelvin) {
        return `${Numbers.format(kelvin, 0)} K`;
    }

    // How the desk light is lit, in a few words, set apart by a dot.
    function deskLit() {
        const light = DeskLight.light;
        return light.on ? `${light.percent}% \u00b7 ${kelvins(light.kelvin)}` : "off";
    }

    // A word the desk light's drawer says, as the vocabulary has it.
    function said(id) {
        return Words.of("light")?.said?.[id] ?? "";
    }

    function deskTold() {
        if (DeskLight.light === null)
            return null;
        return {
            headline: `the desk light is ${deskLit()}`,
            rows: deskParts(),
            note: "a camera lights it",
            thing: true,
            drawer: () => provider.deskDrawer()
        };
    }

    // The desk light's drawer: where it's plugged in and what it is, the
    // light by the name it gives itself, then what its answer doesn't say.
    // Its output is in lumens, the light's own units, of the range it
    // gives where it gives one, a second figure beside the answer's share
    // of that range (VKCF3Z, 2026-11-08); its warmth is the answer's, so
    // it isn't said again; its HID device and serial come last, as identifiers
    // do in every drawer. litra speaks only to lights on USB.
    function deskDrawer(light = DeskLight.light) {
        if (light === null)
            return null;
        const own = String(light.name ?? "").trim();
        const lumens = each => `${Numbers.format(each, 0)} lm`;
        const range = light.dimmest === undefined || light.brightest === undefined ? ""
            : said("of").replace("{least}", Numbers.format(light.dimmest, 0))
                .replace("{most}", lumens(light.brightest));
        const facts = [
            ["output", light.lumens === undefined ? ""
                : [lumens(light.lumens), range].filter(part => part !== "").join(" \u00b7 ")],
            ["hid-device", String(light.node ?? "")],
            ["serial", String(light.serial ?? "")]
        ].filter(([, value]) => value !== "")
            .map(([id, value]) => ({ label: Words.name(`light:desk:${id}`), value: value }));
        return {
            eyebrow: [said("usb"), said("plate")].join(" \u00b7 "),
            title: own === "" ? Words.name("light") : own,
            facts: facts
        };
    }

    // Setting the desk light to a share, a warmth, or on or off, Return
    // doing it.
    function setDesk(share, kelvin, power) {
        const shown = deskTold();
        if (shown === null)
            return null;
        const done = power !== undefined ? {
            headline: power === "on" ? "turn the desk light on" : "turn the desk light off",
            run: () => provider.turn(power === "on")
        } : share !== undefined ? {
            headline: `set the desk light to ${Math.min(100, Number(share))}%`,
            run: () => provider.brighten(Number(share))
        } : {
            headline: `set the desk light to ${kelvins(Number(kelvin))}`,
            run: () => provider.warm(Number(kelvin))
        };
        return Object.assign(done, {
            verb: Words.command("command:desk").verb,
            stays: true,
            rows: shown.rows,
            note: `now ${deskLit()}`
        });
    }

    function stepDesk(way) {
        const shown = deskTold();
        if (shown === null || !DeskLight.light.on)
            return shown;
        const kelvin = DeskLight.light.kelvin + (way === "warmer" ? -warmthStep : warmthStep);
        const kept = Math.max(DeskLight.warmest, Math.min(DeskLight.coolest, kelvin));
        return {
            headline: `make the desk light ${way} \u00b7 ${kelvins(kept)}`,
            verb: Words.command("command:desk").verb,
            run: () => provider.warm(kept),
            stays: true,
            rows: shown.rows,
            note: `now ${deskLit()}`
        };
    }

    // A light's level as a bar. However bright, it wants nothing, so the
    // bar never warns.
    function level(now, most) {
        return { shape: "bar", values: [most > 0 ? Math.max(0, Math.min(1, now / most)) : 0],
            warn: false };
    }

    // Its acts keep the launcher, so what they changed is said at once,
    // wherever its rows are shown.
    Connections {
        target: DeskLight

        function onLightChanged() {
            provider.answered();
        }
    }
}
