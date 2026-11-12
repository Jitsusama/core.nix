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
    // Said by its value alone, as every choice is (VKCF3Z, 2026-11-09):
    // "60%", "4,000 K", its setting's name a hidden word so "warmth 4000"
    // still finds it, and its words' :set deskbrightness= its command.
    // `made` holds its words' id, the value it's keyed by, what fills its
    // words, whether it's chosen, its mark, its run and its setting.
    function choice(made) {
        return thing({
            key: `${made.key}:${made.value}`,
            wordsId: made.key,
            fill: made.fill,
            verb: Words.verb(made.key),
            name: Words.name(made.key, "", made.fill),
            keywords: `${Words.name(made.setting)} ${made.value}`,
            glyph: made.glyph,
            hue: "action",
            side: made.chosen ? said("chosen") : "",
            stays: true,
            run: made.run
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
            side: light.on ? `${light.percent}%`
                : said("outOnceLit").replace("{percent}", light.percent),
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
        ].concat(levels.map(each => choice({
            key: "light:desk:level",
            value: each,
            fill: { n: each },
            chosen: light.percent === each,
            glyph: Theme.glyph.brightness,
            run: () => provider.brighten(each),
            setting: "light:desk:brightness"
        })));
    }

    // A step into its Warmth: a step warmer or cooler, then each warmth.
    function warmthChoices(light = DeskLight.light) {
        const at = () => DeskLight.light.kelvin;
        return [
            deskStep("verb:warmer", "", () => provider.warm(at() - warmthStep)),
            deskStep("verb:cooler", "", () => provider.warm(at() + warmthStep))
        ].concat(warmths.map(each => choice({
            key: "light:desk:kelvin",
            value: each,
            fill: { k: each, warmth: kelvins(each) },
            chosen: light.kelvin === each,
            glyph: Theme.glyph.lamp,
            run: () => provider.warm(each),
            setting: "light:desk:warmth"
        })));
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
        return light.on ? `${light.percent}% \u00b7 ${kelvins(light.kelvin)}` : said("off");
    }

    // A word the desk light's drawer says, as the vocabulary has it.
    function said(id) {
        return Words.of("light")?.said?.[id] ?? "";
    }

    // The desk light's answer. With no light its row stays where Machine's
    // head names it, saying whether none is plugged in or litra isn't
    // answering, with nothing inside it to set (Y2YBTS, 2026-11-12: a
    // place whose source is away keeps its row). It's never gone, since a
    // show skips a gone row and a terminal's show must find the row
    // Machine names, as Heat's held row is (Heat.qml's unheld). It says it
    // has parts, since a place holds an answer only with its parts or
    // whole (Launcher.qml's resolved), so Machine keeps it in its place.
    function deskTold() {
        if (DeskLight.light === null)
            return {
                headline: "",
                glyph: Theme.glyph.lamp,
                rows: [],
                note: said(DeskLight.answering ? "unplugged" : "down"),
                parted: true
            };
        return {
            // Its side says only how it's lit, never its name again
            // (VKCF3Z, 2026-11-12).
            headline: deskLit(),
            rows: deskParts(),
            note: said("cameraLights"),
            thing: true,
            drawer: () => provider.deskDrawer()
        };
    }

    // The desk light's drawer: where it's plugged in and what it is, the
    // light by the name it gives itself, then what its answer doesn't say.
    // Its output is in lumens, the light's own units, of the range it
    // gives where it gives one, a second figure beside the answer's share
    // of that range (VKCF3Z, 2026-11-08); its warmth is the answer's, so
    // it isn't said again; its HID device and serial come last, marked as
    // identifiers (`identifies`), as they are in every drawer. litra speaks
    // only to lights on USB.
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
                : [lumens(light.lumens), range].filter(part => part !== "").join(" \u00b7 "), false],
            ["hid-device", String(light.node ?? ""), true],
            ["serial", String(light.serial ?? ""), true]
        ].filter(([, value]) => value !== "")
            .map(([id, value, identifies]) => ({
                label: Words.name(`light:desk:${id}`), value: value, identifies: identifies
            }));
        return {
            eyebrow: [said("usb"), said("plate")].join(" \u00b7 "),
            title: own === "" ? Words.name("light") : own,
            facts: facts
        };
    }

    // What setting the desk light does, said and run: on or off where a
    // power is given, else a share, else a warmth.
    function deskDone(share, kelvin, power) {
        if (power !== undefined)
            return {
                headline: said(power === "on" ? "turnOn" : "turnOff"),
                run: () => provider.turn(power === "on")
            };
        if (share !== undefined)
            return {
                headline: said("setTo").replace("{to}", `${Math.min(100, Number(share))}%`),
                run: () => provider.brighten(Number(share))
            };
        return {
            headline: said("setTo").replace("{to}", kelvins(Number(kelvin))),
            run: () => provider.warm(Number(kelvin))
        };
    }

    // Setting the desk light to a share, a warmth, or on or off, Return
    // doing it.
    function setDesk(share, kelvin, power) {
        if (DeskLight.light === null)
            return null;
        const shown = deskTold();
        return Object.assign(deskDone(share, kelvin, power), {
            verb: Words.command("command:desk").verb,
            stays: true,
            rows: shown.rows,
            note: said("now").replace("{lit}", deskLit())
        });
    }

    function stepDesk(way) {
        if (DeskLight.light === null)
            return null;
        const shown = deskTold();
        if (!DeskLight.light.on)
            return shown;
        const kelvin = DeskLight.light.kelvin + (way === "warmer" ? -warmthStep : warmthStep);
        const kept = Math.max(DeskLight.warmest, Math.min(DeskLight.coolest, kelvin));
        return {
            headline: said("make").replace("{way}", way).replace("{warmth}", kelvins(kept)),
            verb: Words.command("command:desk").verb,
            run: () => provider.warm(kept),
            stays: true,
            rows: shown.rows,
            note: said("now").replace("{lit}", deskLit())
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
