// How hot the machine runs, answered in the field: "temperature", "temps",
// "heat", "hot", "thermal", "sensors" or "fans". The headline is the
// hottest part and the fans; the rows are each part by what it is, the
// processor as a whole and its hottest core, the disk, the battery, the
// Wi-Fi card, the memory and the air inside, and each fan's speed. They're
// read from the kernel's hwmon files when asked, the folders listed once by
// Qt as the shell starts, so nothing is spawned and nothing is watched. A
// sensor whose chip doesn't say what it measures is left out, so a reading
// is never shown twice under two names.
import Qt.labs.folderlistmodel
import Quickshell.Io
import QtQuick
import "Numbers.js" as Numbers

Provider {
    id: provider

    examples: ["temperature", "fans"]

    // What a chip measures, by the name the kernel gives it, or "" for one
    // that measures many, whose sensors are named by their labels.
    function part(name) {
        const known = [
            [/^(coretemp|k10temp|zenpower)$/, "processor"],
            [/^nvme/, "disk"],
            [/^iwlwifi/, "Wi-Fi card"],
            [/^acpitz$/, "board"],
            [/^pch_/, "chipset"],
            [/^(amdgpu|xe|i915|nouveau)$/, "graphics"],
            // A laptop's BAT0, or a battery named as one, as the kernel's
            // stand-in is.
            [/^BAT|battery/, "battery"]
        ];
        const match = known.find(([pattern]) => pattern.test(name));
        return match ? match[1] : "";
    }
    // A labelled sensor's own part, where its chip measures many.
    readonly property var labelled: ({
        Ambient: "air inside",
        SODIMM: "memory",
        GPU: "graphics"
    })

    function said(file) {
        probe.path = file;
        return read(probe).trim();
    }

    function number(value) {
        return Numbers.format(value, 0, settings.group, settings.point);
    }

    function answer(typed) {
        const text = typed.trim().toLowerCase();
        if (!/^(temperatures?|temps?|heat|hot|thermals?|sensors?|fans?|cooling)$/.test(text))
            return null;
        return told();
    }

    // Each sensor a chip has, of those its folder lists, so only sensors
    // that are there are read: some chips answer every read through the
    // firmware, which is slow, and it's the shell's own thread that waits.
    function sensors(dir, listed, kind) {
        const found = [];
        for (let i = 0; i < listed.count; i++) {
            const file = listed.get(i, "fileName");
            const n = new RegExp(`^${kind}(\\d+)_input$`).exec(file);
            if (!n)
                continue;
            const value = said(`${dir}/${file}`);
            if (value === "")
                continue;
            found.push({ n: Number(n[1]), label: said(`${dir}/${kind}${n[1]}_label`), value: Number(value) });
        }
        return found.sort((a, b) => a.n - b.n);
    }

    function told() {
        const glyph = String.fromCodePoint(0xf050f);
        const fan = String.fromCodePoint(0xf0210);
        const heat = [];
        const fans = [];
        const seen = new Set();
        for (let i = 0; i < chips.count; i++) {
            const dir = `/sys/class/hwmon/${chips.get(i, "fileName")}`;
            const listed = files.objectAt(i);
            if (!listed)
                continue;
            const name = said(`${dir}/name`);
            const temps = sensors(dir, listed, "temp").filter(s => s.value > -40000 && s.value < 150000);
            for (const s of sensors(dir, listed, "fan"))
                if (s.label === "" || !seen.has(s.label)) {
                    seen.add(s.label);
                    fans.push({ name: s.label || "fan", rpm: s.value });
                }
            if (temps.length === 0)
                continue;
            const what = part(name);
            if (what === "processor") {
                const whole = temps.find(s => /^(Package|Tctl|Tdie)/.test(s.label));
                const cores = temps.filter(s => /^Core/.test(s.label));
                const hottest = cores.reduce((a, b) => (b.value > (a ? a.value : -Infinity) ? b : a), null);
                heat.push({ part: "processor", value: (whole ?? hottest ?? temps[0]).value / 1000 });
                // The hottest core only when it says more than the whole,
                // which usually reads as its hottest core.
                if (hottest && whole && Math.round(hottest.value / 1000) !== Math.round(whole.value / 1000))
                    heat.push({ part: `hottest core, ${hottest.label.toLowerCase()}`, value: hottest.value / 1000 });
            } else if (what !== "") {
                const whole = temps.find(s => s.label === "Composite") ?? temps
                    .reduce((a, b) => (b.value > a.value ? b : a));
                heat.push({ part: what, value: whole.value / 1000 });
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

    // The chips, listed once; hwmon's folders are links, which Qt lists as
    // the folders they point to.
    FolderListModel {
        id: chips
        folder: "file:///sys/class/hwmon"
        showFiles: false
        showDotAndDotDot: false
        showHidden: false
        sortField: FolderListModel.Name
    }

    // Each chip's sensor readings, listed once by Qt.
    Instantiator {
        id: files
        model: chips

        FolderListModel {
            required property string fileName
            folder: `file:///sys/class/hwmon/${fileName}`
            showDirs: false
            showDotAndDotDot: false
            nameFilters: ["temp*_input", "fan*_input"]
        }
    }

    FileView {
        id: probe
        blockLoading: true
        printErrors: false
    }
}
