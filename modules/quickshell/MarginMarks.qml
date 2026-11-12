// How many marks the margin draws, centred at the top of each screen a cell
// the gap's size apiece (Margin.qml sets it as they change), so what else
// lives in the gap above the windows, as the margin's cat does
// (modules/margin-cat), keeps off them without working out again what the
// margin shows.
pragma Singleton
import Quickshell
import QtQuick

Singleton {
    property int drawn: 0
}
