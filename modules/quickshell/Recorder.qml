pragma Singleton
// Recording the screen: the focused screen, or a region picked with the
// pointer, each into a file of its own beside the screenshots'
// (Videos/Recordings, named by when it began). niri tells only the portal's
// shares where a tiled window sits, so a window isn't recorded alone. The
// chord or action that starts a recording stops it too, so there's one
// gesture to remember, and the margin shows it while it runs
// (pieces/Recording.qml).
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

    // The region is picked with the pointer, the one way to draw a box.
    function region(): void {
        if (stopped())
            return;
        what = "a region";
        record([], ["slurp", "-d", "-w", String(Theme.border),
            "-b", hex(Theme.shadow, 0), "-c", hex(Theme.accent, 1),
            "-s", hex(Theme.selection, Theme.floating.selection)]);
    }

    function stopped(): bool {
        if (starting)
            return true;
        if (!recording)
            return false;
        stop();
        return true;
    }

    // A colour as slurp writes it, red, green, blue and alpha in hex.
    function hex(colour: color, alpha: real): string {
        const byte = value => Math.round(value * 255).toString(16).padStart(2, "0");
        return "#" + [colour.r, colour.g, colour.b, alpha].map(byte).join("");
    }

    // Records into a new file with these arguments, or, given a picker,
    // into the region it prints.
    function record(given, picker) {
        starting = true;
        // Drawing a region takes a while longer than starting does.
        settled.interval = picker === undefined ? 5000 : 60000;
        settled.restart();
        const script = [
            'folder=$1; shift',
            'if [ "$1" = pick ]; then',
            '    shift; region=$("$@") || exit 0; set -- --geometry "$region"',
            'fi',
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
        const picking = picker === undefined ? [] : ["pick"].concat(picker);
        Niri.spawn(["sh", "-c", script, "recorder", folder].concat(picking, given));
    }

    // Gives up waiting for niri when a recording never began: the region's
    // picker was dismissed, or wf-recorder failed.
    Timer {
        id: settled
        onTriggered: root.starting = false
    }

    IpcHandler {
        target: "recording"

        function screen(): void {
            root.screen();
        }

        function region(): void {
            root.region();
        }

        function stop(): void {
            root.stop();
        }
    }
}
