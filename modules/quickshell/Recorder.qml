pragma Singleton
// Recording the screen: the focused screen, a region picked by the keys
// (Picker.qml) or one window, each into a file of its own beside the
// screenshots' (Videos/Recordings, named by when it began). The chord or
// action that starts a recording stops it too, so there's one gesture to
// remember, and the margin shows it while it runs (pieces/Recording.qml).
//
// A region's picture is taken here too, since niri's own picker takes only
// the pointer. It's saved where niri saves its screenshots, named as niri
// names them, by when it was taken, and copied as theirs are, and says so in
// the notice niri gives its own, so every screenshot ends the same way.
//
// wf-recorder records the screen and regions through niri, which tells the
// shell when it starts and stops, so nothing here keeps any state of its
// own. niri says where a tiled window sits only to a screencast, so a
// window is recorded by record-window (modules/quickshell/record-window)
// through the screencast a call would share, and it says in the runtime
// directory which cast is its own. Each encodes on the GPU where there is
// one, so the processor stays free, and is stopped as Ctrl+C would, so it
// finishes the file. Once it has, a notice says where it went, and playing
// it is the notice's action, which Super+N opens on. It answers what Joel
// just did, so it's never missed (Notices.qml).
import qs
import Quickshell
import Quickshell.Io
import QtQuick

