// niri, for the launcher's Screens (tests/displays.qml): a panel and a
// monitor, answered anew at each ask of Outputs as niri does. The monitor
// goes off while monitorOn is false, and turned answers the same screens in
// the other order, since niri's answer is a map collected afresh each ask.
pragma Singleton
import Quickshell
import QtQuick

Singleton {
    property var workspaces: ({})
    property int revision: 0
    property bool monitorOn: true
    property bool turned: false
    readonly property bool powersOffEach: false

    function workspace() {
        return { output: "eDP-1" };
    }

    function act(request) {}

    function spawn(command) {}

    function request(asked, answer) {
        if (asked !== "Outputs")
            return;
        const mode = (width, height) => ({
            width: width, height: height, refresh_rate: 60000, is_preferred: true
        });
        const screen = (name, make, model, logical) => ({
            name: name, make: make, model: model, serial: null, physical_size: [300, 190],
            modes: [mode(2880, 1800), mode(1920, 1080)], current_mode: 0,
            is_custom_mode: false, vrr_supported: false, vrr_enabled: false, logical: logical
        });
        const at = (x, y) => ({
            x: x, y: y, width: 1440, height: 900, scale: 2, transform: "Normal"
        });
        const screens = [
            ["eDP-1", screen("eDP-1", "LG Display", "0x07C6", at(0, 0))],
            ["DP-1", screen("DP-1", "Dell Inc.", "U2723QE", monitorOn ? at(1440, 0) : null)]
        ];
        const outputs = {};
        for (const [name, said] of turned ? screens.reverse() : screens)
            outputs[name] = said;
        answer({ Outputs: outputs });
    }
}
