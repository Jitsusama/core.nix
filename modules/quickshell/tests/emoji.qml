pragma ComponentBehavior: Bound
// Drives the launcher's Emoji (modules/quickshell/launcher/Emoji.qml)
// through Quickshell itself with nothing at the clipboard history's socket,
// so every search it asks goes unanswered at once, as a history gone
// answers it. Its rows are drawn only while the launcher is open: shut, a
// search and a search typed inside a group that went unanswered are let go;
// open, each is kept to be asked again once the history says hello; and
// the launcher closing lets both go. So Emoji's rows follow the launcher's
// open, which the history's lost searches are kept by, rather than standing
// undrawn as a place that never sets them. Lost again and the history back
// a while on, each is asked again at its hello, and the answer to what was
// typed in the group isn't taken as its key's own (foundMore's fresh), so
// Emoji keeps its rows where they are. An emoji's row is keyed by its
// thing, kept under `row:` where that claims the launcher's own kind, and
// by its key where it has none. It exits 0 once every case holds, and
// otherwise 1, saying which didn't.
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
        return JSON.stringify({ drawn: theEmoji.served.drawn, state: theEmoji.searchState() });
    }

    // Whether each answer Emoji told the launcher of was taken as its key's own.
    property var fresh: []
    Connections {
        target: theEmoji
        function onFoundMore(itsOwn) {
            root.fresh = root.fresh.concat([itsOwn]);
        }
    }

    // The cases run once ReadyRows' daemon is loaded, which its LazyLoader may do after the
    // shell starts, and nothing here waits forever: a daemon that never loads, or a hello
    // stage never reached, fails, by name.
    property bool begun: false
    property bool welcomed: false
    Component.onCompleted: begin()
    Timer {
        running: !root.welcomed
        interval: 30000
        onTriggered: Expect.fail(root.begun ? "never came: the history's hello"
            : "never came: ReadyRows' daemon loaded")
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

    // The cases before the history comes back, which go on to those after it (helloCases)
    // rather than exiting, as Expect.run would once they return.
    function cases() {
        try {
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

            // Typed inside the group first, then searched at the top, as a step out does.
            theLauncher.open = true;
            theEmoji.typedInGroup(root.group, "smil");
            theEmoji.search("turt");
            Expect.expect("open again, both lost are kept, the typed one past a step out",
                served.lost?.words === "turt" && served.typedLost?.words === "smil", root.said());
        } catch (error) {
            Expect.fail(Expect.thrown(error));
            return;
        }
        backAgain.start();
    }

    // The history back a while after the keys, as a restart takes, past the 50 ms Emoji takes
    // an answer as its key's own in (freshFor).
    Timer {
        id: backAgain
        interval: 100
        onTriggered: root.helloCases()
    }

    function helloCases() {
        welcomed = true;
        Expect.run(() => {
            const served = theEmoji.served;
            const link = served.link;
            link.wire = { write: line => {}, flush: () => {} };
            const before = link.asked;
            link.said(JSON.stringify({ id: "hello" }));
            Expect.expect("the history back, each is asked again at its hello",
                link.asked === before + 2 && served.asking === "turt"
                    && served.typedAsking === `${root.group}\nsmil`, root.said());
            // What's typed in the group is asked again first (ReadyRows' onTold).
            root.fresh = [];
            link.said(JSON.stringify({ id: `find-${before + 1}`, keys: [], rows: {} }));
            Expect.expect("and what's typed in the group is what's answered",
                served.typing.within === root.group && served.typing.words === "smil"
                    && served.typedAsking === "" && served.asking === "turt", root.said());
            Expect.expect("and isn't taken as its key's own",
                root.fresh.length === 1 && root.fresh[0] === false, JSON.stringify(root.fresh));

            // An emoji's row is keyed by its thing where it has one, kept under `row:` where
            // that claims the launcher's own kind, and by its key where it has none.
            const claiming = theEmoji.made({ thing: "place:Harbour", kind: "emoji", acts: [] },
                "emoji:x");
            Expect.expect("an emoji whose thing claims a place is kept under row:",
                claiming?.key === "row:place:Harbour", String(claiming?.key));
            const thingless = theEmoji.made({ kind: "emoji", acts: [] }, "emoji:x");
            Expect.expect("and one with no thing is keyed by its key",
                thingless?.key === "emoji:x", String(thingless?.key));
        });
    }
}
