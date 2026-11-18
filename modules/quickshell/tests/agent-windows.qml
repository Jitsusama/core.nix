// How Windows folds agents' windows into one row, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `the-launcher-places-each-window-it-hears` check copies
// launcher/AgentWindows.js and tests/lib/Expect.js beside this file and
// runs `qml -platform offscreen agent-windows.qml` over records made up
// here, as the agents daemon says them.
import QtQuick
import "launcher/AgentWindows.js" as AgentWindows
import "Expect.js" as Expect

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
            Qt.exit(0);
        } catch (error) {
            Expect.fail(`${error}`);
        }
    }

    // An agent-window record as the agents daemon serves one.
    function agent(address, window, title, over) {
        return {
            key: `agent-window:agents:${address}`, kind: "agent-window", address: address,
            kitty: { id: "1", within: "unix:@kitty-1" }, app_id: `pi-hosted.${address}`,
            window: window, title: title,
            orchestration: over ? { address: over, title: `${over}'s` } : null
        };
    }

    function run() {
        const agents = AgentWindows.agentsOf([
            agent("A", "window:2", "Quick Wins", "P"), agent("B", "window:4", "Lab", "Q"),
            agent("C", "window:5", "Pacing", "P"), agent("D", null, "Unpaired", null),
            agent("E", "window:7", "Alone", null), agent("P", "window:8", "Orchestrator", "P")
        ]);
        Expect.same("a session with no window paired is left out",
            Object.keys(agents).sort(),
            ["window:2", "window:4", "window:5", "window:7", "window:8"]);

        const order = ["window:1", "window:2", "window:3", "window:4", "window:5", "window:7"];
        const fold = AgentWindows.folded(order, agents);
        Expect.same("the fold sits where the agents' window used last was, the rest kept",
            fold.listed, ["window:1", "agents", "window:3"]);
        Expect.same("its windows are those of agents, in the order used",
            fold.inside, ["window:2", "window:4", "window:5", "window:7"]);
        Expect.same("Return on it goes to the agents' window used last", fold.lead, "window:2");

        const first = AgentWindows.folded(["window:4", "window:1", "window:2"], agents);
        Expect.same("an agents' window used last of all puts the fold first",
            first.listed, ["agents", "window:1"]);
        Expect.same("and Return goes to it", first.lead, "window:4");

        const none = AgentWindows.folded(["window:1", "window:3"], agents);
        Expect.same("with no agents' window, nothing folds", none.listed, ["window:1", "window:3"]);
        Expect.same("and nothing leads", none.lead, null);

        Expect.same("the fold's note names the session Return goes to and how many more",
            AgentWindows.noted(fold.inside, agents, "{title} \u00b7 {more} more"),
            "Quick Wins \u00b7 3 more");
        Expect.same("one agents' window is named alone",
            AgentWindows.noted(["window:4"], agents, "{title} \u00b7 {more} more"), "Lab");

        const needs = { needsCount: "1 needs you", needCount: "{count} need you" };
        Expect.same("the fold counts the sessions that need Joel as Agents does",
            [0, 1, 2].map(count => AgentWindows.needed(count, needs)),
            ["", "1 needs you", "2 need you"]);

        const rows = [{ id: 1 }, { id: 2, waiting: true }, { id: 4 }, { id: 5, waiting: true }];
        const worked = AgentWindows.foldOf(rows, agents, "{title} \u00b7 {more} more");
        Expect.same("the fold counts the sessions inside it that need Joel", worked.needing, 2);
        Expect.same("and leads with the window id of the one used last", worked.lead, 2);
        Expect.same("its signature moves only with what the row says", worked.said,
            AgentWindows.foldOf(rows.slice(), agents, "{title} \u00b7 {more} more").said);
        Expect.same("rows with no agents' window fold to nothing",
            AgentWindows.foldOf([{ id: 1 }], agents, ""), null);

        const groups = AgentWindows.byOrchestration(fold.inside, agents);
        Expect.same("inside, the windows go by orchestration, each where its last used was",
            groups.map(group => group.orchestration?.address ?? null), ["P", "Q", null]);
        Expect.same("each orchestration's windows in the order used",
            groups.map(group => group.keys),
            [["window:2", "window:5"], ["window:4"], ["window:7"]]);
        Expect.same("with no orchestrator's window open, its sessions sit at no depth",
            AgentWindows.depths(groups).map(one => one.depth), [0, 0, 0, 0]);

        const led = AgentWindows.byOrchestration(["window:2", "window:8", "window:5"], agents);
        Expect.same("an orchestrator's own window leads its orchestration",
            led[0].keys, ["window:8", "window:2", "window:5"]);
        Expect.same("and its sessions sit a step in under it",
            AgentWindows.depths(led).map(one => one.depth), [0, 1, 1]);
    }
}
