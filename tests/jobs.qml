// What Jobs.qml makes of the machine queue's record from the mycelium node,
// run in Qt's own JavaScript engine, the one the shell runs it in. The
// `the-queue-is-heard-from-the-node` check copies modules/quickshell/Jobs.js
// beside this file and runs `qml -platform offscreen jobs.qml`: the file is
// the record without the node's own fields, none of a version it doesn't
// read; a write that says only when it was written says nothing new, one
// that changes anything else does; and the harness goes silent once its
// last heartbeat is the silence old.
import QtQuick
import "Jobs.js" as Read

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    function check(failures, what, got, want) {
        if (JSON.stringify(got) !== JSON.stringify(want))
            failures.push(`${what}: got ${JSON.stringify(got)}, wanted ${JSON.stringify(want)}`);
    }

    function run() {
        const failures = [];
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
        check(failures, "the file is the record without the node's fields", file,
            { v: 1, at: 100.5, heartbeat: 100, machine: { availableGib: 20 }, jobs: [job] });
        check(failures, "a job keeps its own kind", file.jobs[0].kind, "build");
        check(failures, "no record is no file", Read.told(null), null);
        check(failures, "a version it doesn't read is no file",
            Read.told(Object.assign({}, record, { v: 2 })), null);

        const later = { at: 130.2, heartbeat: 130, heard: 130200 };
        const beaten = Read.told(Object.assign({}, record, later));
        check(failures, "a write of only when says nothing new",
            Read.saying(beaten) === Read.saying(file), true);
        const roomier = Read.told(Object.assign({}, record, { machine: { availableGib: 19 } }));
        check(failures, "a write of anything else says something new",
            Read.saying(roomier) === Read.saying(file), false);
        check(failures, "no file says nothing", Read.saying(null), "");

        check(failures, "a heartbeat a minute old is quiet in half a minute",
            Read.quietIn(100, 160000, 90), 30000);
        check(failures, "a heartbeat the silence old is quiet now",
            Read.quietIn(100, 190000, 90), 0);
        check(failures, "an older one is quiet too", Read.quietIn(100, 500000, 90), 0);

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
