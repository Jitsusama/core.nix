// What Now's Home Network piece says ails the home network, run in Qt's
// own JavaScript engine, the one the shell runs it in. The
// `now-says-what-ails-the-home-network` check copies pieces/HomeNetwork.js
// and tests/lib/Expect.js beside this file and runs
// `qml -platform offscreen home-network-ails.qml` over records shaped as
// the eero daemon keeps them (modules/eero/src/model.rs), in words.json's
// on:home-network words.
import QtQuick
import "pieces/HomeNetwork.js" as HomeNetwork
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

    readonly property var said: ({
        internetDown: "internet down",
        providerDown: "provider down",
        mesh: "home network {state}",
        nodeOffline: "{node} offline",
        oneOffline: "1 node offline",
        nodesOffline: "{count} nodes offline"
    })

    // A home network that's well, as eero says one, with `over` laid on
    // its internet and `nodes` in place of its three online.
    function network(cloud, over, nodes) {
        return {
            cloud: { state: cloud, since: "2026-11-07T22:54:43Z" },
            internet: Object.assign({
                state: "connected",
                isp_up: true,
                mesh: "connected",
                nodes: { online: 3, offline: 0, pending: 0 },
                probe: { at: "2026-11-07T22:54:43Z", home: true, reachable: true }
            }, over),
            nodes: nodes ?? [
                { location: "Hall", state: "ONLINE" },
                { location: "Study", state: "ONLINE" },
                { location: "Den", state: "ONLINE" }
            ]
        };
    }

    function out(home) {
        return { at: "2026-11-07T23:00:00Z", home: home, reachable: false };
    }

    function run() {
        Expect.same("a home network that's well says nothing",
            HomeNetwork.ails(network("answering", {}), said), []);
        Expect.same("no record says nothing", HomeNetwork.ails(null, said), []);
        Expect.same("this machine finding the internet out from home says that alone",
            HomeNetwork.ails(network("unreachable", { probe: out(true) }), said),
            ["internet down"]);
        Expect.same("away from home, this machine's look is its own network's, not home's",
            HomeNetwork.ails(network("unreachable", { probe: out(false) }), said), []);
        Expect.same("while eero is out of reach, what it said last is from before",
            HomeNetwork.ails(network("unreachable", { isp_up: false }), said), []);
        Expect.same("signed out, nothing is known now",
            HomeNetwork.ails(network("signed-out", { state: "disconnected" }), said), []);
        Expect.same("the internet and the provider down, as eero says them",
            HomeNetwork.ails(network("answering", { state: "DISCONNECTED", isp_up: false }),
                said),
            ["internet down", "provider down"]);
        Expect.same("the nodes' network ailing, then the node off line, by where it is",
            HomeNetwork.ails(network("answering", {
                mesh: "degraded",
                nodes: { online: 2, offline: 1, pending: 0 }
            }, [
                { location: "Hall", state: "ONLINE" },
                { location: "Den", state: "OFFLINE" }
            ]), said),
            ["home network degraded", "Den offline"]);
        Expect.same("two nodes off line are counted",
            HomeNetwork.ails(network("answering", { nodes: { online: 1, offline: 2 } }, [
                { location: "Hall", state: "ONLINE" },
                { location: "Den", state: "OFFLINE" },
                { location: "Study", state: "OFFLINE" }
            ]), said),
            ["2 nodes offline"]);
        Expect.same("a node eero counts off line but doesn't list is counted alone",
            HomeNetwork.ails(network("answering", { nodes: { online: 2, offline: 1 } }), said),
            ["1 node offline"]);
    }
}
