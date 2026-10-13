pragma Singleton
// Recording the screen: the focused screen, or a region picked by the keys
// (Picker.qml), each into a file of its own beside the screenshots'
// (Videos/Recordings, named by when it began). niri tells only the portal's
// shares where a tiled window sits, so a window isn't recorded alone. The
// chord or action that starts a recording stops it too, so there's one
// gesture to remember, and the margin shows it while it runs
// (pieces/Recording.qml).
//
// A region's picture is taken here too, since niri's own picker takes only
// the pointer. It's saved where niri saves its screenshots, named as niri
// names them, by when it was taken, and copied as theirs are, and says so in
// the notice niri gives its own, so every screenshot ends the same way.
//
// wf-recorder does the recording through niri, which tells the shell when
// it starts and stops, so nothing here keeps any state of its own. It
// encodes on the GPU where there is one, so the processor stays free, and
// is stopped as Ctrl+C would, so it finishes the file. Once it has, a notice
// says where it went, and playing it is the notice's action, which Super+N
// opens on. It answers what Joel just did, so it's never missed
// (Notices.qml).
import qs
import Quickshell
import Quickshell.Io
import QtQuick

Singleton {
    id: root

    // What niri says is recording now, the portal's shares left out.
    readonly property var casts: Object.values(Niri.casts)
        .filter(cast => cast.kind === "WlrScreencopy" && cast.is_active)
    readonly property bool recording: casts.length > 0
    // Between asking for a recording and niri saying it began, so a second
    // press can't start another.
    property bool starting: false
    // What the last one recorded, in words, since niri sees a region as the
    // screen it's cut from.
    property string what: "the whole screen"
    readonly property string folder: `${Quickshell.env("HOME")}/Videos/Recordings`

    onRecordingChanged: starting = false

    function stop(): void {
        for (const cast of casts)
            if (cast.pid !== null)
                Niri.spawn(["kill", "-INT", String(cast.pid)]);
    }

    // Each starts one, or stops the one running.
    function screen(): void {
        if (stopped())
            return;
        const workspace = Niri.workspace();
        what = "the whole screen";
        record(workspace === null ? [] : ["--output", workspace.output]);
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
        Picker.pick("A picture of a region", String.fromCodePoint(0xf0100),
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
            'wf-recorder -y --file "$file" "$@" > /dev/null 2>&1 || exit',
            'asked=$(notify-send --app-name=Recorder --action=play="Play it" \\',
            '    "Recording saved" "Videos/Recordings/${file##*/}")',
            '[ "$asked" = play ] && exec xdg-open "$file"'
        ].join("\n");
        Niri.spawn(["sh", "-c", script, "recorder", folder].concat(given));
    }

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
