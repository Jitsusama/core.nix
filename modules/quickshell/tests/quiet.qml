// Drives do not disturb (modules/quickshell/Quiet.qml) through what can
// happen to it, with niri's casts, the notices and the recorder stood in
// for: sharing the screen holds notices and stopping lets them go,
// recording the screen or a window through its own screencast doesn't, ending
// it while sharing lasts until the sharing stops, and what Joel turned on
// outlasts the sharing. It exits 0 once every case holds, and otherwise 1,
// saying which case didn't.
import qs
import Quickshell
import QtQuick

ShellRoot {
    function share(kind, session = 1) {
        Niri.casts = kind === "" ? {} : { 1: { kind: kind, is_active: true, session_id: session } };
    }

    function expect(what, on, lasts) {
        if (Quiet.on !== on || (on && lasts !== undefined && Quiet.lasts !== lasts))
            throw new Error(`${what}: on ${Quiet.on}, ${Quiet.lasts}`);
    }

    Component.onCompleted: {
        try {
            expect("at first", false);
            share("PipeWire");
            expect("sharing", true, "while you share");
            share("");
            expect("after sharing", false);
            share("WlrScreencopy");
            expect("recording", false);
            Recorder.windowSession = 7;
            share("PipeWire", 7);
            expect("recording a window", false);
            Recorder.windowSession = null;
            share("PipeWire");
            Quiet.end();
            expect("ended while sharing", false);
            share("");
            share("PipeWire");
            expect("sharing again", true);
            Quiet.keep(0);
            share("");
            expect("kept past sharing", true, "until it's ended");
            Quiet.end();
            expect("ended", false);
            Quiet.keep(60);
            const hour = Qt.formatDateTime(new Date(Quiet.until), "HH:mm");
            expect("kept for an hour", true, `until ${hour}`);
            Quiet.end();
            Qt.callLater(() => Qt.exit(0));
        } catch (error) {
            console.warn(`FAIL ${error.message}`);
            Qt.callLater(() => Qt.exit(1));
        }
    }
}
