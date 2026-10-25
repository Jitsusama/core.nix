// Shows the battery's charge for a moment when power is plugged in or
// pulled out, so a cable that seated, or didn't, is seen at once, as the
// same bar the volume uses, with the launcher's marks for the charger while
// it's plugged in and for the charge while the battery drains. UPower says
// when, so nothing is asked every so often.
import qs
import Quickshell
import Quickshell.Services.UPower
import QtQuick

Scope {
    id: root

    readonly property UPowerDevice battery: UPower.displayDevice
    property bool shown: false

    Connections {
        target: UPower

        function onOnBatteryChanged() {
            if (!root.battery.isLaptopBattery)
                return;
            root.shown = true;
            hide.restart();
        }
    }

    Timer {
        id: hide
        interval: 1500
        onTriggered: root.shown = false
    }

    Meter {
        name: "power"
        glyph: UPower.onBattery ? Theme.glyph.charge : Theme.glyph.charging
        visible: root.shown
        level: root.battery.percentage
    }
}
