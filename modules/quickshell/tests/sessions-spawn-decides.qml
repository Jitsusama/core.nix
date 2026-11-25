pragma ComponentBehavior: Bound
// Hands Sessions (modules/quickshell/Sessions.qml) five sessions to open, as
// the harness's quest spawn does, with niri stood in and the harness's
// spawn-decide too (modules/quickshell/tests/stand-ins/spawn-decide.sh). Each fresh one is asked of
// the decider first: the one it hosts opens no window, the one it keeps out
// opens told why, the one it says nothing of in three seconds opens once
// it's ended, and the one it fails on opens as before; a saved one opens
// at once, never asked. Then, with the harness's config saying sessions start
// in its host, nine are started on quests: the one hosted opens only a
// viewer, the ones kept out open their windows, four hosted under an id or
// host no viewer could carry open neither, and are said, and of two the
// host answers with one session, the second is said opening while the
// first's viewer comes; started again once its viewer is listed, the first
// raises it. Once the config says nothing, one opens its window never
// asked. It exits 0 once that holds, and otherwise 1, saying what didn't.
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
    // Why Sessions said each start by its quest wasn't viewed, and the
    // name it gave each, which the launcher's foot says it by.
    property var unhosted: ({})
    property var unhostedNames: ({})
    property real startedAt: 0
    // When QEST-HOSTED's session was started again, once its viewer was
    // listed, or 0 before.
    property real againAt: 0
    // How many acts niri had been asked for then, so only a raise asked
    // for after counts as the start's.
    property int actsAgain: 0
    // How long a start the host runs may go unsaid, well past the stand-in
    // decider's answer and well inside the check's 60 s.
    readonly property int unsaidFor: 10000
    readonly property string folder: Quickshell.env("HOME")

    function spawned(id) {
        return Niri.spawns.filter(command => command.includes(` pi --session-id ${id}`)
            || command.includes(` pi --session ${id}`));
    }

    // Each window the shell spawns opens, as niri would say it, so the
    // walk goes on to the next rather than waiting out its patience; but
    // s-late's viewer, which niri is slow to list. Each is drawn by the one
    // sessions' kitty, whose pid the check writes, as its unit does.
    function openSpawned() {
        for (const command of Niri.spawns) {
            const classed = /\/pi-sessions\/kitty\.sock (\S+) /.exec(command)?.[1] ?? "";
            if (classed === "" || classed === "pi-hosted.s-late" || root.opened[classed])
                continue;
            const id = 100 + Object.keys(root.opened).length;
            root.opened = Object.assign({}, root.opened, { [classed]: Date.now() });
            const window = { id, pid: 4242, workspace_id: 40, session: true,
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
        Expect.expect("the sessions' kitty's pid is known, so its windows are judged by it",
            Sessions.sessionsKittyPid === "4242", Sessions.sessionsKittyPid);
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
        Sessions.start("QEST-HOSTED", root.folder, true, "Kelp Harbour");
        Sessions.start("QEST-AWAY", root.folder, false);
        Sessions.start("QEST-PLAIN", root.folder, false);
        Sessions.start("QEST-DOTTED", root.folder, false, "Dotted Reef");
        Sessions.start("QEST-BADHOST", root.folder, false);
        Sessions.start("QEST-DASHID", root.folder, false);
        Sessions.start("QEST-DASHHOST", root.folder, false);
        Sessions.start("QEST-LATE", root.folder, false, "Late Reef");
        Sessions.start("QEST-LATER", root.folder, false, "Later Reef");
        root.startedAt = Date.now();
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
        // Hosted under an id no viewer's app id could carry, or a host
        // mycelium couldn't be given, it has no viewer to open, nor a
        // window, which would begin a second pi on its quest: it's said
        // started instead.
        const unviewed = ["QEST-DOTTED", "QEST-BADHOST", "QEST-DASHID", "QEST-DASHHOST"];
        Expect.expect("one hosted under an id or host no viewer can carry opens neither",
            !Niri.spawns.some(command => /--session (s\.dotted|s-badhost|-s-x|s-y)/.test(command))
            && unviewed.every(quest => startedOn(quest).length === 0), Niri.spawns.join("\n"));
        const late = ["QEST-LATE", "QEST-LATER"];
        if (!unviewed.every(quest => quest in root.unhosted)
            || !late.some(quest => quest in root.unhosted)) {
            // The decider answers within moments, so one still unsaid
            // after a few seconds was let go with nothing said.
            if (Date.now() - root.startedAt > unsaidFor)
                Expect.fail(`started but never said: ${JSON.stringify(root.unhosted)}`);
            return;
        }
        // Said by which held: its id, or its host's name, which mycelium
        // couldn't be given.
        const why = { "QEST-DOTTED": "unviewable", "QEST-DASHID": "unviewable",
            "QEST-BADHOST": "unviewableHost", "QEST-DASHHOST": "unviewableHost" };
        Expect.expect("and each is said started, though it can't be viewed, by which held",
            unviewed.every(quest => root.unhosted[quest] === why[quest]),
            JSON.stringify(root.unhosted));
        // By its quest's name where the launcher gave one, never its id.
        Expect.expect("one started by its quest's name is said by that name",
            root.unhostedNames["QEST-DOTTED"] === "Dotted Reef",
            JSON.stringify(root.unhostedNames));
        // The host answers both with one session: the first's viewer opens
        // and, while niri is slow to list it, the second is said opening.
        Expect.expect("of two started on one session, one viewer opens and the second says so",
            late.filter(quest => root.unhosted[quest] === "opening").length === 1
            && Niri.spawns.filter(command => command.includes("/kitty.sock pi-hosted.s-late "))
                .length === 1, `${JSON.stringify(root.unhosted)}\n${Niri.spawns.join("\n")}`);
        // Held back, it's said by its quest's name too, never its id.
        const titled = { "QEST-LATE": "Late Reef", "QEST-LATER": "Later Reef" };
        Expect.expect("the one held back is said by its quest's name",
            late.every(quest => root.unhosted[quest] !== "opening"
                || root.unhostedNames[quest] === titled[quest]),
            JSON.stringify(root.unhostedNames));
        const viewer = Niri.spawns.filter(command => command.includes(
            " mycelium --realm harbour attach --host harbour-attended --session s-begun"));
        if (viewer.length === 0 || startedOn("QEST-AWAY").length === 0
            || startedOn("QEST-PLAIN").length === 0)
            return;
        // The opening takes the sessions kitty's socket, then the class
        // its window is named by.
        Expect.expect("its viewer is named for it",
            viewer[0].includes("/kitty.sock pi-hosted.s-begun "), viewer[0]);
        // Titled for its quest's name before its words run, not "mycelium"
        // nor its id.
        Expect.expect("and titled for its quest's name before its words run",
            viewer[0].includes('sh -c printf "\\033]2;%s\\007" "$0"; exec "$@" '
                + `\u03c0 - Kelp Harbour - ${root.folder.slice(root.folder.lastIndexOf("/") + 1)}`
                + " mycelium "), viewer[0]);
        decided.reload();
        const lines = decided.text().split("\n");
        const started = (quest, orchestrates) => `started ${quest} in ${root.folder}`
            + ` orchestrate=${orchestrates ? 1 : 0}`;
        // Each is asked in its folder, with whether it orchestrates, which
        // is looked for again until it's read, as the first stage's are.
        if (![started("QEST-HOSTED", true), started("QEST-AWAY", false),
            started("QEST-PLAIN", false)].every(line => lines.includes(line)))
            return;
        // Started again once niri lists its viewer, that viewer is raised,
        // never a second.
        const shown = Object.values(Niri.windows).find(window => window.opened
            === "pi-hosted.s-begun");
        if (!shown)
            return;
        if (root.againAt === 0) {
            root.actsAgain = Niri.acts.length;
            Sessions.start("QEST-AGAIN", root.folder, false);
            root.againAt = Date.now();
            return;
        }
        if (!Niri.acts.slice(root.actsAgain).some(act => act.FocusWindow?.id === shown.id)) {
            if (Date.now() - root.againAt > unsaidFor)
                Expect.fail("started again but its viewer wasn't raised: "
                    + JSON.stringify(Niri.acts));
            return;
        }
        Expect.expect("started again, its listed viewer is raised, never a second",
            Niri.spawns.filter(command => command.includes("/kitty.sock pi-hosted.s-begun "))
                .length === 1, Niri.spawns.join("\n"));
        // Said outside sections, where the harness never reads it, is said
        // nothing of.
        config.setText('{"quest-workflow": {"startHost": "attended"}}');
        stage = "unset";
    }

    // Said nothing of where the harness reads it, a start opens its window
    // at once, never asking.
    function unset() {
        // Waits until the old shape is what's read, so a moment's empty
        // file as it's written can't pass for it.
        if (Sessions.startsHosted || !config.text().includes("quest-workflow"))
            return;
        Sessions.start("QEST-QUIET", root.folder, false);
        Expect.expect("said outside sections, a start opens its window at once",
            startedOn("QEST-QUIET").length === 1, Niri.spawns.join("\n"));
        decided.reload();
        Expect.expect("and its decider is never asked", !decided.text().includes("QEST-QUIET"),
            decided.text());
        console.log("a spawn kept in the background opens no window");
        Qt.exit(0);
    }

    Connections {
        target: Sessions

        function onUnhosted(session, why) {
            root.unhosted = Object.assign({}, root.unhosted, { [session.address]: why });
            root.unhostedNames = Object.assign({}, root.unhostedNames,
                { [session.address]: session.name ?? "" });
        }
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
