// Drives the launcher's Machine (modules/quickshell/launcher/Machine.qml)
// through Quickshell itself, over the sandbox's own /proc, with a stand-in
// for the program that lists the processes, which says each time it's run:
// the processors' and the memory's headlines, asked again as each second
// asks them, make none of their parts and start nothing, and only once
// their parts are looked at are the processes listed, once, for the
// programs using them. It exits 0 once every case holds, and otherwise 1,
// saying which case didn't.
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher

ShellRoot {
    id: root

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    Launcher.Machine {
        id: machine

        settings: ({ processes: Quickshell.env("PROCESSES") })
    }

    // How many times the processes were listed, a line each.
    FileView {
        id: listings
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/listed`
        blockLoading: true
        printErrors: false
    }

    function listed() {
        listings.reload();
        listings.waitForJob();
        return listings.text().split("\n").filter(line => line !== "").length;
    }

    // Quickshell hears Qt.exit only once the shell has loaded, so the cases
    // run just after.
    Component.onCompleted: Qt.callLater(root.headlines)

    function fail(error) {
        console.warn(`FAIL ${error.message ?? error}`);
        Qt.callLater(() => Qt.exit(1));
    }

    property var processors: null
    property var memory: null

    // The headlines, asked as the launcher asks them, before anything
    // looks at their rows.
    function headlines() {
        try {
            processors = machine.answer("cpu");
            memory = machine.answer("memory");
            expect("the processors' headline", processors !== null
                && /^\d+% busy$/.test(processors.headline), processors?.headline);
            expect("the memory's headline", memory !== null
                && / used, \d+%$/.test(memory.headline), memory?.headline);
            expect("both say they have parts", processors.parted && memory.parted,
                `${processors.parted} ${memory.parted}`);
            waited.start();
        } catch (error) {
            fail(error);
        }
    }

    // Long enough for a list started by the headlines to have been read,
    // and for each headline to be asked again, as each second would.
    Timer {
        id: waited
        interval: 1500
        onTriggered: {
            try {
                machine.answer("cpu");
                machine.answer("memory");
                root.expect("the headlines started nothing", root.listed() === 0
                    && machine.programs === null, `${root.listed()} listings`);
                const parts = root.processors.rows.map(row => row.name);
                root.expect("the processors' parts", parts.includes("Cores"),
                    parts.join(", "));
                root.expect("the memory's parts", root.memory.rows.length > 0, "none");
                partsShown.start();
            } catch (error) {
                root.fail(error);
            }
        }
    }

    // The parts looked at, the processes are listed, once for both.
    Timer {
        id: partsShown
        interval: 100
        repeat: true
        property int left: 50
        onTriggered: {
            if (machine.programs === null && --left > 0)
                return;
            stop();
            try {
                root.expect("the parts listed the processes", machine.programs !== null
                    && root.listed() === 1, `${root.listed()} listings`);
                Qt.callLater(() => Qt.exit(0));
            } catch (error) {
                root.fail(error);
            }
        }
    }
}
