pragma ComponentBehavior: Bound
// Drives the launcher's Emoji (modules/quickshell/launcher/Emoji.qml)
// through Quickshell itself with nothing at the clipboard history's socket,
// so every search it asks goes unanswered at once, as a history gone
// answers it. Its rows are drawn only while the launcher is open: shut, a
// search and a search typed inside a group that went unanswered are let go;
// open, each is kept to be asked again once the history says hello; and
// the launcher closing lets both go. So Emoji's rows follow the launcher's
// open, which the history's lost searches are kept by, rather than standing
// undrawn as a place that never sets them. It exits 0 once every case
// holds, and otherwise 1, saying which didn't.
import Quickshell
import QtQuick
import "Expect.js" as Expect
import "launcher" as Launcher

ShellRoot {
    id: root

    // A group's row, whose emoji what's typed inside it is searched among.
    readonly property string group: "row:emoji:group:smileys"

    // The launcher as Emoji reads it: shut at first, opened and shut by
    // the cases.
    QtObject {
        id: theLauncher

        property bool open: false
        property string typed: ""
        property var held: []
        function wants(source) {
            return true;
        }
        function afterFrame(then) {
            then();
        }
    }

    Launcher.Emoji {
        id: theEmoji

        launcher: theLauncher
        settings: ({ name: "emoji" })
    }

    // How the searches stand, for a failure to say.
    function said() {
        const served = theEmoji.served;
        return JSON.stringify({ drawn: served.drawn, lost: served.lost,
            typedLost: served.typedLost, state: served.searchState() });
    }

    // The cases run once ReadyRows' daemon is loaded, which its LazyLoader may do after the
    // shell starts, and nothing here waits forever: a daemon that never loads fails, by name.
    property bool begun: false
    Component.onCompleted: begin()
    Timer {
        running: !root.begun
        interval: 30000
        onTriggered: Expect.fail("never came: ReadyRows' daemon loaded")
    }
    Connections {
        target: theEmoji.served
        function onLinkChanged() {
            root.begin();
        }
    }

    function begin() {
        if (begun || !theEmoji.served.link)
            return;
        begun = true;
        Qt.callLater(root.cases);
    }

    function cases() {
        Expect.run(() => {
            const served = theEmoji.served;
            Expect.expect("shut, its rows aren't drawn", served.drawn === false, root.said());
            theEmoji.search("tu");
            theEmoji.typedInGroup(root.group, "sm");
            Expect.expect("shut, a search gone unanswered isn't kept, as its rows aren't drawn",
                served.lost === null && served.typedLost === null, root.said());

            theLauncher.open = true;
            Expect.expect("open, its rows are drawn", served.drawn === true, root.said());
            theEmoji.search("tur");
            Expect.expect("open, a search gone unanswered is kept to ask once the history's back",
                served.lost?.words === "tur", root.said());
            theEmoji.typedInGroup(root.group, "smi");
            Expect.expect("and so is one typed inside a group",
                served.typedLost?.within === root.group && served.typedLost?.words === "smi",
                root.said());

            theLauncher.open = false;
            Expect.expect("the launcher shut, its rows aren't drawn, and both are let go",
                served.drawn === false && served.lost === null && served.typedLost === null,
                root.said());
        });
    }
}
