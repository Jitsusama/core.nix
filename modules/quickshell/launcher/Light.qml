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

    // A choice a step into a setting, as the vocabulary names it.
    function choice(key, fill, inUse, glyph, run) {
        return thing({
            key: key,
            verb: Words.verb(key),
            name: Words.name(key, "", fill),
            glyph: glyph,
            hue: "action",
            side: inUse ? "in use" : "",
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
            glyph: Theme.glyph.lamp,
            hue: "action",
            stays: true,
            run: run
        });
    }

    // Inside the desk light: its Brightness and its Warmth, each a setting
    // stepping it or setting a level, then putting it out or lighting it.
    function deskParts() {
        const light = DeskLight.light;
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
            side: `${light.kelvin} K`,
            glyph: Theme.glyph.lamp,
            graph: level(DeskLight.coolest - light.kelvin, span)
        }, () => provider.warmthChoices());
        return [brightness, warmth, deskStep("verb:turn", light.on ? "" : "off",
            () => DeskLight.turn(!DeskLight.light.on))];
    }

    // A step into the desk light's Brightness: a step brighter or dimmer,
    // then each level, the one it's at in use.
    function brightnesses() {
        const at = () => DeskLight.light.percent;
        return [
            deskStep("verb:brighter", "", () => DeskLight.brighten(at() + brightnessStep)),
            deskStep("verb:dimmer", "", () => DeskLight.brighten(at() - brightnessStep))
        ].concat(levels.map(each => choice("light:desk:level", { n: each }, at() === each,
            Theme.glyph.brightness, () => DeskLight.brighten(each))));
    }

    // A step into its Warmth: a step warmer or cooler, then each warmth.
    function warmthChoices() {
        const at = () => DeskLight.light.kelvin;
        return [
            deskStep("verb:warmer", "", () => DeskLight.warm(at() - warmthStep)),
            deskStep("verb:cooler", "", () => DeskLight.warm(at() + warmthStep))
        ].concat(warmths.map(each => choice("light:desk:kelvin", { k: each }, at() === each,
            Theme.glyph.lamp, () => DeskLight.warm(each))));
    }

    // How the desk light is lit, in a few words.
    function deskLit() {
        const light = DeskLight.light;
        return light.on ? `${light.percent}%, ${light.kelvin} K` : "off";
    }

    function deskTold() {
        if (DeskLight.light === null)
            return null;
        return {
            headline: `the desk light is ${deskLit()}`,
            rows: deskParts(),
            note: "a camera lights it"
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
            run: () => DeskLight.turn(power === "on")
        } : share !== undefined ? {
            headline: `set the desk light to ${Math.min(100, Number(share))}%`,
            run: () => DeskLight.brighten(Number(share))
        } : {
            headline: `set the desk light to ${kelvin} K`,
            run: () => DeskLight.warm(Number(kelvin))
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
            headline: `make the desk light ${way}, ${kept} K`,
            verb: Words.command("command:desk").verb,
            run: () => DeskLight.warm(kept),
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
