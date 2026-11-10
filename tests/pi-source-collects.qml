import QtQuick
import "PiSource.js" as Pi

// pi's sessions, as many and as large as Joel's, each changing its step in
// turn, sorted and told against the last sorting taken as PiSource.qml
// does with the launcher shut. It counts nothing itself: the check runs it
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
    readonly property var place: ({ machine: "skiff", bootId: "boot", bootedAt: 0, markers: {} })

    function session(at) {
        const names = [];
        for (let one = 0; one < 40; one += 1)
            names.push(`#quest/${at}-${one} the group's own long name to be said`);
        return {
            id: `s-${at}`,
            state: "busy",
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

    Component.onCompleted: {
        const held = {};
        for (let at = 0; at < sessions; at += 1)
            held[`session:s-${at}`] = session(at);
        root.held = held;
        root.sorting = Pi.sorted(held, place);
        changing.start();
    }

    Timer {
        id: changing

        interval: 1
        repeat: true
        onTriggered: {
            // A change arrives as the node's text, parsed and laid over
            // what's held, as Node.qml does.
            const key = `session:s-${root.changed % root.sessions}`;
            const said = JSON.parse(JSON.stringify({ presence: Object.assign({},
                root.held[key].presence, { step: `step ${root.changed}` }) }));
            const held = Object.assign({}, root.held);
            held[key] = Object.assign({}, root.held[key], said);
            root.held = held;
            const next = Pi.sorted(held, root.place);
            if (!Pi.tellsAlike(root.sorting, next, false)) {
                root.taken += 1;
                root.sorting = next;
            }
            root.changed += 1;
            if (root.changed < root.changes)
                return;
            stop();
            if (root.taken !== 0) {
                console.log(`FAIL a step moving alone was taken ${root.taken} times`);
                Qt.exit(1);
                return;
            }
            console.log(`changes ${root.changes}`);
            Qt.quit();
        }
    }
}
