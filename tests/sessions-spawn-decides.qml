pragma ComponentBehavior: Bound
// Hands Sessions (modules/quickshell/Sessions.qml) five sessions to open, as
// the harness's quest spawn does, with niri stood in and the harness's
// spawn-decide too (stand-ins/spawn-decide.sh). Each fresh one is asked of
// the decider first: the one it hosts opens no window, the one it keeps out
// opens told why, the one it says nothing of in three seconds opens once
// it's ended, and the one it fails on opens as before; a saved one opens
// at once, never asked. It exits 0 once that holds, and otherwise 1, saying
// what didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "Expect.js" as Expect

ShellRoot {
    id: root

    property string stage: "handing"
    property real handedAt: 0
    property var opened: ({})
    readonly property string folder: Quickshell.env("HOME")

    function spawned(id) {
        return Niri.spawns.filter(command => command.includes(` pi --session-id ${id}`)
            || command.includes(` pi --session ${id}`));
    }

    // Each window the shell spawns opens, as niri would say it, so the
    // walk goes on to the next rather than waiting out its patience.
    function openSpawned() {
        for (const command of Niri.spawns) {
            const classed = / --class (\S+) /.exec(command)?.[1] ?? "";
            if (classed === "" || root.opened[classed])
                continue;
            const id = 100 + Object.keys(root.opened).length;
            root.opened = Object.assign({}, root.opened, { [classed]: Date.now() });
            const window = { id, pid: id * 10, workspace_id: 40, session: true,
                opened: classed, layout: { pos_in_scrolling_layout: [2, 1] } };
            Niri.windows = Object.assign({}, Niri.windows, { [String(id)]: window });
            Niri.revision += 1;
            Niri.windowOpenedOrChanged(window);
        }
    }

    function one(id, fresh) {
        return { id, cwd: root.folder, fresh, quest: "QEST-1-KELP", model: "m-1" };
    }

    function check() {
        try {
            openSpawned();
            if (stage === "handing")
                handing();
            else if (stage === "deciding")
                deciding();
        } catch (error) {
            Expect.fail(error.message);
        }
    }

    function handing() {
        try {
            Sessions.decider = ["bash", Quickshell.env("SPAWN_DECIDE")];
        } catch (error) {
            console.log(`no decider to stand in: ${error.message}`);
        }
        root.handedAt = Date.now();
        const taken = Sessions.handedOver(JSON.stringify({ sessions: [one("s-saved", false),
            one("s-hosted", true), one("s-stayed", true), one("s-broken", true),
            one("s-slow", true)] }));
        Expect.expect("all five are taken", taken === "taken", taken);
        stage = "deciding";
    }

    // Every one but the hosted opens, the slow one once ended. The hosted
    // one is looked for on every look, since it would open at any point of
    // the walk.
    function deciding() {
        Expect.expect("the one kept in the background opens no window", spawned("s-hosted").length === 0,
            Niri.spawns.join("\n"));
        const lines = decided.text().split("\n").filter(line => line !== "");
        const ended = lines.find(line => line.startsWith("ended s-slow at "));
        if (!ended
            || ["s-saved", "s-stayed", "s-broken", "s-slow"].some(id => spawned(id).length === 0))
            return;
        Expect.expect("the one kept out of a host is told why",
            spawned("s-stayed")[0].includes(" QUEST_WORKFLOW_SPAWN_STAYED=the host is full "),
            spawned("s-stayed")[0]);
        Expect.expect("the others aren't told any such thing",
            ["s-saved", "s-broken", "s-slow"].every(id => !spawned(id)[0].includes("STAYED")),
            Niri.spawns.join("\n"));
        // Ended as it's given up on, which the walk's pace doesn't touch.
        const waited = Number(ended.split(" ").pop()) - root.handedAt;
        Expect.expect("the one its decider says nothing of is ended at three seconds",
            waited >= 3000 && waited < 4500, `${waited} ms`);
        const asked = id => `asked ${id} in ${root.folder} quest=QEST-1-KELP model=m-1`;
        Expect.expect("each fresh one is asked in its folder, with its quest and model",
            ["s-hosted", "s-stayed", "s-broken", "s-slow"].every(id => lines.includes(asked(id))),
            lines.join("\n"));
        Expect.expect("a saved one is never asked", !lines.some(line => line.includes("s-saved")),
            lines.join("\n"));
        console.log("a spawn kept in the background opens no window");
        Qt.exit(0);
    }

    FileView {
        id: decided

        path: Quickshell.env("DECIDED")
        blockLoading: true
        watchChanges: true
        onFileChanged: reload()
    }

    // Every step waits on a timer of Sessions' own or the decider, so
    // each is looked at often rather than heard.
    Timer {
        running: true
        repeat: true
        interval: 100
        onTriggered: root.check()
    }

    Timer {
        running: true
        interval: 40000
        onTriggered: Expect.fail(`still at ${root.stage}: ${Niri.spawns.join("\n")}`)
    }
}
