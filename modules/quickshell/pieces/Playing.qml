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
        glyph: Words.glyph("verb:on:playing:pause"),
        name: Words.name("verb:on:playing:pause"),
        verb: Words.verb("verb:on:playing:pause"),
        chord: chord("toggle"),
        stays: true,
        run: () => Music.toggle()
    }].concat(Music.player.canGoNext ? [{
        glyph: Words.glyph("verb:on:playing:next"),
        name: Words.name("verb:on:playing:next"),
        verb: Words.verb("verb:on:playing:next"),
        chord: chord("next"),
        stays: true,
        run: () => Music.next()
    }] : [], Music.player.canGoPrevious ? [{
        glyph: Words.glyph("verb:on:playing:previous"),
        name: Words.name("verb:on:playing:previous"),
        verb: Words.verb("verb:on:playing:previous"),
        chord: chord("previous"),
        stays: true,
        run: () => Music.previous()
    }] : [], Music.revealable ? [{
        glyph: Music.mark,
        name: Words.name("verb:on:playing:show", "", { player: Music.from }),
        verb: Words.verb("verb:on:playing:show"),
        run: () => Music.reveal(Music.player)
    }] : [], Music.others.map(other => ({
        glyph: Words.glyph("verb:on:playing:pauseOther"),
        name: Words.name("verb:on:playing:pauseOther", "", { player: Music.name(other) }),
        verb: Words.verb("verb:on:playing:pauseOther"),
        stays: true,
        run: () => other.pause()
    })))
}
