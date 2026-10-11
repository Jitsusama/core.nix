// Caps lock: true while it's on. It's a mode, so it never holds unseen; the
// key that set it is the one way out, as the hint says.
import qs
import QtQuick

Piece {
    active: Lights.caps
    value: Lights.caps ? "on" : "off"
    title: "Caps Lock"
    hint: "Caps lock is on"
    context: "caps lock turns it off"
}
