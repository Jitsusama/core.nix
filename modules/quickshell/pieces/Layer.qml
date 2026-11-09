// The keyboard's layer (KeyboardLayer.qml): true while it's locked on one,
// which Joel can forget, and not while a key holds one, which he can't. It's
// a mode, so it never holds unseen; the layer lock key lets it go, and so
// does the action. It's held only while the board is among the keyboards
// connected now (ConnectedKeyboards.qml), so an unplugged board's last
// word never leaves its mark.
import qs
import QtQuick

Piece {
    // The board's own drawer, as the Keyboards place draws its row
    // (launcher/Now.qml's borrowed), since the layer is the board's.
    drawnBy: ConnectedKeyboards.board === null ? null : {
        place: "keyboards",
        key: `keyboards:${ConnectedKeyboards.board.phys || ConnectedKeyboards.board.name}`
    }
    present: KeyboardLayer.present && ConnectedKeyboards.board !== null
    active: KeyboardLayer.locked
    value: KeyboardLayer.locked ? `${KeyboardLayer.name} \u00b7 locked` : KeyboardLayer.name
    title: "Keyboard Layer"
    hint: `The keyboard is locked on ${KeyboardLayer.name}`
    context: "the layer lock key lets it go"
    actions: KeyboardLayer.locked ? [{
        glyph: Words.glyph("verb:on:layer:release"),
        name: Words.name("verb:on:layer:release"),
        verb: Words.verb("verb:on:layer:release"),
        stays: true,
        run: () => Niri.spawn(["ergodox", "layer", "0"])
    }] : []
}
