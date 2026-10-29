pragma Singleton
// Where the laptop is, the one place the shell learns it from: what
// whereabouts last found through GeoClue (whereabouts/), its town and the
// zone it keeps, the time zone the clock keeps now, and the clock's last
// move to follow the laptop (clock-follows/). The weather's here
// and Now's Location read it, and nothing else asks where the laptop is.
//
// Each is read as the shell starts and again as it changes, never assumed:
// with no file yet, here is unknown, and Now says so (Joel, 2026-10-28: "i
// never want it to assume anything"). The clock's zone is asked of
// systemd as the shell starts and each time /etc/localtime is replaced.
import Quickshell
import Quickshell.Io
import QtQuick

Singleton {
    id: root

    // The position whereabouts writes, replaced whole each time here
    // moves or is named. A test points it at a file of its own.
    property string file: `${Quickshell.env("XDG_RUNTIME_DIR")}/whereabouts.json`

    // Here, as the file says it: { latitude, longitude, accuracy, when,
    // place, zone }, place and zone null while they couldn't be asked; or
    // null with no file, or one of another version.
    property var here: null
    // Whether the file has been read once, so "unknown" is said only once
    // it's known to be.
    property bool read: false

    // The zone the clock keeps, "America/Toronto", or "" until systemd has
    // said.
    property string clockZone: ""

    // Whether the place keeps another zone than the clock does.
    readonly property bool zoneDiffers: here !== null && (here.zone ?? "") !== ""
        && clockZone !== "" && here.zone !== clockZone

    // The clock's last move to where the laptop is (clock-follows/), {
    // from, to, place, when }, kept until it's put back or moves again, or
    // null; said only while the clock still keeps the zone it moved to.
    property var moved: null
    readonly property bool clockMoved: moved !== null && moved.to === clockZone

    readonly property string kept: `${Quickshell.env("XDG_STATE_HOME")
        || `${Quickshell.env("HOME")}/.local/state`}/clock-follows`

    function movedOf(text) {
        let said;
        try {
            said = JSON.parse(text);
        } catch (error) {
            return null;
        }
        return typeof said?.from === "string" && typeof said.to === "string" ? said : null;
    }

    // Puts the clock back in the zone it kept before it followed the
    // laptop, where it's held until the laptop is somewhere else.
    function putBack() {
        Quickshell.execDetached(["systemctl", "--user", "start", "--no-block",
            "clock-put-back.service"]);
    }

    FileView {
        path: `${root.kept}/moved.json`
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        onLoaded: root.moved = root.movedOf(text())
        onLoadFailed: root.moved = null
    }

    function hereOf(text) {
        let said;
        try {
            said = JSON.parse(text);
        } catch (error) {
            return null;
        }
        return said?.v === 2 && Number.isFinite(said.latitude) && Number.isFinite(said.longitude)
            ? said : null;
    }

    FileView {
        path: root.file
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        onLoaded: {
            root.here = root.hereOf(text());
            root.read = true;
        }
        onLoadFailed: {
            root.here = null;
            root.read = true;
        }
    }

    // The clock's zone is the link /etc/localtime makes, which a file read
    // can't say, so systemd is asked, once at the start and whenever the
    // link is replaced.
    FileView {
        path: "/etc/localtime"
        watchChanges: true
        printErrors: false
        blockLoading: false
        onFileChanged: zoneAsked.running = true
    }

    Process {
        id: zoneAsked
        command: ["timedatectl", "show", "--property=Timezone", "--value"]
        running: true
        stdout: StdioCollector {
            onStreamFinished: root.clockZone = text.trim()
        }
    }
}
