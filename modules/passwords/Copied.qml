// A password, or a one-time code, the Passwords place copied, on the
// clipboard until the passwords daemon clears it (Clearing.qml): its mark
// in the margin from the moment it's copied until it clears, since the
// launcher goes as a copy does, so Joel can paste at once, and isn't there
// to say it (VKCF3Z's ruling). It was left on by Joel's own key, and goes
// by itself, so it's muted, never a notice. Now says it in the foot's own
// words, counting down.
import qs
import QtQuick
import "../launcher/Passwords.js" as Passwords

Piece {
    readonly property var said: Words.of("passwords").said ?? {}
    readonly property var copy: Clearing.copied

    active: Clearing.waiting
    kind: "leftOn"
    title: "Copied"
    value: copy ? `${copy.title}'s ${copy.detail}` : ""
    hint: Passwords.told(copy, Clearing.now, said)
    context: copy ? Passwords.told(copy, Clearing.now, said).split(" \u00b7 ").slice(-1)[0] : ""
}
