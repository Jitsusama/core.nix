// Drives do not disturb (modules/quickshell/Quiet.qml) through what can
// happen to it, with niri's casts, the notices and the recorder stood in
// for: sharing the screen holds notices and stopping lets them go, and
// so does niri not having said its casts, as at the shell's start or while
// the windows daemon is away, since nothing can yet say nobody sees, and
// ending that hold never carries over to a share niri then says is real;
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
        // Spend, Runs and Agents read whether it's shared, not whether it's on.
        if (!Quiet.kept && Quiet.sharing !== (on || Quiet.waived))
            throw new Error(`${what}: sharing ${Quiet.sharing}`);
    }

    Component.onCompleted: {
        try {
            expect("before niri said its casts", true, "while the screen may be shared");
            Quiet.end();
            expect("ended before niri said its casts", false);
            share("PipeWire");
            Niri.castsHeard = true;
            expect("a share heard after its hold was ended unheard", true, "while you share");
            share("");
            expect("once niri said none", false);
            Niri.castsHeard = false;
            Options.doNotDisturb = { whileSharing: false };
            expect("unheard, with a share left to Joel", false);
            Options.doNotDisturb = { whileSharing: true };
            expect("unheard, once a share holds them", true, "while the screen may be shared");
            Niri.castsHeard = true;
            expect("heard once more", false);
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
            Niri.castsHeard = false;
            expect("once niri's casts went unheard again", true, "while the screen may be shared");
            Niri.castsHeard = true;
            expect("heard again", false);
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