Singleton {
    id: root

    // What niri says is recording now: the screen or a region straight
    // from niri, and the window whose screencast record-window says is its
    // own. A call's shares are left out.
    readonly property var casts: Object.values(Niri.casts)
        .filter(cast => cast.is_active && (cast.kind === "WlrScreencopy"
            || cast.kind === "PipeWire" && windowSession !== null
                && cast.session_id === windowSession))
    // The screencast session record-window records a window through, which
    // looks to niri like a call's share, so sharing and do not disturb leave
    // it out (pieces/Sharing.qml, Quiet.qml).
    readonly property var windowSession: windowRecording.said?.session ?? null
    // A video the camera's viewfinder records (modules/camera), which niri
    // never sees, as its recording says it while it runs: its process, the
    // file that asks it to stop, and where it goes. Joel recorded one and
    // saw only the camera in use, never that it was recording.
    readonly property var camera: cameraRecording.said
    readonly property bool recording: casts.length > 0 || camera !== null
    // Whether a recording can be stopped from here, which takes its process.
    readonly property bool stoppable: camera !== null
        || casts.some(cast => recorderOf(cast) !== null)
    // Between asking for a recording and niri saying it began, so a second
    // press can't start another.
    property bool starting: false
    // What the last one recorded, in words, since niri sees a region as the
    // screen it's cut from.
    property string what: "the whole screen"
    readonly property string folder: `${Quickshell.env("HOME")}/Videos/Recordings`

    onRecordingChanged: starting = false

    // The process recording a cast: niri's own casts say it, and a window's
    // is record-window's.
    function recorderOf(cast: var): var {
        if (cast.kind === "PipeWire")
            return windowRecording.said?.pid ?? null;
        return cast.pid;
    }

    function stop(): void {
        // As the viewfinder's Return does: asked to stop, then told, so
        // the recording finishes its file.
        if (camera !== null)
            Niri.spawn(["sh", "-c", ': > "$1"; kill -INT "$2"', "recorder", camera.stop,
                String(camera.pid)]);
        for (const cast of casts) {
            const pid = recorderOf(cast);
            if (pid !== null)
                Niri.spawn(["kill", "-INT", String(pid)]);
        }
    }

    // Each starts one, or stops the one running.
    function screen(): void {
        if (stopped())
            return;
        const workspace = Niri.workspace();
        what = "the whole screen";
        record(workspace === null ? [] : ["--output", workspace.output]);
    }

    // The window the keys are in, or the one given. The video is as many
    // pixels as the window shows, so it's as sharp as it's seen, and keeps
    // that size, the window fitted in, should the window change its own.
    function window(chosen: var): void {
        if (stopped())
            return;
        const target = chosen ?? (Niri.current === null ? null : Niri.windows[Niri.current] ?? null);
        if (target === null)
            return;
        starting = true;
        settled.restart();
        Niri.request("Outputs", answer => {
            const workspace = Niri.workspaces[target.workspace_id] ?? null;
            const output = workspace === null ? null : answer.Outputs[workspace.output] ?? null;
            const scale = output?.logical?.scale ?? 1;
            const [width, height] = target.layout.window_size;
            what = "a window";
            Niri.spawn(["sh", "-c", windowScript, "recorder", folder, String(target.id),
                String(Math.round(width * scale)), String(Math.round(height * scale))]);
        });
    }

    // Escape in the picker calls back, if given.
    function region(back: var): void {
        if (stopped())
            return;
        Picker.pick("Record a region", Theme.glyph.recording, geometry => {
            what = "a region";
            record(["--geometry", geometry]);
        }, back ?? null);
    }

    // niri gives its own pictures their notice.
    Connections {
        target: Niri

        function onScreenshotCaptured(path: var): void {
            Report.say(`captured ${path}`);
        }

        // record-window writes what it says before its cast begins and
        // takes it back once it ends, so it's read again as they change.
        function onCastsChanged(): void {
            windowRecording.reload();
        }
    }

    onCameraChanged: {
        if (camera !== null)
            what = "a video from the camera";
    }

    FileView {
        id: cameraRecording
        property var said: null
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/camera-recording.json`
        printErrors: false
        watchChanges: true
        onFileChanged: reload()
        onLoaded: {
            try {
                const read = JSON.parse(text());
                said = typeof read?.pid === "number" ? read : null;
            } catch (error) {
                said = null;
            }
        }
        onLoadFailed: said = null
    }

    // What record-window says while it records: its process, and which of
    // niri's casts is its own.
    FileView {
        id: windowRecording
        property var said: null
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/record-window.json`
        printErrors: false
        onLoaded: {
            try {
                said = JSON.parse(text());
            } catch (error) {
                said = null;
            }
        }
        onLoadFailed: said = null
    }

    function screenshot(back: var): void {
        const script = [
            'folder=$HOME/Pictures/Screenshots',
            'mkdir -p "$folder" || exit',
            // The picture is made under a hidden name, then linked to its own,
            // so what watches the folder sees it only whole. A link fails
            // where the name is taken, so a second picture in the same second
            // is numbered, not written over.
            'name=$folder/$(date "+%Y-%m-%d %H-%M-%S")',
            'part=$folder/.screenshot-$$.png',
            'grim -g "$1" "$part" || { rm -f "$part"; exit 1; }',
            'file=$name.png n=1',
            'until ln "$part" "$file" 2> /dev/null; do',
            '    [ -e "$file" ] || { rm -f "$part"; exit 1; }',
            '    n=$((n + 1)) file="$name $n.png"',
            'done',
            'rm -f "$part"',
            'wl-copy --type image/png < "$file"',
            'notify-send --app-name=niri --transient --icon="file://$file" \\',
            '    "Screenshot captured" "You can paste the image from the clipboard."'
        ].join("\n");
        Picker.pick("A picture of a region", Theme.glyph.capture,
            geometry => Niri.spawn(["sh", "-c", script, "screenshot", geometry]), back ?? null);
    }

    function stopped(): bool {
        if (starting)
            return true;
        if (!recording)
            return false;
        stop();
        return true;
    }

    // Records into a new file with these arguments.
    function record(given: var): void {
        starting = true;
        settled.restart();
        const script = [
            'folder=$1; shift',
            'mkdir -p "$folder" || exit',
            'file=$folder/$(date "+%Y-%m-%d %H-%M-%S").mp4',
            // Intel's and AMD's drivers encode through VA-API.
            'case $(readlink /sys/class/drm/renderD128/device/driver) in',
            '    */i915 | */xe | */amdgpu)',
            '        set -- "$@" --codec h264_vaapi --device /dev/dri/renderD128 ;;',
            'esac',
            'wf-recorder -y --file "$file" "$@" > /dev/null 2>&1 || exit'
        ].concat(saved).join("\n");
        Niri.spawn(["sh", "-c", script, "recorder", folder].concat(given));
    }

    // Records a window into a new file: its id, then the video's width and
    // height. record-window says why it couldn't on its standard error, and
    // exits 3 when it was stopped before it began, which needs no notice.
    readonly property string windowScript: [
        'folder=$1 window=$2 width=$3 height=$4',
        'mkdir -p "$folder" || exit',
        'file=$folder/$(date "+%Y-%m-%d %H-%M-%S").mp4',
        'why=$(record-window "$window" "$width" "$height" "$file" 2>&1 > /dev/null)',
        'case $? in',
        '    0) ;;',
        '    3) exit ;;',
        '    *) printf "%s\\n" "$why" >&2',
        '       exec notify-send --app-name=Recorder "The window could not be recorded" "$why" ;;',
        'esac'
    ].concat(saved).join("\n")

    // Says where a recording went, with playing it as the notice's action.
    readonly property var saved: [
        'asked=$(notify-send --app-name=Recorder --action=play="Play it" \\',
        '    "Recording saved" "Videos/Recordings/${file##*/}")',
        '[ "$asked" = play ] && exec xdg-open "$file"'
    ]

    // Gives up waiting for niri when a recording never began, as when
    // wf-recorder failed.
    Timer {
        id: settled
        interval: 5000
        onTriggered: root.starting = false
    }

    IpcHandler {
        target: "recording"

        function screen(): void {
            root.screen();
        }

        function region(): void {
            root.region(null);
        }

        function window(): void {
            root.window(null);
        }

        function stop(): void {
            root.stop();
        }
    }

    IpcHandler {
        target: "screenshot"

        function region(): void {
            root.screenshot(null);
        }
    }
}
