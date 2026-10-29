// Drives the launcher's quests (modules/quickshell/launcher/Quests.qml)
// through Quickshell itself, over a quest store written beside this and
// read by the real pi-quests, with niri, pi's sessions and the launcher
// stood in. With nothing typed it lists the open quests, the one driving
// first, each saying its kind and priority; typing finds a finished one
// too, after the open ones that match as well, and by its id; Return opens
// pi on a quest in the folder the harness would pick, autoloaded, unless a
// session has it loaded, when it goes to that session's window; the right
// arrow steps into its facts, its parent's title among them; and a quest
// made while the launcher was shut is there when it opens. It exits 0 once
// every case holds, and otherwise 1, saying which didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher

ShellRoot {
    id: root

    readonly property string store: Quickshell.env("QUESTS")
    readonly property string here: Quickshell.env("PWD")
    property string waiting: "the first reading"

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    function fail(message) {
        console.warn(`FAIL ${message}`);
        Qt.callLater(() => Qt.exit(1));
    }

    QtObject {
        id: launcher

        property bool open: false
    }

    Launcher.Quests {
        id: quests

        launcher: launcher
        settings: ({ name: "quests" })
        // Each after the change is done, since a case changes them again.
        onAllChanged: {
            if (root.waiting === "the first reading" && quests.all.length > 0)
                Qt.callLater(root.first);
            else if (root.waiting === "the quest made since" && quests.all.length > 4)
                Qt.callLater(root.since);
        }
    }

    Process {
        id: making
        command: ["mv", `${root.here}/later/QEST-20261020-NEWONE`, `${root.store}/`]
        onExited: {
            launcher.open = true;
        }
    }

    // Nothing here waits forever: a case that never comes fails, by name.
    Timer {
        running: true
        interval: 30000
        onTriggered: root.fail(`never came: ${root.waiting}`)
    }

    function names(rows) {
        return rows.map(row => row.name).join(", ");
    }

    function first() {
        if (root.waiting !== "the first reading")
            return;
        root.waiting = "the first cases";
        try {
            const open = quests.everything();
            expect("the open quests, driving first",
                names(open) === "Bootstrap the laptop, Retune the prompt, Wait on the stick", names(open));
            const [laptop, prompt, stick] = open;
            expect("a quest says its priority", laptop.side === "driving", laptop.side);
            expect("a subquest says its kind too", prompt.side === "subquest \u00b7 queued", prompt.side);
            expect("a blocked one wears waiting's mark", stick.glyph === Theme.glyph.pending, stick.glyph);
            expect("an active one wears its own", laptop.glyph === Theme.glyph.active, laptop.glyph);
            expect("one reads its summary's first paragraph",
                laptop.reads === "What Bootstrap the laptop is about.", laptop.reads);
            expect("the foot's count", quests.counted() === "3 open, 1 driving", quests.counted());

            const banner = quests.search("banner").rows;
            expect("a finished one, found by typing",
                names(banner) === "Tune the banner" && banner[0].glyph === Theme.glyph.done
                    && banner[0].side === "5 Jan 2025", `${names(banner)} ${banner[0]?.side}`);
            // Each matches "the" as well, and the finished one is the
            // shortest, yet the open ones come first.
            const the = quests.search("the").rows;
            expect("the open ones before the finished one",
                the.length === 4 && names(the).endsWith(", Tune the banner"), names(the));
            const byId = quests.search("20261003").rows;
            expect("found by its id", names(byId) === "Retune the prompt", names(byId));

            laptop.run();
            expect("starting a session in its newest tree",
                JSON.stringify(Sessions.started)
                    === JSON.stringify(["QEST-20261001-LAPTOP", `${root.here}/newer`]),
                JSON.stringify(Sessions.started));
            prompt.run();
            expect("starting one where its newest session ran",
                JSON.stringify(Sessions.started)
                    === JSON.stringify(["QEST-20261003-PROMPT", `${root.here}/session`]),
                JSON.stringify(Sessions.started));

            // A day this year goes without its year.
            const touched = new Date().getFullYear() === 2026 ? "21 Oct" : "21 Oct 2026";
            const facts = prompt.verbs();
            // Each fact named first, as every part is, then what it says.
            expect("stepping in", names(facts)
                === "Start a Session, ID, Status, Last Touched, Part Of, Folder", names(facts));
            const sides = facts.slice(1).map(row => row.side).join(", ");
            expect("what each says", sides
                === `QEST-20261003-PROMPT, active \u00b7 queued, ${touched}, `
                + "Bootstrap the laptop, ~/session", sides);
            const folder = facts[facts.length - 1];
            expect("its folder copied whole", folder.copy === `${root.here}/session`, folder.copy);
            expect("a fact copies itself", facts[1].verb === "copy", facts[1].verb);

            // A session with it loaded: Return goes there.
            Sessions.running = [{
                address: "WCJQSG",
                window: { id: 7 },
                presence: { owns: ["QEST-20261003-PROMPT"] }
            }];
            const held = quests.everything()[1];
            expect("a loaded one names its session",
                held.side === "subquest \u00b7 queued \u00b7 WCJQSG" && held.verb === "switch", held.side);
            held.run();
            expect("going to its session", JSON.stringify(Niri.acted) === '{"FocusWindow":{"id":7}}',
                JSON.stringify(Niri.acted));
            expect("its step says so", held.verbs()[0].name === "Switch" && held.verbs()[0].verb === "switch",
                held.verbs()[0].name);

            root.waiting = "the quest made since";
            making.running = true;
        } catch (error) {
            fail(error.message);
        }
    }

    function since() {
        if (root.waiting !== "the quest made since")
            return;
        root.waiting = "the end";
        try {
            const found = quests.search("newly").rows;
            expect("a quest made since", names(found) === "Newly made", names(found));
            Qt.callLater(() => Qt.exit(0));
        } catch (error) {
            fail(error.message);
        }
    }
}