// The keyboard's layout: the ErgoDox's while it's connected, as its module
// describes it in ergodox/layout.json among the account's settings
// (modules/ergodox), and otherwise the laptop's own, as keyd lays it out
// (modules/keyd/layout.nix, in /etc/keyboard/layout.json). Each says its
// layers' names and keys, each key where it sits in key widths, its cap, its
// words, what holding it does, and whether it falls through to the layer
// below. A board that isn't connected is never shown, since a layout for
// keys that aren't there only misleads (Joel, 2026-11-07: "seeing ergodox
// layouts and shit when it isn't plugged in is weird as fuck"), so it's the
// one list of keyboards connected now that says (ConnectedKeyboards.qml),
// and an open overlay turns to the laptop's in place as the board goes, and
// back as it comes (VKCF3Z, 2026-11-07). With neither, as on a machine with
// no keyboard of its own and no board, there's nothing to show.
//
// It also holds what the overlay (KeyboardMap.qml) shows: nothing, the
// layer the board is on, or a layer and a key the launcher chose.
pragma Singleton
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher/Keyboards.js" as Keyboards

Singleton {
    id: root

    readonly property var boardLayers: layersIn(file)
    readonly property var laptopLayers: layersIn(laptop)
    // Whether the board is connected and its layout known, so what only
    // the board can do (its layers, its lights, flashing it) is offered.
    readonly property bool boarded: Keyboards.boarded(ConnectedKeyboards.listed, boardLayers)
    readonly property var layers: Keyboards.layout(ConnectedKeyboards.listed, boardLayers,
        laptopLayers)
    readonly property bool present: layers.length > 0
    // A layer or key chosen on one keyboard's layout means nothing on the
    // other's, so the overlay follows the keyboard again as it changes.
    onBoardedChanged: {
        chosen = -1;
        marked = -1;
        if (!present)
            hide();
    }

    function layersIn(view) {
        if (!view.loaded)
            return [];
        try {
            return JSON.parse(view.text()).layers ?? [];
        } catch (e) {
            return [];
        }
    }

    // The layer the board is on, by its place, or the first while the board
    // can't say, as the laptop's one layer is.
    readonly property int live: !boarded ? 0
        : Math.max(0, layers.findIndex(layer => layer.name === KeyboardLayer.name))

    property bool shown: false
    // A layer the launcher chose to show, or -1 to follow the board, and the
    // key it chose on it, or -1.
    property int chosen: -1
    property int marked: -1
    readonly property int layer: chosen >= 0 ? chosen : live

    function toggle(): void {
        if (shown) {
            hide();
        } else {
            chosen = -1;
            marked = -1;
            shown = present;
        }
    }

    function show(layer: int, key: int): void {
        chosen = layer;
        marked = key;
        shown = present;
    }

    function hide(): void {
        shown = false;
        chosen = -1;
        marked = -1;
    }

    // The board's commands go through `ergodox`, which a notice answers for
    // flashing, since that takes a while and happens with the launcher gone.
    function flash(): void {
        Niri.spawn(["sh", "-c", "if ergodox flash; then notify-send --app-name=Keyboard "
            + "'Flashed the keyboard' \"It's running this machine's build.\"; else "
            + "notify-send --app-name=Keyboard --urgency=critical "
            + "'The keyboard wasn'\\''t flashed' 'Run ergodox flash in a terminal to see why.'; fi"]);
    }

    function lock(layer: int): void {
        Niri.spawn(["ergodox", "layer", String(layer)]);
    }

    // Whether the board's lights were last lit, from here, since the board
    // can't be asked; lit until then, as it starts.
    property bool lit: true

    function lights(on: bool): void {
        lit = on;
        Niri.spawn(["ergodox", "lights", on ? "on" : "off"]);
    }

    function brighter(more: bool): void {
        Niri.spawn(["ergodox", more ? "brighter" : "dimmer"]);
    }

    FileView {
        id: file
        path: `${Quickshell.env("XDG_CONFIG_HOME") || Quickshell.env("HOME") + "/.config"}/ergodox/layout.json`
        printErrors: false
        watchChanges: true
        onFileChanged: reload()
    }

    // Where keyd's module says the laptop's layout; a test says its own.
    property string laptopLayout: "/etc/keyboard/layout.json"

    FileView {
        id: laptop
        path: root.laptopLayout
        printErrors: false
        watchChanges: true
        onFileChanged: reload()
    }
}
