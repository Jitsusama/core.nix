// The keyboard's layer (KeyboardLayer.qml): true while it's locked on one,
// which Joel can forget, and not while a key holds one, which he can't. It's
// a mode, so it never holds unseen; the layer lock key lets it go, and so
// does the action.
import qs
import QtQuick

Piece {
    present: KeyboardLayer.present
    active: KeyboardLayer.locked
    value: KeyboardLayer.locked ? `${KeyboardLayer.name}, locked` : KeyboardLayer.name
    title: "Keyboard Layer"
    hint: `The keyboard is locked on ${KeyboardLayer.name}`
    context: "the layer lock key lets it go"
    actions: KeyboardLayer.locked ? [{
        glyph: Theme.glyph.layerOff,
        name: "Let it go",
        verb: "release",
        stays: true,
        run: () => Niri.spawn(["ergodox", "layer", "0"])
    }] : []
}
