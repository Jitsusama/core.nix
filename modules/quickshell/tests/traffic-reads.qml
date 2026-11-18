// Drives Traffic's answer through Quickshell itself, over counters, a
// shaper's status, an uptime and a sysfs folder for lo of its own, counting
// the files it loads of lo to tell what it is. A step into Network asks
// Traffic after a gap every time, and a blocking read of each file took a
// median 6 to 14 ms and up to 133 under load on Joel's laptop (2026-11-17),
// of a step that has 50 ms for all of it. So an answer reads no file and
// waits on none: what it says is what was last loaded, though the files
// have changed since, and an interface's files are loaded in the
// background after a load, one uevent for an interface already known
// rather than all four again. A build has no sysfs, so the files are
// written here, in sys beside the case, whose lo folder whoever runs it
// makes first. It exits 0 once every case holds, and otherwise 1, saying
// which didn't.
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
        counterFile: `${root.sys}/dev`
        statusFile: `${root.sys}/status.json`
        uptimeFile: `${root.sys}/uptime`
        launcher: shown
    }

    // How many times Traffic told the launcher to ask again, and how many
    // loads of the counters landed, since each tells while its answer is
    // shown: a tell past them is a kind's. What lo was as the first load
    // after it's cleared landed, before its files could.
    property int told: 0
    property int loads: 0
    property var atLoad: null

    Connections {
        target: traffic

        function onAnswered() {
            root.told++;
        }

        function onReadsChanged() {
            root.loads++;
            if (root.atLoad === null)
                root.atLoad = root.kind();
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
    FileView {
        id: dev
        path: `${root.sys}/dev`
        blockWrites: true
        printErrors: false
    }
    FileView {
        id: status
        path: `${root.sys}/status.json`
        blockWrites: true
        printErrors: false
    }
    FileView {
        id: uptime
        path: `${root.sys}/uptime`
        blockWrites: true
        printErrors: false
    }

    // A uevent the kernel won't give, gone rather than empty.
    Process {
        id: unmade
        command: ["rm", "-f", `${root.sys}/lo/uevent`]
        onExited: root.unread(2)
    }

    Until {
        id: waits
    }

    readonly property string loopback: Interfaces.sysKind("lo", { uevent: "IFINDEX=1", type: "772" })
    readonly property string bare: Interfaces.sysKind("lo", { uevent: "IFINDEX=99", type: "65534" })

    Component.onCompleted: {
        counted({ lo: [1000, 2000] });
        write(uptime, "1000.00 4000.00\n");
        made("INTERFACE=lo\nIFINDEX=1\n", "772\n");
        traffic.look();
        waits.until("lo known", () => idle() && traffic.interfaceKinds.lo?.sure, root.first);
    }

    function write(file, text) {
        file.setText(text);
        file.waitForJob();
    }

    // lo's sysfs folder, as the kernel would write it for the facts given.
    function made(said, typed) {
        write(uevent, said);
        write(type, typed);
    }

    // The counters, as /proc/net/dev says them: each interface's bytes in
    // and out, nothing dropped.
    function counted(each) {
        write(dev, Object.keys(each).map(name => `${name}: ${each[name][0]} 10 0 0 0 0 0 0 `
            + `${each[name][1]} 20 0 0 0 0 0 0`).join("\n") + "\n");
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

    // What an answer says: its headline, each row's figure and note, and
    // what its drawer says has come in since startup.
    function said() {
        const speed = traffic.answer("speed");
        return JSON.stringify([speed.headline, speed.note,
            speed.rows.map(row => `${row.name}=${row.side} ${row.note}`),
            speed.drawer().facts.map(fact => `${fact.label}=${fact.value}`)]);
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
        Expect.expect("the case's counters count lo", names.includes("lo"), JSON.stringify(names));
        Expect.expect("an interface first seen is read whole, four files", probed() === 4,
            `${probed()}`);
        Expect.expect("it's what its files say", kind() === loopback, `${kind()}`);
        answered("an answer reads no file");

        // A card beside lo, as though its files said so, so the answer
        // counts it, and the shaper's measure of it.
        counted({ lo: [1000, 2000], eth0: [1000000, 2000000] });
        write(status, JSON.stringify([{ name: "eth0", ifb: "ifb0", down: 100000, up: 20000,
            state: "climbing", baselines: [["one", 12]] }]));
        traffic.look();
        waits.until("eth0 counted", () => traffic.reads.last.all.eth0 !== undefined && idle(),
            () => {
                traffic.interfaceKinds = Object.assign({}, traffic.interfaceKinds,
                    { eth0: { kind: "wired", uevent: "", sure: true } });
                waits.until("the shaper's measure loaded", () => traffic.answer("speed").rows
                    .some(row => row.name === Words.name("traffic:line")), root.loaded);
            });
    }

    // Each file changed under it, an answer says what was last loaded.
    function loaded() {
        const was = said();
        counted({ lo: [1000, 2000], eth0: [9000000000, 9000000000] });
        write(status, JSON.stringify([{ name: "eth0", ifb: "ifb0", down: 1, up: 1,
            state: "holding", baselines: [["one", 99]] }]));
        Expect.expect("an answer says the counters and the shaper's status as last loaded",
            said() === was, `${said()} after ${was}`);
        const kept = traffic.reads;
        traffic.reads = { last: kept.last, before: null };
        const started = said();
        write(uptime, "1.00 1.00\n");
        Expect.expect("a first read's rate since startup says the uptime as last loaded",
            said() === started, `${said()} after ${started}`);
        traffic.reads = kept;

        counted({ lo: [1000, 2000] });
        traffic.look();
        waits.until("eth0 gone", () => traffic.reads.last.all.eth0 === undefined && idle(),
            root.gapping);
    }

    function gapping() {
        reopened();
        answered("an answer as the launcher opens after a gap reads no file");
        Expect.expect("an answer with nothing loaded lately says it's from when last read",
            traffic.answer("speed").note === "when last read" && traffic.stillReading,
            `${traffic.answer("speed").note}, reading ${traffic.stillReading}`);
        mark = probed();
        waits.until("lo asked after the gap", () => probed() > mark && idle(), root.gapped);
    }

    function gapped() {
        const read = probed() - mark;
        Expect.expect("after a gap, an interface already known is read once, its uevent",
            read === 1, `${read}`);
        Expect.expect("it's sure of it again", traffic.interfaceKinds.lo.sure, "unsure");
        Expect.expect("once loaded again, it's reading no more", !traffic.stillReading,
            "reading");
        mark = probed();
        const landed = loads;
        traffic.look();
        waits.until("a load straight after", () => loads > landed && idle(), root.straight);
    }

    // A load with no gap asks nothing again, and a second gap in a row
    // asks the same file again, once.
    function straight() {
        Expect.expect("a load straight after reads nothing again", probed() === mark,
            `${probed() - mark}`);
        reopened();
        waits.until("lo asked after a second gap", () => probed() > mark && idle(), () => {
            Expect.expect("after a second gap in a row, it's read once again",
                probed() - mark === 1, `${probed() - mark}`);
            remaking();
        });
    }

    // The same name made again as something else, as WARP's tunnel is: its
    // uevent names another index.
    function remaking() {
        Expect.expect("the case makes it something else", bare !== loopback, `${bare}`);
        made("INTERFACE=lo\nIFINDEX=99\n", "65534\n");
        reopened();
        atLoad = null;
        answered("an answer before the files land reads none");
        mark = probed();
        told = 0;
        loads = 0;
        waits.until("lo made again known", () => kind() === bare && idle(), root.remade);
    }

    function remade() {
        const read = probed() - mark;
        Expect.expect("until its files land, it's what it was", atLoad === loopback, `${atLoad}`);
        Expect.expect("an interface made again under its name is read whole again", read === 4,
            `${read}, kept ${JSON.stringify(traffic.interfaceKinds.lo)}`);
        Expect.expect("a kind changed tells the answer shown", told > loads,
            `${told} tells for ${loads} loads`);

        // A uevent that says nothing proves nothing, and nor does one gone.
        made("", "65534\n");
        unread(1);
    }

    function unread(gap) {
        reopened();
        mark = probed();
        waits.until(`lo asked after gap ${gap}`, () => probed() > mark && idle(), () => {
            const read = probed() - mark;
            Expect.expect(`a uevent ${gap < 2 ? "empty" : "gone"} has all four read again`,
                read === 4, `${read}`);
            Expect.expect(`and it's what the others say, gap ${gap}`, kind() === bare, kind());
            if (gap < 2)
                unmade.running = true;
            else
                Qt.exit(0);
        });
    }
}
