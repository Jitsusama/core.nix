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
//
// A piece with a mark is also drawn in the margin while it's true
// (Margin.qml): its mark in the drawer, a short `badge` beside it, the
// `hint` said when it's pointed at, and its panel, which is the launcher
// holding it: its `title` in a chip, how it stands (`value`), what can be
// done (`actions`, launcher rows that run with the margin's `shell`) and
// what more there is to know (`context`). It `wants` Joel when it needs
// him now, and only then wears its kind's colour (`hue`), or its role's
// when that isn't plain text.
import QtQuick

Item {
    visible: false

    property var settings: ({})
    // Whether there's anything to show at all: no battery, no battery piece.
    property bool present: true
    property bool active: false
    property string value: ""
    property string role: "text"

    property string badge: ""
    property string hint: value
    property string title: settings.label ?? ""
    property string context: ""
    property bool wants: false
    property string hue: "machine"
    // { glyph, name, chord, run(shell) }
    property var actions: []
}
