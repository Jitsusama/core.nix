import QtQuick
import "PiSource.js" as Pi

// pi's sessions, as many and as large as Joel's, each changing its step in
// turn, each change told as the node tells it and sorted as PiSource.qml
// does, the launcher shut for half of them and showing them for the rest,
// and each taken lived again as Sessions.qml does. It counts only what's
// taken and lived again itself: the check runs it
// with QV4's collector reporting each collection, and holds their number
// well under one a change. Told as text, Joel's 117 sessions' 280,000
// characters had made each change start a collection of its own (303 in
// 300 changes on 2026-11-10), each ending in a sweep nothing can split.
Item {
    id: root

    // As many sessions as Joel had, each with a presence about as long as
    // his median, most of it the groups and work its session owns.
    readonly property int sessions: 120
    readonly property int changes: 100
    property var held: ({})
    property var sorting: ({ live: [], resting: [], ending: [] })
    property int changed: 0
    property int taken: 0
    // What Sessions.qml's relive() last made, and how often what it made
    // moved.
    property var lived: ({ live: [], resting: [], ending: [] })
    property int relived: 0

    function session(at) {
        const names = [];
        for (let one = 0; one < 40; one += 1)
            names.push(`#quest/${at}-${one} the group's own long name to be said`);
        return {
            id: `s-${at}`,
            state: "busy",
            standing: "running",
            here: true,
            machine: "skiff",
            presence: {
                addresses: [`A${at}`],
                session: `s-${at}`,
                title: `Stand-in session ${at}`,
                groups: names,
                owns: names.slice(0, 30),
                step: "bash",
                at: "2026-11-10T03:00:00Z"
            }
        };
    }

    // As Sessions.qml's relive() does: each session copied with its kitty,
    // and each list kept unless what it holds moved.
    function relive() {
        const kitted = record => Object.assign({}, record, { kitty: null, opened: "" });
        const next = { live: sorting.live.map(kitted), resting: sorting.resting.map(kitted),
            ending: sorting.ending.map(kitted) };
        if (!Pi.alike(lived.live, next.live))
            relived += 1;
        lived = next;
    }

    Component.onCompleted: {
        const held = {};
        for (let at = 0; at < sessions; at += 1)
            held[`session:s-${at}`] = session(at);
        root.held = held;
        root.sorting = Pi.sorted(held);
        changing.start();
    }

    Timer {
        id: changing

        interval: 1
        repeat: true
        onTriggered: {
            // A change arrives as the node's text, parsed and laid over
            // what's held, as Daemon.qml does. A step moving alone carries
            // no new digest of what's read shut.
            const key = `session:s-${root.changed % root.sessions}`;
            const said = JSON.parse(JSON.stringify({ presence: Object.assign({},
                root.held[key].presence, { step: `step ${root.changed}` }) }));
            const answer = { id: 1, changed: [Object.assign({ key }, said)] };
            const held = Object.assign({}, root.held);
            held[key] = Object.assign({}, root.held[key], said);
            root.held = held;
            // The launcher shows them for the second half, so each change
            // is taken there, and lived again as Sessions.qml does.
            const shown = root.changed >= root.changes / 2;
            if (shown || Pi.movesShut(answer)) {
                root.taken += 1;
                root.sorting = Pi.sorted(held);
                root.relive();
            }
            root.changed += 1;
            if (root.changed < root.changes)
                return;
            stop();
            if (root.taken !== root.changes / 2 || root.relived !== root.taken) {
                console.log(`FAIL ${root.taken} changes were taken and ${root.relived} `
                    + `lived again, where only the ${root.changes / 2} shown should be`);
                Qt.exit(1);
                return;
            }
            console.log(`changes ${root.changes}`);
            Qt.quit();
        }
    }
}
