// Drives Traffic's answer through Quickshell itself, over the sandbox's own
// /proc/net/dev, counting the sysfs files it reads to tell what each
// interface is. Each is a blocking read, and a step into Network asks
// Traffic after a gap every time, so an answer after a gap reads one file
// for an interface it already knows, its uevent, rather than all four
// again: four interfaces' sixteen reads took a median 6 ms and up to 49 ms
// on Joel's laptop (2026-11-17), of a step that has 50 ms for all of it.
// It exits 0 once every case holds, and otherwise 1, saying which didn't.
import qs
import Quickshell
import QtQuick
import "launcher" as Launcher
import "Expect.js" as Expect

ShellRoot {
    id: root

    Launcher.Traffic {
        id: traffic
    }

    Component.onCompleted: Qt.callLater(root.begun)

    function begun() {
        try {
            asked();
            Qt.exit(0);
        } catch (error) {
            Expect.fail(error.message ?? error);
        }
    }

    // The reads kept as though the last was taken `ago` ms before now, as
    // a launcher opened again after a while finds them.
    function aged(ago) {
        const kept = traffic.reads;
        const back = Date.now() - ago - kept.last.at;
        traffic.reads = {
            last: { at: kept.last.at + back, all: kept.last.all },
            before: kept.before ? { at: kept.before.at + back, all: kept.before.all } : null
        };
    }

    function asked() {
        const names = Object.keys(traffic.sample().last.all);
        Expect.expect("the sandbox has an interface to count", names.length > 0,
            JSON.stringify(names));
        const first = traffic.probes;
        Expect.expect("speed answers", traffic.answer("speed") !== null, "null");
        // The shaper's mirrors are never asked what they are, so the count
        // is of those asked: four files each.
        const whole = traffic.probes - first;
        const known = whole / 4;
        Expect.expect("an interface first seen is read whole, four files each",
            whole > 0 && Number.isInteger(known), `${whole} for ${names.length} named`);
        aged(60000);
        const before = traffic.probes;
        Expect.expect("speed answers after a gap", traffic.answer("speed") !== null, "null");
        const again = traffic.probes - before;
        Expect.expect("after a gap, an interface already known is read once, its uevent",
            again === known, `${again} for ${known}`);
        const soon = traffic.probes;
        traffic.answer("speed");
        Expect.expect("straight after, nothing is read again", traffic.probes === soon,
            `${traffic.probes - soon}`);
        remade();
    }

    // One kept from an interface since gone, the same name made again as
    // something else, as WARP's tunnel is: its uevent names another index,
    // so it's read whole again and its kind is what sysfs says now.
    function remade() {
        const name = Object.keys(traffic.interfaceKinds)[0];
        const truly = traffic.interfaceKinds[name];
        const kinds = Object.assign({}, traffic.interfaceKinds);
        kinds[name] = { kind: "not this", uevent: `INTERFACE=${name}\nIFINDEX=0`, sure: false };
        traffic.interfaceKinds = kinds;
        const before = traffic.probes;
        traffic.answer("speed");
        Expect.expect("an interface made again under its name is read whole again",
            traffic.probes - before === 4, `${traffic.probes - before}`);
        Expect.expect("an interface made again is what sysfs says it is now",
            traffic.interfaceKinds[name].kind === truly.kind,
            `${traffic.interfaceKinds[name].kind} for ${truly.kind}`);
    }
}
