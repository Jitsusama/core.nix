// The shape of one piece of a surface's content. A piece says what it shows
// and how things stand; the surface decides where and how it's drawn, so
// every piece, built in or a machine's own, lines up with the rest. A new
// piece is a file that starts from this one:
//
//     Piece {
//         value: "what it shows"
//         active: true // while it's true: tier "when" shows it, and its mark
//         role: "warning" // the theme's colour for it, by role
//     }
//
// Its settings from Nix, its format or its thresholds, are in `settings`.
import QtQuick

Item {
    visible: false

    property var settings: ({})
    // Whether there's anything to show at all: no battery, no battery piece.
    property bool present: true
    property bool active: false
    property string value: ""
    property string role: "text"
}
