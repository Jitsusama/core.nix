// The kernel's hwmon chips and the readings each has, for the launcher's
// Heat and Machine, so the processor's and the disk's temperatures are read
// one way and said the same in both. The folders are listed once by Qt as
// the shell starts, and a reading is read only when asked, so only sensors
// that are there are read: some chips answer every read through the
// firmware, which is slow, and it's the shell's own thread that waits.
import Qt.labs.folderlistmodel
import Quickshell
import Quickshell.Io
import QtQuick
import "Machine.js" as Machine

Scope {
    id: sensors

    function said(file) {
        probe.path = file;
        probe.reload();
        probe.waitForJob();
        return probe.text().trim();
    }

    // What a chip measures, by the name the kernel gives it, or "" for one
    // that measures many, whose sensors are named by their labels.
    function part(name) {
        const known = [
            [/^(coretemp|k10temp|zenpower)$/, "processor"],
            [/^(nvme|drivetemp)/, "disk"],
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

    // Each chip: its folder, its name, what it measures, and its listed
    // files.
    function chips() {
        const found = [];
        for (let i = 0; i < folders.count; i++) {
            const listed = files.objectAt(i);
            if (!listed)
                continue;
            const dir = `/sys/class/hwmon/${folders.get(i, "fileName")}`;
            const name = said(`${dir}/name`);
            found.push({ dir: dir, name: name, part: part(name), listed: listed });
        }
        return found;
    }

    // Each reading of a kind ("temp" or "fan") a chip has, of those its
    // folder lists, by number: its label and value, a temperature in
    // thousandths of a degree, and one that can't be a temperature left out.
    function readings(chip, kind) {
        const found = [];
        const listed = chip.listed;
        for (let i = 0; i < listed.count; i++) {
            const file = listed.get(i, "fileName");
            const n = new RegExp(`^${kind}(\\d+)_input$`).exec(file);
            if (!n)
                continue;
            const value = said(`${chip.dir}/${file}`);
            if (value === "")
                continue;
            found.push({
                n: Number(n[1]),
                label: said(`${chip.dir}/${kind}${n[1]}_label`),
                value: Number(value)
            });
        }
        return found
            .filter(s => kind !== "temp" || (s.value > -40000 && s.value < 150000))
            .sort((a, b) => a.n - b.n);
    }

    // A temperature in degrees, with the most it's meant to reach where the
    // chip says, or null.
    function degrees(chip, sensor) {
        const most = Number(said(`${chip.dir}/temp${sensor.n}_crit`));
        return { value: sensor.value / 1000, most: most > 0 ? most / 1000 : null };
    }

    // The processor's temperature, as Heat says it, or null.
    function processor() {
        const chip = chips().find(each => each.part === "processor");
        const sensor = chip ? Machine.processor(readings(chip, "temp")) : null;
        return sensor ? degrees(chip, sensor) : null;
    }

    // A disk's temperature, as Heat says it, by its folder in /sys/block,
    // or null: the one disk chip there is, or of several, the one on the
    // disk's own controller.
    function disk(block) {
        const all = chips().filter(each => each.part === "disk");
        const own = said(`/sys/block/${block}/device/dev`);
        const chip = all.length === 1 ? all[0]
            : all.find(each => own !== "" && said(`${each.dir}/device/dev`) === own);
        if (!chip)
            return null;
        const temps = readings(chip, "temp");
        const sensor = temps.find(s => s.label === "Composite")
            ?? temps.reduce((a, b) => (a && a.value >= b.value ? a : b), null);
        return sensor ? degrees(chip, sensor) : null;
    }

    // The chips, listed once; hwmon's folders are links, which Qt lists as
    // the folders they point to.
    FolderListModel {
        id: folders
        folder: "file:///sys/class/hwmon"
        showFiles: false
        showDotAndDotDot: false
        showHidden: false
        sortField: FolderListModel.Name
    }

    // Each chip's readings, listed once by Qt.
    Instantiator {
        id: files
        model: folders

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
