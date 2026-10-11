// Whether the machine is kept awake, and whether shutting its lid leaves it
// running too. It's true while it's kept awake, which is a mode, so it never
// holds unseen.
import qs
import QtQuick

Piece {
    active: Caffeinate.on
    value: !Caffeinate.on ? "no" : Caffeinate.lid ? "lid shut too" : "while open"
}
