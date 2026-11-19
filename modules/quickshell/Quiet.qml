pragma Singleton
// Do not disturb: while it's on, a notice is held rather than shown, kept in
// the session's history (Notices.qml) unseen, for the launcher's collection
// of them; a critical one still shows, since it needs Joel now. It's on for
// a while or until it's ended, and says so in the margin, since a mode
// should never hold unseen.
//
// It's on by itself while another program sees the screen, as a call shares
// it, and off again once nothing does (jitsusama.quickshell.doNotDisturb.whileSharing).
// Ending it then lets notices show until the sharing stops.
//
// What Joel turned on lasts through a restart of the shell, as a switch does,
// from a file in the runtime directory holding when it began and when it ends,
// in milliseconds since 1970, an end of 0 meaning until it's ended. The
// session's end clears it with the runtime directory. The sharing needs no
// file, since niri says again what streams.
import Quickshell
import Quickshell.Io
import QtQuick

Singleton {
    id: root

    // Turned on by Joel, rather than by the sharing.
    property bool kept: false
    property real since: 0
    property real until: 0
    // Whether niri has yet said which casts there are (Niri.castsHeard), as
    // it hasn't at the shell's start or while the windows daemon is away:
    // until then nothing can say nobody sees the screen, so it's taken as
    // shared, failing closed as copies do (launcher/Shared.js), and a
    // notice never shows to a call in that moment (#5399's council,
    // 2026-11-18).
    readonly property bool unheard: Niri.castsHeard !== true
    // A window being recorded streams as a share would, but nobody sees it.
    readonly property bool sharing: Options.doNotDisturb.whileSharing && (unheard
        || Object.values(Niri.casts).some(cast => cast.kind === "PipeWire" && cast.is_active
            && cast.session_id !== Recorder.windowSession))
    // Ended while sharing, so notices show until the sharing stops.
    property bool waived: false
    property real sharedSince: 0
    readonly property bool shared: sharing && !waived
    readonly property bool on: kept || shared
    // When notices began to be held, and how long they will be.
    readonly property real began: kept && shared ? Math.min(since, sharedSince)
        : kept ? since : sharedSince
    readonly property string lasts: kept && until > 0
        ? `until ${Qt.formatDateTime(new Date(until), "HH:mm")}`
        : kept ? "until it's ended" : unheard ? "while the screen may be shared"
        : "while you share"
    // The notices held since it began and not yet seen.
    readonly property var held: Notices.history
        .filter(notice => notice.held && notice.missed && notice.at >= began)

    onSharingChanged: {
        sharedSince = sharing ? Date.now() : 0;
        waived = false;
    }
    // A waiver given while the screen only may have been shared never carries
    // over to a share niri then says is real, which leaves sharing true and so
    // fires no change of its own (#5399's council, 2026-11-18).
    onUnheardChanged: waived = false

    // A hold for want of niri's casts that lasts past a moment is said in the
    // journal, since it holds every notice, and only the margin's mark says
    // so otherwise.
    Timer {
        running: root.sharing && root.unheard && !root.waived
        interval: 10000
        onTriggered: console.info(
            "quiet: notices held for 10 s now, since niri hasn't said who sees the screen")
    }
    Component.onCompleted: sharedSince = sharing ? Date.now() : 0

    // Holds notices for this many minutes, or until it's ended with none.
    function keep(minutes) {
        since = kept ? since : Date.now();
        until = minutes > 0 ? Date.now() + minutes * 60000 : 0;
        kept = true;
        save();
    }

    // Holds them this many minutes longer than it would have.
    function more(minutes) {
        if (until > 0)
            until = Math.max(until, Date.now()) + minutes * 60000;
        save();
    }

    function end() {
        kept = false;
        waived = sharing;
        save();
    }

    function save() {
        file.setText(kept ? `${since} ${until}\n` : "");
    }

    // Ends when its time is up, however long that is.
    Timer {
        running: root.kept && root.until > 0
        interval: Math.max(1, root.until - Date.now())
        onTriggered: {
            root.kept = false;
            root.save();
        }
    }

    FileView {
        id: file
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/quiet`
        printErrors: false
        onLoaded: {
            const [since, until] = text().trim().split(" ").map(Number);
            const left = text().trim() !== "" && (until === 0 || until > Date.now());
            root.since = left ? since : 0;
            root.until = left ? until : 0;
            root.kept = left;
        }
    }
}
