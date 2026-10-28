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
    // A song isn't a mode Joel left on, so Now lists it in a group of its
    // own, between what reaches other people and the modes.
    group: "playing"
    actions: Music.player === null ? [] : [{
        glyph: Theme.glyph.pause,
        name: "Pause",
        chord: chord("toggle"),
        stays: true,
        run: () => Music.toggle()
    }].concat(Music.player.canGoNext ? [{
        glyph: Theme.glyph.next,
        name: "Next track",
        chord: chord("next"),
        stays: true,
        run: () => Music.next()
    }] : [], Music.player.canGoPrevious ? [{
        glyph: Theme.glyph.previous,
        name: "Previous track",
        chord: chord("previous"),
        stays: true,
        run: () => Music.previous()
    }] : [], Music.revealable ? [{
        glyph: Music.mark,
        name: `Show ${Music.from}`,
        run: () => Music.reveal(Music.player)
    }] : [], Music.others.map(other => ({
        glyph: Theme.glyph.pause,
        name: `Pause ${Music.name(other)} too`,
        stays: true,
        run: () => other.pause()
    })))
}
