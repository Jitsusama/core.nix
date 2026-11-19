// Drives Traffic's answer through Quickshell itself, over counters, a
// shaper's status, an uptime and a sysfs folder for lo of its own, counting
// the files it loads of lo to tell what it is. A step into Network asks
// Traffic after a gap every time, and a blocking read of each file took a
// median 6 to 14 ms and up to 133 under load on Joel's laptop (2026-11-17),
// of a step that has 50 ms for all of it. So an answer reads no file and
// waits on none: what it says is what was last loaded, though the files
// have changed since, and an interface's files are loaded in the
// background after a load, one uevent for an interface already known
// rather than all four again. A file that never lands is given up, and
// its interface isn't asked again until it does and its hold is over, a
// minute and then twice as long, so one read that hangs holds one of the
// pool's threads, not one more each load, and a slow one isn't asked each
// second. A build has no sysfs, so the files are written here, in sys
// beside the case, whose lo folder whoever runs it makes first; an
// interface with no folder reads as virtual. It exits 0 once every case
// holds, and otherwise 1, saying which didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher
import "launcher/Interfaces.js" as Interfaces
import "Expect.js" as Expect
import "Written.js" as Written

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
        counterFile: ""
        statusFile: `${root.sys}/status.json`
        launcher: shown
    }

    // How many times Traffic told the launcher to ask again, and how many
    // loads of the counters landed, since each tells while its answer is
    // shown: a tell past them is a kind's. What lo was as the first load
    // after it's cleared landed, and as its files were first asked, before
    // they could land; and what eth0 was as the load that first saw it
    // landed.
    property int told: 0
    property int loads: 0
    property var atLoad: null
    property var atProbe: null
    property var firstSeen: null
    property var firstProbed: null
    // Where the reads stood as Traffic last stopped reading, and how long
    // each read's rate is over after each change, null with no read before.
    property var ended: null
    property var spans: []

    Connections {
        target: traffic

        function onAnswered() {
            root.told++;
        }

        function onReadsChanged() {
            root.spans.push(traffic.reads.before === null ? null
                : traffic.reads.last.at - traffic.reads.before.at);
            root.loads++;
            if (root.atLoad === null)
                root.atLoad = root.kind();
            if (root.firstSeen === null && traffic.reads?.last.all.eth0 !== undefined)
                root.firstSeen = traffic.interfaceKind("eth0");
        }

        function onStillReadingChanged() {
            if (!traffic.stillReading)
                root.ended = { settled: traffic.settled(traffic.reads), idle: root.idle(),
                    over: traffic.reads.last.at - (traffic.reads.before?.at ?? 0) };
        }

        function onProbingChanged() {
            if (root.atProbe === null && traffic.probing?.name === "lo")
                root.atProbe = root.kind();
            if (root.firstProbed === null && traffic.probing?.name === "eth0")
                root.firstProbed = traffic.interfaceKind("eth0");
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
        Written.written(uptime, "1000.00 4000.00\n");
        // Traffic loads the uptime and the counters as it's pointed at
        // them, which it only is once there are some to load.
        traffic.uptimeFile = `${root.sys}/uptime`;
        made("INTERFACE=lo\nIFINDEX=1\n", "772\n");
        traffic.counterFile = `${root.sys}/dev`;
        waits.until("lo known", () => idle() && traffic.interfaceKinds.lo?.sure, root.first);
    }

    // lo's sysfs folder, as the kernel would write it for the facts given.
    function made(said, typed) {
        Written.written(uevent, said);
        Written.written(type, typed);
    }

    // The counters, as /proc/net/dev says them: each interface's bytes in
    // and out, nothing dropped.
    function counted(each) {
        Written.written(dev, Object.keys(each)
            .map(name => `${name}: ${each[name][0]} 10 0 0 0 0 0 0 `
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

    // Loads again, past the tenth of a second after the last load landed
    // in which Traffic loads nothing.
    function lookAgain() {
        if (traffic.reads !== null)
            aged(traffic.soonest);
        traffic.look();
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
        Written.written(status, JSON.stringify([{ name: "eth0", ifb: "ifb0", down: 100000,
            up: 20000,
            state: "resting", baselines: [["one", 12]] }]));
        lookAgain();
        waits.until("eth0 counted", () => traffic.reads.last.all.eth0 !== undefined && idle(),
            () => {
                Expect.expect("an interface first seen is left out until its files land",
                    firstSeen === "" && firstProbed === "", `${firstSeen} then ${firstProbed}`);
                traffic.interfaceKinds = Object.assign({}, traffic.interfaceKinds,
                    { eth0: { kind: "wired", uevent: "", sure: true } });
                waits.until("the shaper's measure loaded", () => traffic.answer("speed").rows
                    .some(row => row.name === Words.name("traffic:line")), root.loaded);
            });
    }

    // Each file changed under it, an answer says what was last loaded.
    function loaded() {
        Expect.expect("a line resting is said as measured", lineNote() === "as measured",
            lineNote());
        const resting = lineRow().side;
        const was = said();
        counted({ lo: [1000, 2000], eth0: [9000000000, 9000000000] });
        Written.written(status, JSON.stringify([{ name: "eth0", ifb: "ifb0", down: 1, up: 1,
            state: "holding", baselines: [["one", 99]] }]));
        Expect.expect("an answer says the counters and the shaper's status as last loaded",
            said() === was, `${said()} after ${was}`);
        const kept = traffic.reads;
        traffic.reads = { last: kept.last, before: null };
        const started = said();
        // eth0's 1,000,000 bytes in and 2,000,000 out, over the 1000 s
        // the uptime said as it loaded.
        const sinceStart = traffic.said("ways", { in: traffic.bits(1000),
            out: traffic.bits(2000) });
        Expect.expect("a first read's rate is over the uptime loaded",
            traffic.answer("speed").headline === sinceStart,
            `${traffic.answer("speed").headline}, not ${sinceStart}`);
        Written.written(uptime, "1.00 1.00\n");
        Expect.expect("a first read's rate since startup says the uptime as last loaded",
            said() === started, `${said()} after ${started}`);
        traffic.reads = kept;

        // A look asked while a load is under way is passed over, and one
        // Traffic began itself, as its first load's follow-up does, may have
        // read the status before holding was written. So the look waits for
        // no load to be under way, and then for the counters' load to land
        // too, since the status may land first. Aged first, since aging
        // counts as a load. The note is claimed once the row says holding's
        // rates, whatever its note, so a note lost or reworded says what it
        // was, and a row gone as the status loads again isn't taken for one.
        waits.until("no load under way", () => traffic.loadAsked === 0, () => {
            aged(traffic.soonest);
            const landed = loads;
            traffic.look();
            waits.until("the holding status loaded", () => loads > landed && idle()
                && lineRow() !== undefined && lineRow().side !== resting, root.heldOn);
        });
    }

    // The line holding, said so; then in a state Traffic doesn't know,
    // named as something of an object's own, said as measured; and the
    // card gone.
    function heldOn() {
        Expect.expect("a line holding is said so",
            lineNote() === "as measured, held to the shaper's limit", lineNote());
        Written.written(status, JSON.stringify([{ name: "eth0", ifb: "ifb0", down: 1, up: 1,
            state: "constructor", baselines: [["one", 99]] }]));
        waits.until("no load under way after holding", () => traffic.loadAsked === 0, () => {
            aged(traffic.soonest);
            traffic.look();
            waits.until("a line in a state not known, whatever it's called, is said as measured",
                () => idle() && traffic.loadAsked === 0 && lineNote() === "as measured", () => {
                counted({ lo: [1000, 2000] });
                lookAgain();
                waits.until("eth0 gone", () => traffic.reads.last.all.eth0 === undefined
                    && idle(), root.gapping);
            });
        });
    }

    // The shaper's line row, or undefined with none.
    function lineRow() {
        return traffic.answer("speed").rows.find(row => row.name
            === Words.name("traffic:line"));
    }

    // What the shaper's line row notes of what it's doing with the line, or
    // undefined with no row.
    function lineNote() {
        return lineRow()?.note;
    }

    // Asked after a gap with the launcher closed, as a terminal's `launcher
    // show` asks, so only the answer itself can start a load.
    function gapping() {
        aged(60000);
        shown.open = false;
        const landed = loads;
        answered("an answer after a gap reads no file");
        Expect.expect("an answer with nothing loaded lately says it's from when last read",
            traffic.answer("speed").note === "when last read" && traffic.stillReading,
            `${traffic.answer("speed").note}, reading ${traffic.stillReading}`);
        mark = probed();
        waits.until("lo asked after the gap", () => loads > landed && probed() > mark && idle(),
            root.gapped);
    }

    function gapped() {
        const read = probed() - mark;
        Expect.expect("after a gap, an interface already known is read once, its uevent",
            read === 1, `${read}`);
        Expect.expect("it's sure of it again", traffic.interfaceKinds.lo.sure, "unsure");
        // The first load after a gap is a rate over the gap, so it reads
        // on until a rate of now lands, over a tenth of a second at least.
        waits.until("a rate of now", () => !traffic.stillReading, () => {
            Expect.expect("it reads until a rate of now has landed",
                ended.settled && ended.over >= traffic.soonest / 2, JSON.stringify(ended));
            Expect.expect("once they have, it says so",
                traffic.answer("speed").note === "just now", traffic.answer("speed").note);
            // As though a load had just landed: one asked now loads nothing.
            aged(0);
            mark = probed();
            const landed = loads;
            traffic.look();
            waits.until("a moment on", () => Date.now() - traffic.reads.last.at > traffic.soonest,
                () => {
                    Expect.expect("a load asked just after one landed loads nothing",
                        loads === landed, `${loads - landed} loaded`);
                    traffic.look();
                    waits.until("a load straight after", () => loads > landed && idle(),
                        root.straight);
                });
        });
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
        atProbe = null;
        answered("an answer before the files land reads none");
        mark = probed();
        told = 0;
        loads = 0;
        waits.until("lo made again known", () => kind() === bare && idle(), root.remade);
    }

    function remade() {
        const read = probed() - mark;
        Expect.expect("until its files land, it's what it was",
            atLoad === loopback && atProbe === loopback, `${atLoad} then ${atProbe}`);
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
                steppedBack();
        });
    }

    // The clock stepped back past the last load, which is stamped ahead of
    // now: it's old, and the read after it makes no rate over it.
    function steppedBack() {
        shown.open = false;
        aged(-60000);
        const landed = loads;
        const next = spans.length;
        Expect.expect("a load stamped ahead of now is old",
            traffic.answer("speed").note === "when last read" && traffic.stillReading,
            `${traffic.answer("speed").note}, reading ${traffic.stillReading}`);
        waits.until("a rate of now after the clock stepped back",
            () => loads > landed && !traffic.stillReading, () => {
                Expect.expect("the read after it makes no rate over it", spans[next] === null,
                    `${spans[next]} ms`);
                unloadable();
            });
    }

    // Counters that can't be loaded leave nothing to wait for, so a
    // terminal asking after a gap isn't kept asking.
    function unloadable() {
        shown.open = false;
        traffic.counterFile = `${root.sys}/none`;
        aged(60000);
        traffic.answer("speed");
        Expect.expect("asked after a gap, it's reading", traffic.stillReading, "not reading");
        waits.until("a load that fails ends the reading", () => !traffic.stillReading, () => {
            traffic.answer("speed");
            Expect.expect("asked again, it reads on no more", !traffic.stillReading, "reading");
            waits.until("no probe under way", () => traffic.probing === null
                && !traffic.probeWatching, root.hung);
        });
    }

    // The interface the probe is under way with, and those it'll ask next.
    function asked() {
        return JSON.stringify({ probing: traffic.probing?.name ?? null,
            asking: traffic.asking.map(one => one.name) });
    }

    // Whether hung()'s view, its read landed late, is gone.
    property bool hungGone: false

    // A file in flight that never lands is given up by the probe's watch,
    // about two seconds on, its view kept and a new one made, its interface
    // held off a minute, and not asked again until that read lands, its
    // hold over or not; landing late, the view goes, the interface under
    // way isn't credited with it, and its hold still holds it off, as a
    // file slow to land is. The sandbox can't hang a read, so the view
    // asked tells no one it landed, as one hung never would.
    function hung() {
        const reading = Qt.createQmlObject("import QtQml; import Quickshell.Io; FileView {"
            + " Component.onDestruction: root.hungGone = true }", root);
        traffic.probe = reading;
        traffic.probing = { name: "eth9", files: ["uevent"], said: {} };
        traffic.probeFile();
        const began = Date.now();
        waits.until("the watch gives up a file that never lands",
            () => traffic.probesHung.eth9 !== undefined, () => {
            const after = Date.now() - began;
            Expect.expect("the watch gives up a file about two seconds on",
                after >= 1500 && after <= 8000, `${after} ms`);
            Expect.expect("the watch keeps the view in flight and makes another",
                traffic.probesHung.eth9 === reading && traffic.probe !== reading,
                JSON.stringify(Object.keys(traffic.probesHung)));
            Expect.expect("an interface given up is held off a minute",
                traffic.probesStuck.eth9?.wait === 60000, JSON.stringify(traffic.probesStuck));
            traffic.probesStuck.eth9.at -= 61000;
            traffic.probeFor(["eth9"]);
            Expect.expect("an interface whose read hangs isn't asked again",
                traffic.probing?.name !== "eth9"
                    && !traffic.asking.some(one => one.name === "eth9"), root.asked());
            const under = { name: "lo", files: ["uevent"], said: {}, inFlight: true };
            traffic.probing = under;
            traffic.probeLanded(reading, "DEVTYPE=late");
            Expect.expect("a hung read landing lets its view go and isn't taken for the file"
                + " under way", traffic.probesHung.eth9 === undefined
                && under.said.uevent === undefined, JSON.stringify(under.said));
            waits.until("a hung read's view, landed, is destroyed", () => root.hungGone, () => {
                traffic.probing = null;
                traffic.probesStuck.eth9.at = Date.now();
                traffic.probeFor(["eth9"]);
                Expect.expect("an interface whose read landed late is held off, not asked"
                    + " again at once", traffic.probing?.name !== "eth9"
                    && !traffic.asking.some(one => one.name === "eth9"), root.asked());
                traffic.probesStuck.eth9.at -= 61000;
                traffic.probeFor(["eth9"]);
                Expect.expect("an interface whose hung read landed is asked again",
                    traffic.probing?.name === "eth9"
                        || traffic.asking.some(one => one.name === "eth9"), root.asked());
                waits.until("no probe under way before a late read", () => traffic.probing === null
                    && !traffic.probeWatching, root.lateRead);
            });
        });
    }

    // Traffic's own view, given up in the turn its read began, so the read
    // is still under way: the interface after is asked through a new view,
    // and the read landing late reaches Traffic's own handlers, which let
    // its interface go. lo's uevent, gone since the gaps, is written again,
    // so the late read lands with text through onLoaded; whether a late
    // read is credited is hung's to show, since here it lands before or
    // after the next interface is done as the pool runs it.
    property var lateView: null
    // What the view given up said as it landed, or null before then.
    property var lateText: null
    Connections {
        target: root.lateView
        function onLoaded() {
            root.lateText = root.lateView.text().trim();
        }
        function onLoadFailed() {
            root.lateText = "";
        }
    }
    function lateRead() {
        Written.written(uevent, "DEVTYPE=late\n");
        traffic.asking = [{ name: "eth8", files: ["uevent"], said: {} }];
        traffic.probing = { name: "lo", files: ["uevent"], said: {} };
        traffic.probeFile();
        root.lateView = traffic.probe;
        traffic.probeGivenUp();
        const next = traffic.probing;
        Expect.expect("given up in flight, the next interface is asked through a new view",
            next?.name === "eth8" && traffic.probe !== root.lateView
                && traffic.probesHung.lo === root.lateView, root.asked());
        waits.until("a read given up lands late through Traffic's own view",
            () => traffic.probesHung.lo === undefined && root.lateText !== null
                && next.done === true, () => {
            Expect.expect("a read given up lands late with its text",
                root.lateText === "DEVTYPE=late", JSON.stringify(root.lateText));
            waits.until("the view a late read landed through is destroyed",
                () => root.lateView === null, root.lateFailed);
        });
    }

    // A read given up that ends failing, as a wedged driver's may once it
    // stalls, reaches Traffic's own handlers too: eth7 has no folder.
    // Given up again, it's held off twice as long.
    function lateFailed() {
        root.lateText = null;
        traffic.asking = [];
        traffic.probing = { name: "eth7", files: ["uevent"], said: {} };
        traffic.probeFile();
        root.lateView = traffic.probe;
        traffic.probeGivenUp();
        Expect.expect("a read given up as it begins is kept",
            traffic.probesHung.eth7 === root.lateView, root.asked());
        waits.until("a read given up that fails lands late through Traffic's own view",
            () => traffic.probesHung.eth7 === undefined && root.lateText === "", () => {
            waits.until("the view a failed late read landed through is destroyed",
                () => root.lateView === null, () => {
                traffic.probing = { name: "eth7", files: ["uevent"], said: {}, inFlight: false };
                traffic.probeGivenUp();
                Expect.expect("an interface given up again is held off twice as long",
                    traffic.probesStuck.eth7?.wait === 120000, JSON.stringify(traffic.probesStuck));
                root.twoHung();
            });
        });
    }

    // Two interfaces hung at once are each kept by their own name, so one
    // whose read lands lets only its own go.
    function twoHung() {
        const views = ["eth5", "eth6"].map(name => {
            const view = Qt.createQmlObject("import QtQml; import Quickshell.Io; FileView {}",
                root);
            traffic.probe = view;
            traffic.probing = { name: name, files: ["uevent"], said: {}, inFlight: true };
            traffic.probeGivenUp();
            return view;
        });
        Expect.expect("two interfaces hung are both kept",
            traffic.probesHung.eth5 === views[0] && traffic.probesHung.eth6 === views[1],
            JSON.stringify(Object.keys(traffic.probesHung)));
        traffic.probeLanded(views[0], "");
        for (const name of ["eth5", "eth6"])
            traffic.probesStuck[name].at -= 61000;
        traffic.probeFor(["eth5", "eth6"]);
        const asks = name => traffic.probing?.name === name
            || traffic.asking.some(one => one.name === name);
        Expect.expect("of two hung, the one landed is asked again and the other still kept",
            traffic.probesHung.eth5 === undefined && traffic.probesHung.eth6 === views[1]
                && asks("eth5") && !asks("eth6"), root.asked());
        traffic.probeLanded(views[1], "");
        waits.until("no probe under way after two hung", () => traffic.probing === null
            && !traffic.probeWatching, root.notInFlight);
    }

    // An interface whose files land in time has no read in flight and lets
    // its hold go, lo's from its late read; a file isn't asked while
    // another is in flight; and a turn given up with none in flight, as
    // one that threw after its file landed, keeps no view but is held off.
    function notInFlight() {
        Expect.expect("lo is held off from its late read", traffic.probesStuck.lo !== undefined,
            JSON.stringify(traffic.probesStuck));
        const one = { name: "lo", files: ["uevent"], said: {} };
        traffic.probing = one;
        traffic.probeFile();
        waits.until("lo's uevent landed", () => one.done === true, () => {
            Expect.expect("an interface whose files have landed has none in flight",
                one.inFlight === false, String(one.inFlight));
            Expect.expect("an interface whose files land in time lets its hold go",
                traffic.probesStuck.lo === undefined, JSON.stringify(traffic.probesStuck));
            const before = traffic.probe;
            const path = String(before.path);
            const count = traffic.probed.lo;
            traffic.probing = { name: "lo", files: ["uevent"], said: {}, inFlight: true };
            traffic.probeFile();
            Expect.expect("a file isn't asked while another is in flight",
                traffic.probed.lo === count && String(traffic.probe.path) === path,
                `${traffic.probed.lo} loads, at ${traffic.probe.path}`);
            traffic.probing = { name: "lo", files: ["type"], said: { uevent: "DEVTYPE=late" },
                inFlight: false };
            traffic.probeGivenUp();
            Expect.expect("given up with no read in flight, no view is kept, but it's held off",
                traffic.probesHung.lo === undefined && traffic.probe === before
                    && traffic.probesStuck.lo?.wait === 60000,
                JSON.stringify(Object.keys(traffic.probesHung)));
            root.heldThroughUevent();
        });
    }

    // A hold goes only once every file of its interface has landed: lo,
    // held, has a uevent land that isn't the one it was read from, so its
    // other files are still to come, and given up before they land, it's
    // held twice as long rather than a minute afresh.
    function heldThroughUevent() {
        const held = traffic.probesStuck.lo;
        const one = { name: "lo", files: ["uevent"], said: {} };
        traffic.probing = one;
        traffic.probeSaid("DEVTYPE=other");
        Expect.expect("an interface held whose uevent lands, its other files to come, is held",
            traffic.probesStuck.lo === held && one.files.length === 3,
            `${JSON.stringify(traffic.probesStuck.lo)}, ${one.files.length} to come`);
        traffic.probeGivenUp();
        Expect.expect("given up before its other files land, it's held twice as long",
            traffic.probesStuck.lo?.wait === 120000, JSON.stringify(traffic.probesStuck));
        waits.until("no probe under way before odd names", () => traffic.probing === null
            && !traffic.probeWatching, root.oddNamed);
    }

    // Interfaces named as something of an object's own, counted beside lo,
    // are each asked, known and described as any other, since no map by an
    // interface's name has a prototype. lo, held, isn't asked.
    readonly property var oddNames: ["constructor", "__proto__"]

    // A map's entries for the odd names, as [name, ...what each says].
    function oddOf(map, says) {
        return JSON.stringify(Object.entries(map).filter(([name]) => oddNames.includes(name))
            .map(([name, value]) => [name].concat(says(value))));
    }

    function oddNamed() {
        const each = Object.create(null);
        each.lo = [1000, 2000];
        each[oddNames[0]] = [5, 6];
        each[oddNames[1]] = [7, 8];
        counted(each);
        const landed = loads;
        mark = probed();
        traffic.counterFile = `${root.sys}/dev`;
        waits.until("odd names counted and asked", () => loads > landed && idle(), () => {
            Expect.expect("an interface held isn't asked as a load wants it", probed() === mark,
                `${probed() - mark} read`);
            const read = oddOf(traffic.probed, count => [count]);
            Expect.expect("interfaces named as an object's own are each read whole",
                read === '[["constructor",4],["__proto__",4]]', read);
            const kinds = oddOf(traffic.interfaceKinds, kind => [kind.kind, kind.sure]);
            Expect.expect("and each is what its files say",
                kinds === '[["constructor","virtual",true],["__proto__","virtual",true]]', kinds);
            const described = oddOf(traffic.described(traffic.reads.last.all, []),
                one => [one.kind, one.counters.in]);
            Expect.expect("and each is described beside its counters",
                described === '[["constructor","virtual",5],["__proto__","virtual",7]]',
                described);
            Qt.exit(0);
        });
    }
}
