// What Joel sees when he opens niri's overview or moves between workspaces:
// the time and the date, and how the battery, the network and the speakers
// are. niri's layer rule in config.kdl puts it in the backdrop behind the
// workspaces, so it shows only then and takes no space while he works. The
// overview shrinks the workspaces to half the screen's width, centred, which
// leaves the left quarter free for it.
import Quickshell
import Quickshell.Networking
import Quickshell.Services.Pipewire
import Quickshell.Services.UPower
import Quickshell.Wayland
import QtQuick

Scope {
    id: root

    readonly property UPowerDevice battery: UPower.displayDevice
    readonly property PwNode sink: Pipewire.defaultAudioSink
    readonly property NetworkDevice online: Networking.devices.values.find(device => device.connected) ?? null

    readonly property string batteryState: {
        const charge = Math.round(battery.percentage * 100) + "%";
        switch (battery.state) {
        case UPowerDeviceState.Charging:
            return charge + ", charging";
        case UPowerDeviceState.Discharging:
            return battery.timeToEmpty > 0 ? charge + ", " + duration(battery.timeToEmpty) + " left" : charge;
        default:
            return charge;
        }
    }

    readonly property string networkState: {
        if (!online)
            return "offline";
        if (online.type === DeviceType.Wired)
            return "wired";
        const network = online.networks.values.find(network => network.connected);
        return network ? network.name : online.name;
    }

    readonly property string volumeState: {
        if (!sink)
            return "none";
        return sink.audio.muted ? "muted" : Math.round(sink.audio.volume * 100) + "%";
    }

    function duration(seconds) {
        const minutes = Math.round(seconds / 60);
        return Math.floor(minutes / 60) + ":" + String(minutes % 60).padStart(2, "0");
    }

    component Line: Text {
        color: Theme.text
        font.family: Theme.monoFamily
        font.pointSize: Theme.monoSize
    }

    component Label: Line {
        color: Theme.muted
    }

    SystemClock {
        id: clock
        precision: SystemClock.Minutes
    }

    // Pipewire reports a node's volume only once something binds it.
    PwObjectTracker {
        objects: [root.sink]
    }

    Variants {
        model: Quickshell.screens

        PanelWindow {
            required property ShellScreen modelData

            screen: modelData
            anchors.top: true
            anchors.left: true
            margins.top: Theme.gap * 4
            margins.left: Theme.gap * 4
            implicitWidth: status.implicitWidth
            implicitHeight: status.implicitHeight
            // niri takes only a background surface that ignores the space
            // others reserve into the backdrop.
            exclusionMode: ExclusionMode.Ignore
            color: "transparent"
            WlrLayershell.layer: WlrLayer.Background
            WlrLayershell.namespace: "backdrop"

            Column {
                id: status
                spacing: Theme.gap

                Line {
                    text: Qt.formatDateTime(clock.date, "HH:mm")
                    font.pointSize: Theme.monoSize * 4
                }

                Line {
                    text: Qt.formatDateTime(clock.date, "dddd d MMMM")
                }

                Grid {
                    columns: 2
                    columnSpacing: Theme.gap * 2
                    rowSpacing: Theme.gap / 2
                    topPadding: Theme.gap

                    Label {
                        text: "battery"
                        visible: root.battery.isLaptopBattery
                    }
                    Line {
                        text: root.batteryState
                        visible: root.battery.isLaptopBattery
                    }
                    Label {
                        text: "network"
                    }
                    Line {
                        text: root.networkState
                    }
                    Label {
                        text: "volume"
                    }
                    Line {
                        text: root.volumeState
                    }
                }
            }
        }
    }
}
