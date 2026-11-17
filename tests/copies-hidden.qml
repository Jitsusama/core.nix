// When Copies hides what each copy holds, run in Qt's own JavaScript
// engine, the one the shell runs it in. The
// `copies-hide-while-the-screen-is-shared` check copies launcher/Shared.js
// and tests/lib/Expect.js beside this file and runs it over casts made up
// here, as niri says them.
import QtQuick
import "Shared.js" as Shared
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

    // A cast as niri's CastsChanged says one, by its stream.
    function cast(stream, kind, session, active) {
        return { [stream]: { stream_id: stream, session_id: session, kind: kind,
            is_active: active, target: { Output: { name: "eDP-1" } } } };
    }

    function run() {
        Expect.same("nothing cast, the copies show", Shared.hidden(true, {}, null), false);
        Expect.same("before niri says its casts, they hide", Shared.hidden(false, {}, null), true);
        Expect.same("and with no word of them at all", Shared.hidden(undefined, undefined, null),
            true);
        Expect.same("a call sharing the screen hides them",
            Shared.hidden(true, cast(4, "PipeWire", 9, true), null), true);
        Expect.same("and one starting, not yet streaming",
            Shared.hidden(true, cast(4, "PipeWire", 9, false), null), true);
        Expect.same("the shell recording one window shows them",
            Shared.hidden(true, cast(4, "PipeWire", 9, true), 9), false);
        Expect.same("but not while a call shares beside it",
            Shared.hidden(true, Object.assign(cast(4, "PipeWire", 9, true),
                cast(5, "PipeWire", 12, true)), 9), true);
        Expect.same("a screen recorder outside the portal shows them",
            Shared.hidden(true, cast(4, "WlrScreencopy", 9, true), null), false);
    }
}
