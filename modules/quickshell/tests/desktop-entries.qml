// Drives Quickshell's desktop entries
// (modules/quickshell/desktop-entries-heard-only-when-theirs.patch) through
// three changes: a file written beside the folders that hold the entries,
// as the home folder's hidden files and the recent files' list are; an entry
// added to one; and a link on the way to one pointed elsewhere, as a switch
// points the system's profile. The last two are heard, each bringing its
// entry. It exits 0 once every case holds, and otherwise 1, saying which
// didn't. How often the entries were read again is the check's to count,
// from Quickshell's log: once for each of the last two, never for the first.
import Quickshell
import Quickshell.Io
import QtQuick

ShellRoot {
    id: root

    // The folders, as checks.nix makes them: entries of the account's own
    // under data/applications, and the system's under the link's
    // share/applications.
    readonly property string home: Quickshell.env("HOME")
    readonly property string data: Quickshell.env("XDG_DATA_HOME")
    readonly property string link: Quickshell.env("ENTRIES_LINK")

    function has(id) {
        return DesktopEntries.applications.values.some(entry => entry.id === id);
    }

    function entry(name) {
        return `printf '[Desktop Entry]\\nType=Application\\nName=${name}\\nExec=true\\n'`;
    }

    readonly property var cases: [{
        what: "the entries there as the shell starts",
        does: null,
        holds: () => root.has("first") && root.has("system")
    }, {
        // Nothing to hear, so this waits out the monitor's 100 ms of quiet
        // four times over before it's taken as holding.
        what: "nothing read again for a file written beside the entries",
        does: () => `echo x > ${root.data}/.list.new && mv ${root.data}/.list.new ${root.data}/list`
            + ` && echo x > ${root.home}/.settings`,
        quiet: true,
        holds: () => root.has("first") && !root.has("added")
    }, {
        what: "an entry added heard",
        does: () => `${root.entry("Added")} > ${root.data}/applications/added.desktop`,
        holds: () => root.has("added")
    }, {
        what: "the system's entries heard once its link points elsewhere",
        does: () => `ln -sfn ${root.link}-next ${root.link}`,
        holds: () => root.has("next") && !root.has("system")
    }]

    property int at: 0
    property int left: 0

    Process {
        id: doing
        onExited: root.wait()
    }

    function wait() {
        left = cases[at].quiet ? 8 : 100;
        waiting.start();
    }

    function begin() {
        const step = cases[at];
        if (step.does === null) {
            wait();
        } else {
            doing.command = ["sh", "-c", step.does()];
            doing.running = true;
        }
    }

    // Quickshell reads the entries in the background, so each case waits for
    // what it holds, for five seconds at most; a quiet one waits its whole
    // span and then holds as it stands.
    Timer {
        id: waiting
        interval: 50
        repeat: true
        onTriggered: {
            const step = root.cases[root.at];
            const over = --root.left === 0;
            if (step.quiet && !over)
                return;
            if (step.holds()) {
                stop();
                console.log(`CASE ${step.what}`);
                if (++root.at === root.cases.length) {
                    console.log("PASS every case holds");
                    Qt.exit(0);
                } else {
                    root.begin();
                }
            } else if (over) {
                stop();
                const ids = DesktopEntries.applications.values.map(entry => entry.id);
                console.warn(`FAIL ${step.what}: the entries are ${JSON.stringify(ids)}`);
                Qt.exit(1);
            }
        }
    }

    Component.onCompleted: begin()
}
