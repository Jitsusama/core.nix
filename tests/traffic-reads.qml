// Drives Traffic's answer through Quickshell itself, over the sandbox's own
// /proc/net/dev and a sysfs folder of its own for lo, counting the files it
// reads of lo to tell what it is. Each is a blocking read, and a step into
// Network asks Traffic after a gap every time, so an answer after a gap
// reads one file for an interface it already knows, its uevent, rather
// than all four again: four interfaces' sixteen reads took a median 6 ms
// and up to 49 ms on Joel's laptop (2026-11-17), of a step that has 50 ms
// for all of it. A build has no sysfs, so the folder is written here, in
// sys/lo beside the case, which whoever runs it makes first.
// It exits 0 once every case holds, and otherwise 1, saying which didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher
import "launcher/Interfaces.js" as Interfaces
import "Expect.js" as Expect

ShellRoot {
    id: root

    readonly property string sys: `${Quickshell.shellDir}/sys`

    Launcher.Traffic {
        id: traffic
        interfaces: root.sys
    }

    // One for each file, each at its own path for good, since a write to
    // one moved from path to path can land after the read that follows.
    FileView {
        id: uevent
        path: `${root.sys}/lo/uevent`
        blockWrites: true
        printErrors: false
    }
    FileView {
        id: type
        path: `${root.sys}/lo/type`
        blockWrites: true
        printErrors: false
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

    // lo's sysfs folder, as the kernel would write it for the facts given.
    function made(said, typed) {
        uevent.setText(said);
        uevent.waitForJob();
        type.setText(typed);
        type.waitForJob();
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

    // How many of lo's files an answer read.
    function reads() {
        const before = traffic.probed.lo ?? 0;
        Expect.expect("speed answers", traffic.answer("speed") !== null, "null");
        return (traffic.probed.lo ?? 0) - before;
    }

    // What lo is, as Traffic last told it.
    function kind() {
        return traffic.interfaceKinds.lo?.kind;
    }

    function asked() {
        const names = Object.keys(traffic.sample().last.all);
        Expect.expect("the sandbox counts lo", names.includes("lo"), JSON.stringify(names));
        made("INTERFACE=lo\nIFINDEX=1\n", "772\n");
        Expect.expect("an interface first seen is read whole, four files", reads() === 4,
            `${traffic.probed.lo}`);
        const loopback = Interfaces.sysKind("lo", { uevent: "IFINDEX=1", type: "772" });
        Expect.expect("it's what its files say", kind() === loopback, `${kind()}`);

        aged(60000);
        let read = reads();
        Expect.expect("after a gap, an interface already known is read once, its uevent",
            read === 1, `${read}`);
        read = reads();
        Expect.expect("straight after, nothing is read again", read === 0, `${read}`);

        // The same name made again as something else, as WARP's tunnel
        // is: its uevent names another index.
        made("INTERFACE=lo\nIFINDEX=99\n", "65534\n");
        aged(60000);
        read = reads();
        Expect.expect("an interface made again under its name is read whole again",
            read === 4, `${read}`);
        const bare = Interfaces.sysKind("lo", { uevent: "IFINDEX=99", type: "65534" });
        Expect.expect("the case makes it something else", bare !== loopback, `${bare}`);
        Expect.expect("an interface made again is what its files say now", kind() === bare,
            `${kind()} for ${bare}`);

        // A uevent that can't be read proves nothing, gap after gap.
        made("", "65534\n");
        for (const gap of [1, 2]) {
            aged(60000);
            read = reads();
            Expect.expect(`an unreadable uevent has all four read again, gap ${gap}`,
                read === 4, `${read}`);
        }
    }
}
