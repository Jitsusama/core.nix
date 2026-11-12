pragma ComponentBehavior: Bound
// Hands Sessions (modules/quickshell/Sessions.qml) five sessions to open, as
// the harness's quest spawn does, with niri stood in and the harness's
// spawn-decide too (stand-ins/spawn-decide.sh). Each fresh one is asked of
// the decider first: the one it hosts opens no window, the one it keeps out
// opens told why, the one it says nothing of in three seconds opens once
// it's ended, and the one it fails on opens as before; a saved one opens
// at once, never asked. Then, with the harness's config saying sessions start
// in its host, three are started on quests: the one hosted opens only a
// viewer, the ones kept out open their windows; and once the config says
// nothing, one opens its window never asked. It exits 0 once that holds,
// and otherwise 1, saying what didn't.
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
            else if (stage === "starting")
                starting();
            else if (stage === "unset")
                unset();
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
        // Read afresh on each look, since a change notice for the last
        // line written can be missed, and nothing would be written after.
        decided.reload();
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
        Expect.expect("the config says sessions start in pi's host", Sessions.startsHosted, "");
        Sessions.start("QEST-HOSTED", root.folder, true);
        Sessions.start("QEST-AWAY", root.folder, false);
        Sessions.start("QEST-PLAIN", root.folder, false);
        stage = "starting";
    }

    function startedOn(quest) {
        const told = `QUEST_WORKFLOW_AUTOLOAD_ID=${quest} `;
        return Niri.spawns.filter(command => command.includes(told));
    }

    // Hosted, a viewer of it opens and no window of pi; kept out, a window
    // opens as it always did.
    function starting() {
        Expect.expect("one started in pi's host opens no window",
            startedOn("QEST-HOSTED").length === 0, Niri.spawns.join("\n"));
        const viewer = Niri.spawns.filter(command => command.includes(
            " mycelium --realm harbour attach --host harbour-attended --session s-begun"));
        if (viewer.length === 0 || startedOn("QEST-AWAY").length === 0
            || startedOn("QEST-PLAIN").length === 0)
            return;
        Expect.expect("its viewer is named for it",
            viewer[0].includes(" --class pi-hosted.s-begun "), viewer[0]);
        decided.reload();
        const lines = decided.text().split("\n");
        const started = (quest, orchestrates) => `started ${quest} in ${root.folder}`
            + ` orchestrate=${orchestrates ? 1 : 0}`;
        Expect.expect("each is asked in its folder, with whether it orchestrates",
            [started("QEST-HOSTED", true), started("QEST-AWAY", false),
                started("QEST-PLAIN", false)].every(line => lines.includes(line)),
            lines.join("\n"));
        config.setText("{}");
        stage = "unset";
    }

    // Said nothing of, a start opens its window at once, never asking.
    function unset() {
        if (Sessions.startsHosted)
            return;
        Sessions.start("QEST-QUIET", root.folder, false);
        Expect.expect("with nothing said, a start opens its window at once",
            startedOn("QEST-QUIET").length === 1, Niri.spawns.join("\n"));
        decided.reload();
        Expect.expect("and its decider is never asked", !decided.text().includes("QEST-QUIET"),
            decided.text());
        console.log("a spawn kept in the background opens no window");
        Qt.exit(0);
    }

    FileView {
        id: config

        path: `${Quickshell.env("HOME")}/.config/pi/agentic-harness.pi/config.json`
        blockLoading: true
    }

    FileView {
        id: decided

        path: Quickshell.env("DECIDED")
        blockLoading: true
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
        onTriggered: Expect.fail(`still at ${root.stage}: ${Niri.spawns.join("\n")}`
            + `\ndecided:\n${decided.text()}`)
    }
}
