// How hot the machine runs, answered in the field: "temperature", "temps",
// "heat", "hot", "thermal", "sensors" or "fans". The headline is the
// hottest part and the fans; the rows are each part by what it is, the
// processor as a whole and its hottest core, the disk, the battery, the
// Wi-Fi card, the memory and the air inside, and each fan's speed. They're
// read from the kernel's hwmon files when asked (Sensors.qml), the same way
// the machine's answers read the processor's and the disk's, so nothing is
// spawned and nothing is watched. A sensor whose chip doesn't say what it
// measures is left out, so a reading is never shown twice under two names.
import QtQuick
import "Machine.js" as Machine
import "Numbers.js" as Numbers

Provider {
    id: provider

    examples: ["temperature", "fans"]

    // A labelled sensor's own part, where its chip measures many.
    readonly property var labelled: ({
        Ambient: "air inside",
        SODIMM: "memory",
        GPU: "graphics"
    })

    function number(value) {
        return Numbers.format(value, 0, settings.group, settings.point);
    }

    // The words it answers whole, and by their start (Provider.qml).
    questions: ["temperature", "temp", "heat", "hot", "how hot", "thermal", "sensor", "fan", "fan speed",
        "cooling", "cpu temperature", "cpu temp"]

    function answer(typed) {
        if (asks(typed) === "")
            return null;
        return told();
    }

    function told() {
        const glyph = String.fromCodePoint(0xf050f);
        const fan = String.fromCodePoint(0xf0210);
        const heat = [];
        const fans = [];
        const seen = new Set();
        for (const chip of sensors.chips()) {
            const temps = sensors.readings(chip, "temp");
            for (const s of sensors.readings(chip, "fan"))
                if (s.label === "" || !seen.has(s.label)) {
                    seen.add(s.label);
                    fans.push({ name: s.label || "fan", rpm: s.value });
                }
            if (temps.length === 0)
                continue;
            if (chip.part === "processor") {
                const whole = Machine.wholeProcessor(temps);
                const hottest = Machine.hottestCore(temps);
                heat.push({ part: "processor", value: Machine.processor(temps).value / 1000 });
                // The hottest core only when it says more than the whole,
                // which usually reads as its hottest core.
                if (hottest && whole && Math.round(hottest.value / 1000) !== Math.round(whole.value / 1000))
                    heat.push({ part: `hottest core, ${hottest.label.toLowerCase()}`, value: hottest.value / 1000 });
            } else if (chip.part !== "") {
                const whole = temps.find(s => s.label === "Composite") ?? temps
                    .reduce((a, b) => (b.value > a.value ? b : a));
                heat.push({ part: chip.part, value: whole.value / 1000 });
            } else {
                for (const s of temps)
                    if (labelled[s.label])
                        heat.push({ part: labelled[s.label], value: s.value / 1000 });
            }
        }
        // Two chips can report the same fans, one naming them and one not,
        // so the unnamed are left out where any are named.
        const named = fans.filter(f => f.name !== "fan");
        if (named.length > 0)
            fans.splice(0, fans.length, ...named);
        if (heat.length === 0 && fans.length === 0)
            return null;
        const rows = heat
            .map(h => copyRow({ name: `${number(h.value)} \u00b0C`, side: h.part, glyph: glyph }))
            .concat(fans.map(f => copyRow({
                name: f.rpm > 0 ? `${number(f.rpm)} rpm` : "still",
                side: f.name.toLowerCase(),
                glyph: fan
            })));
        const hottest = heat.reduce((a, b) => (a && a.value >= b.value ? a : b), null);
        const turning = fans.filter(f => f.rpm > 0).length;
        const summary = [];
        if (hottest)
            summary.push(`${number(hottest.value)} \u00b0C at the ${hottest.part.split(",")[0]}`);
        if (fans.length > 0)
            summary.push(turning === 0 ? "fans still" : turning === 1 ? "a fan turning" : `${turning} fans turning`);
        return { headline: summary.join(", "), rows: rows, note: "read just now, from the kernel" };
    }

    Sensors {
        id: sensors
    }
}
