// The keyboard's layout, as a keyboard's module describes it in
// ergodox/layout.json among the account's settings (modules/ergodox): each
// layer's name and keys, each key where it sits in key widths, its cap, its
// words, what holding it does, and whether it falls through to the layer
// below. With no file, as on a machine without the module, there's no
// layout, and the overlay and the launcher's Keyboard collection have
// nothing to show.
//
// It also holds what the overlay (KeyboardMap.qml) shows: nothing, the
// layer the board is on, or a layer and a key the launcher chose.
pragma Singleton
import Quickshell
import Quickshell.Io
import QtQuick

Singleton {
    id: root

    readonly property var layers: {
        if (!file.loaded)
            return [];
        try {
            return JSON.parse(file.text()).layers ?? [];
        } catch (e) {
            return [];
        }
    }
    readonly property bool present: layers.length > 0

    // The layer the board is on, by its place, or the first while the board
    // can't say.
    readonly property int live: Math.max(0, layers.findIndex(layer => layer.name === KeyboardLayer.name))

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

    function lights(on: bool): void {
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
}
