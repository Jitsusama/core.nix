pragma Singleton
// The kernel's hwmon chips and the readings each has, for the launcher's
// Heat and Machine, so the processor's and the disk's temperatures are read
// one way and said the same in both. Both ask this one, so the chips are
// listed and what never changes is read once for the shell, not once each. The chips are listed once by Qt as
// the shell starts, and the readings each has by one listing of them all,
// made again only when the chips change, never a reader per chip, since
// each would be a thread and a watcher of its own over folders that don't
// change. A reading is read only when asked, so only sensors
// that are there are read: some chips answer every read through the
// firmware, which is slow, and it's the shell's own thread that waits.
// What a chip says of itself that never changes, its name and each
// sensor's label and limits, a fan's most, and which core each processor
// is, is read once, so asking again each second reads only the readings:
// a third of the files the processor's chip has, which under a loaded
// processor was most of the time a read took. A sensor that can't be
// asked for (a chip's unlabelled one) is never read at all. The thermal
// zones are listed too, for a part no chip gives, and the processors'
// counts of slowing themselves to cool down.
import Qt.labs.folderlistmodel
import Quickshell
import Quickshell.Io
import QtQuick
import "Machine.js" as Machine

Singleton {
    id: sensors

    function said(file) {
        probe.path = file;
        probe.reload();
        probe.waitForJob();
        return probe.text().trim();
    }

    // What's read once, by file. Forgotten whenever the chips listed
    // change, since the number one unplugged leaves can go to another.
    property var kept: ({})

    function fixed(file) {
        if (!(file in kept))
            kept[file] = said(file);
        return kept[file];
    }

    // What a chip measures, by the name the kernel gives it, as Heat's
    // words name the part (heat:<id>), or "" for one that measures many,
    // whose sensors are named by their labels.
    function part(name) {
        const known = [
            [/^(coretemp|k10temp|zenpower)$/, "processor"],
            [/^(nvme|drivetemp)/, "disk"],
            [/^iwlwifi/, "wifi"],
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
            const folder = folders.get(i, "fileName");
            const listed = listing[folder];
            if (!listed)
                continue;
            const dir = `/sys/class/hwmon/${folder}`;
            const name = fixed(`${dir}/name`);
            found.push({ dir: dir, name: name, part: part(name), listed: listed });
        }
        return found;
    }

    // Whether a chip's folder lists a file.
    function lists(chip, file) {
        return chip.listed.includes(file);
    }

    // Each reading of a kind ("temp" or "fan") a chip has, of those its
    // folder lists, by number: its label and value, a temperature in
    // thousandths of a degree, and one that can't be a temperature left out.
    // Given `wanted`, only a sensor whose label it wants is read.
    function readings(chip, kind, wanted) {
        const found = [];
        for (const file of chip.listed) {
            const n = new RegExp(`^${kind}(\\d+)_input$`).exec(file);
            if (!n)
                continue;
            const label = fixed(`${chip.dir}/${kind}${n[1]}_label`);
            if (wanted && !wanted(label))
                continue;
            const value = said(`${chip.dir}/${file}`);
            if (value === "")
                continue;
            found.push({ n: Number(n[1]), label: label, value: Number(value) });
        }
        return found
            .filter(s => kind !== "temp" || (s.value > -40000 && s.value < 150000))
            .sort((a, b) => a.n - b.n);
    }

    // A limit a chip says in thousandths of a degree, in degrees, or null
    // where it says none or one no temperature could be, as a disk's
    // unused sensors say 65,261 degrees.
    function limit(file) {
        const most = Number(fixed(file));
        return most > 0 && most < 150000 ? most / 1000 : null;
    }

    // A temperature in degrees, with the most it's meant to reach (its
    // critical point) and where its chip says it's getting hot, where the
    // chip says them, or null.
    function degrees(chip, sensor) {
        const crit = limit(`${chip.dir}/temp${sensor.n}_crit`);
        return {
            value: sensor.value / 1000,
            most: crit,
            high: limit(`${chip.dir}/temp${sensor.n}_max`)
        };
    }

    // What a fan's chip says of it beside its speed: its most, its aim
    // and its power from 0 to 1, each null where the chip says none. Its
    // aim and power are read each time, its most once.
    function fan(chip, sensor) {
        const most = Number(fixed(`${chip.dir}/fan${sensor.n}_max`));
        const target = lists(chip, `fan${sensor.n}_target`)
            ? Number(said(`${chip.dir}/fan${sensor.n}_target`)) : NaN;
        const power = lists(chip, `pwm${sensor.n}`)
            ? Number(said(`${chip.dir}/pwm${sensor.n}`)) : NaN;
        return {
            max: most > 0 ? most : null,
            target: target > 0 ? target : null,
            power: isNaN(power) ? null : Math.max(0, Math.min(1, power / 255))
        };
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
            : all.find(each => own !== "" && fixed(`${each.dir}/device/dev`) === own);
        if (!chip)
            return null;
        const temps = readings(chip, "temp");
        const sensor = temps.find(s => s.label === "Composite")
            ?? temps.reduce((a, b) => (a && a.value >= b.value ? a : b), null);
        return sensor ? degrees(chip, sensor) : null;
    }

    // The thermal zones, each { type, dir }, their types read once.
    function zones() {
        const found = [];
        for (let i = 0; i < zoneFolders.count; i++) {
            const name = zoneFolders.get(i, "fileName");
            if (!/^thermal_zone\d+$/.test(name))
                continue;
            const dir = `/sys/class/thermal/${name}`;
            found.push({ type: fixed(`${dir}/type`), dir: dir });
        }
        return found;
    }

    // A zone's temperature in degrees, or null where it says none.
    function zoneDegrees(zone) {
        const text = said(`${zone.dir}/temp`);
        const value = Number(text);
        return text !== "" && value > -40000 && value < 150000 ? value / 1000 : null;
    }

    // The logical processors, each with the core it runs on, read once:
    // { cpu, core }.
    function cpus() {
        const online = Machine.cpus(fixed("/sys/devices/system/cpu/online"));
        return online.map(cpu => ({
            cpu: cpu,
            core: Number(fixed(`/sys/devices/system/cpu/cpu${cpu}/topology/core_id`))
        })).filter(each => !isNaN(each.core));
    }

    // How often each processor has seen itself and its package slowed to
    // cool down, as Heat.js's throttled reads them: each core's count once,
    // from its first processor, and the package's time only from the
    // processor that saw most. None where the kernel doesn't count them.
    function throttles() {
        const all = cpus();
        const at = cpu => `/sys/devices/system/cpu/cpu${cpu}/thermal_throttle`;
        if (all.length === 0 || fixed(`${at(all[0].cpu)}/package_throttle_count`) === "")
            return [];
        const first = {};
        for (const each of all)
            if (first[each.core] === undefined || each.cpu < first[each.core])
                first[each.core] = each.cpu;
        const counted = all.map(each => ({
            cpu: each.cpu,
            core: each.core,
            cores: first[each.core] === each.cpu
                ? Number(said(`${at(each.cpu)}/core_throttle_count`)) || 0 : 0,
            package: Number(said(`${at(each.cpu)}/package_throttle_count`)) || 0
        }));
        const most = counted.reduce((a, b) => (a && a.package >= b.package ? a : b), null);
        most.packageMs = Number(said(`${at(most.cpu)}/package_throttle_total_time_ms`)) || 0;
        most.packageLongest = Number(said(`${at(most.cpu)}/package_throttle_max_time_ms`)) || 0;
        return counted;
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

    // Each chip's readings, by its folder's name, as one listing of every
    // chip says them; made again whenever the chips listed change, which
    // also forgets what was read once.
    property var listing: ({})
    // Chips that change while they're being listed are listed again after.
    property bool relist: false
    function listChips() {
        if (folders.status !== FolderListModel.Ready)
            return;
        if (chipFiles.running)
            relist = true;
        else
            chipFiles.running = true;
    }
    Connections {
        target: folders

        function onCountChanged() {
            Qt.callLater(sensors.listChips);
        }

        function onStatusChanged() {
            Qt.callLater(sensors.listChips);
        }
    }
    Process {
        id: chipFiles
        command: ["sh", "-c", 'cd /sys/class/hwmon 2>/dev/null || exit 0; for f in */temp*_input'
            + ' */fan*_input */fan*_target */pwm*; do [ -e "$f" ] && echo "$f"; done; true']
        onExited: {
            if (!sensors.relist)
                return;
            sensors.relist = false;
            Qt.callLater(sensors.listChips);
        }
        stdout: StdioCollector {
            onStreamFinished: {
                const next = {};
                for (const line of text.split("\n")) {
                    const at = line.indexOf("/");
                    if (at <= 0)
                        continue;
                    const chip = line.slice(0, at);
                    next[chip] = (next[chip] ?? []).concat([line.slice(at + 1)]);
                }
                sensors.kept = {};
                sensors.listing = next;
            }
        }
    }

    // The thermal zones, listed once.
    FolderListModel {
        id: zoneFolders
        folder: "file:///sys/class/thermal"
        showFiles: false
        showDotAndDotDot: false
        showHidden: false
        sortField: FolderListModel.Name
    }

    // Read on the shell's own thread, as Machine's files are: a sensor's
    // file is a few bytes, and handing each read to a reader thread and
    // waiting for it cost more than the read.
    FileView {
        id: probe
        preload: false
        blockAllReads: true
        printErrors: false
    }
}
