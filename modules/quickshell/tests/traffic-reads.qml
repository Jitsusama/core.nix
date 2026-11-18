// Drives Traffic's answer through Quickshell itself, over the sandbox's own
// /proc/net/dev and a sysfs folder of its own for lo, counting the files it
// reads of lo to tell what it is. A step into Network asks Traffic after a
// gap every time, and a blocking read of each file took a median 6 to 14 ms
// and up to 133 under load on Joel's laptop (2026-11-17), of a step that
// has 50 ms for all of it. So an answer reads no file and waits on none: the
// counters are loaded as the launcher opens, and an interface's files are
// loaded in the background after, one uevent for an interface already known
// rather than all four again. A build has no sysfs, so the folder is written
// here, in sys/lo beside the case, which whoever runs it makes first.
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

    // The launcher, as much of it as Traffic reads: whether it's open, and
    // the answer shown, which is Traffic's own, so a changed kind is told.
    QtObject {
        id: shown

        property bool open: false
        property var held: []
        property var answer: ({ live: traffic })
    }

    Launcher.Traffic {
        id: traffic
        interfaces: root.sys
        launcher: shown
    }

    // How many times Traffic told the launcher to ask again, and how many
    // loads of the counters landed, since each tells while its answer is
    // shown: a tell past them is a kind's.
    property int told: 0
    property int loads: 0

    Connections {
        target: traffic

        function onAnswered() {
            root.told++;
        }

        function onReadsChanged() {
            root.loads++;
        }
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

    Until {
        id: waits
    }

    readonly property string loopback: Interfaces.sysKind("lo", { uevent: "IFINDEX=1", type: "772" })
    readonly property string bare: Interfaces.sysKind("lo", { uevent: "IFINDEX=99", type: "65534" })

    // lo's folder is written before Traffic's first load lands, since the
    // counters load in the background as it's made.
    Component.onCompleted: {
        made("INTERFACE=lo\nIFINDEX=1\n", "772\n");
        waits.until("lo known", () => idle() && traffic.interfaceKinds.lo?.sure, root.first);
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

    // Whether nothing is being asked of sysfs.
    function idle() {
        return traffic.probing === null && traffic.asking.length === 0;
    }

    // What lo is, as Traffic last told it.
    function kind() {
        return traffic.interfaceKinds.lo?.kind ?? "";
    }

    function probed() {
        return traffic.probed.lo ?? 0;
    }

    // An answer, which must read no file.
    function answered(what) {
        const before = probed();
        Expect.expect("speed answers", traffic.answer("speed") !== null, "null");
        Expect.expect(what, probed() === before && idle(), `${probed() - before} read`);
    }

    // After a gap, as the launcher opens: the counters load, and lo's
    // files are asked once they land.
    function reopened() {
        aged(60000);
        shown.open = false;
        shown.open = true;
    }

    property int mark: 0

    function first() {
        const names = Object.keys(traffic.reads.last.all);
        Expect.expect("the sandbox counts lo", names.includes("lo"), JSON.stringify(names));
        Expect.expect("an interface first seen is read whole, four files", probed() === 4,
            `${probed()}`);
        Expect.expect("it's what its files say", kind() === loopback, `${kind()}`);
        answered("an answer reads no file");

        reopened();
        answered("an answer as the launcher opens after a gap reads no file");
        mark = probed();
        waits.until("lo asked after the gap", () => probed() > mark && idle(), root.gapped);
    }

    function gapped() {
        const read = probed() - mark;
        Expect.expect("after a gap, an interface already known is read once, its uevent",
            read === 1, `${read}`);
        Expect.expect("it's sure of it again", traffic.interfaceKinds.lo.sure, "unsure");

        // The same name made again as something else, as WARP's tunnel
        // is: its uevent names another index.
        Expect.expect("the case makes it something else", bare !== loopback, `${bare}`);
        made("INTERFACE=lo\nIFINDEX=99\n", "65534\n");
        reopened();
        answered("an answer before the files land reads none");
        Expect.expect("until its files land, it's what it was", kind() === loopback, `${kind()}`);
        mark = probed();
        told = 0;
        loads = 0;
        waits.until("lo made again known", () => kind() === bare && idle(), root.remade);
    }

    function remade() {
        const read = probed() - mark;
        Expect.expect("an interface made again under its name is read whole again", read === 4,
            `${read}, kept ${JSON.stringify(traffic.interfaceKinds.lo)}`);
        Expect.expect("a kind changed tells the answer shown", told > loads,
            `${told} tells for ${loads} loads`);

        // A uevent that can't be read proves nothing, gap after gap.
        made("", "65534\n");
        unread(1);
    }

    function unread(gap) {
        reopened();
        mark = probed();
        waits.until(`lo asked after gap ${gap}`, () => probed() > mark && idle(), () => {
            const read = probed() - mark;
            Expect.expect(`an unreadable uevent has all four read again, gap ${gap}`, read === 4,
                `${read}`);
            if (gap < 2)
                unread(gap + 1);
            else
                Qt.exit(0);
        });
    }
}
