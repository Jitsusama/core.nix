// Music playing, from whichever player matters (Music.qml): true while it
// plays, so paused is nothing to show. What's on says the track, the artist
// and the player, and offers what the media keys do, with their chords, and
// a way to the player. Mpris says when anything changes, so nothing is asked.
import qs
import QtQuick

Piece {
    id: piece

    // The chord bound to a music command, beside the action that does it.
    function chord(command) {
        return (Shortcuts.actions.find(action => action.run.join(" ")
            === `qs ipc call music ${command}`) ?? { chord: [] }).chord;
    }

    active: Music.playing
    value: Music.title
    title: "Playing"
    hint: Music.artist === "" ? Music.title : `${Music.title}, ${Music.artist}`
    // The player once: here, unless the action to show it names it.
    context: [Music.artist, Music.album, Music.revealable ? "" : Music.from]
        .filter(said => said !== "").join(" · ")
    hue: "program"
    actions: Music.player === null ? [] : [{
        glyph: String.fromCodePoint(0xf03e4),
        name: "Pause",
        chord: chord("toggle"),
        run: () => Music.toggle()
    }].concat(Music.player.canGoNext ? [{
        glyph: String.fromCodePoint(0xf04ad),
        name: "Next track",
        chord: chord("next"),
        run: () => Music.next()
    }] : [], Music.player.canGoPrevious ? [{
        glyph: String.fromCodePoint(0xf04ae),
        name: "Previous track",
        chord: chord("previous"),
        run: () => Music.previous()
    }] : [], Music.revealable ? [{
        glyph: Music.mark,
        name: `Show ${Music.from}`,
        run: () => Music.reveal(Music.player)
    }] : [], Music.others.map(other => ({
        glyph: String.fromCodePoint(0xf03e4),
        name: `Pause ${Music.name(other)} too`,
        run: () => other.pause()
    })))
}
