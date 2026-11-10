// What Jobs.qml makes of the machine queue's record from the mycelium node,
// run in Qt's own JavaScript engine, the one the shell runs it in. The
// `the-queue-is-heard-from-the-node` check copies modules/quickshell/Jobs.js
// and tests/lib/Expect.js beside this file and runs
// `qml -platform offscreen jobs.qml`: the file is the record without the
// node's own fields, none of a version it doesn't read; and a write that
// says only when it was written says nothing new, one that changes
// anything else does.
import QtQuick
import "Expect.js" as Expect
import "Jobs.js" as Read

QtObject {
    // A case that doesn't hold throws, naming it, and ends as a failure,
    // rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    function run() {
        const job = { id: "a", kind: "build", state: "running", owner: { session: "s1", pid: 1 } };
        const record = {
            v: 1,
            at: 100.5,
            heartbeat: 100,
            machine: { availableGib: 20 },
            jobs: [job],
            key: "queue:harbour",
            kind: "queue",
            heard: 100500
        };
        const file = Read.told(record);
        Expect.same("the file is the record without the node's fields", file,
            { v: 1, at: 100.5, heartbeat: 100, machine: { availableGib: 20 }, jobs: [job] });
        Expect.same("a job keeps its own kind", file.jobs[0].kind, "build");
        Expect.same("no record is no file", Read.told(null), null);
        Expect.same("a version it doesn't read is no file",
            Read.told(Object.assign({}, record, { v: 2 })), null);

        const later = { at: 130.2, heartbeat: 130, heard: 130200 };
        const beaten = Read.told(Object.assign({}, record, later));
        Expect.same("a write of only when says nothing new",
            Read.saying(beaten) === Read.saying(file), true);
        const roomier = Read.told(Object.assign({}, record, { machine: { availableGib: 19 } }));
        Expect.same("a write of anything else says something new",
            Read.saying(roomier) === Read.saying(file), false);
        Expect.same("no file says nothing", Read.saying(null), "");

        Qt.exit(0);
    }
}
