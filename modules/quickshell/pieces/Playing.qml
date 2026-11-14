// Music playing, from whichever player matters (Music.qml): true while it
// plays, so paused is nothing to show. Now says it as Music's row, the one
// the arrival line draws too (Music.js's row): the track, who plays it and
// on what, and what the media keys do, with their chords, and a way to the
// player. Mpris says when anything changes, so nothing is asked but where
// in the track it is, which its drawer reads each second while the
// launcher shows it (Now.qml's drawn clock), MPRIS saying a position only
// when asked.
import qs
import QtQuick
import "../launcher/Media.js" as Media

Piece {
    id: piece

    readonly property var row: Music.row

    // Everything the player says, under Now's row, as a program's Media
    // row draws it (launcher/Media.js).
    drawer: Music.player === null ? null : allowed => Media.drawer(Music.player,
        id => Words.name(`program:media:${id}`), Words.of("program:media").said,
        allowed.lookups)

    active: Music.playing
    value: Music.title
    title: Music.heading
    hint: Music.artist === "" ? Music.title : `${Music.title}, ${Music.artist}`
    // The player once: here, unless the act to show it names it.
    context: piece.row?.note ?? ""
    hue: "program"
    // A song isn't a mode Joel left on, so Now lists it in a group of its
    // own, between what reaches other people and the modes.
    group: "playing"
    actions: piece.row?.acts ?? []
}
