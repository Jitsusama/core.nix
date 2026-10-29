// The power profile, while Joel has picked one by hand: a profile the
// machine didn't choose is a mode he set and might forget, so Now lists it
// among the modes that are on and the margin wears its glyph. The machine
// chooses for itself as the charger and the battery change, performance on
// mains and balanced or power saver on battery, and says what it chose in
// /run/power-profile/wanted (modules/power-profiles-daemon/follow.sh), so
// its own choice says nothing however it reads (Joel, 2026-10-28: "why is
// power profile in now?"). Without that file, as where nothing follows the
// charger, balanced is the machine's. Its words are the launcher's own for
// each profile, so Now and Machine › Power say it the same way.
import qs
import Quickshell.Io
import Quickshell.Services.UPower
import QtQuick

Piece {
    id: piece

    // Each profile as follow.sh and the daemon name it, with the words
    // the launcher says it in.
    readonly property var named: ({
        "power-saver": { profile: PowerProfile.PowerSaver, words: "session:power:Power Saver" },
        "balanced": { profile: PowerProfile.Balanced, words: "session:power:Balanced Power" },
        "performance": { profile: PowerProfile.Performance, words: "session:power:Performance" }
    })
    readonly property string said: chosen.loaded ? chosen.text().trim() : ""
    readonly property var machine: named[said] ?? named.balanced
    readonly property var current: Object.values(named)
        .find(each => each.profile === PowerProfiles.profile) ?? machine

    active: current !== machine
    // The profile's own glyph, which the margin and Now draw in place of
    // the entry's mark, so the mark says which one is on.
    markGlyph: Words.of(current.words).glyph ?? ""
    value: Words.of(current.words).part
    title: "Power Profile"
    hint: `${value} is in use`
    context: `picked by hand over ${Words.of(machine.words).part}`
    // Back to what the machine chose, which it then follows again.
    actions: active ? [{
        glyph: Words.glyph(machine.words),
        name: Words.of(machine.words).name,
        verb: Words.verb(machine.words),
        // A setting, so the launcher stays to show it (the closing rule).
        stays: true,
        run: () => PowerProfiles.profile = piece.machine.profile
    }] : []

    FileView {
        id: chosen
        path: piece.settings.wanted ?? "/run/power-profile/wanted"
        printErrors: false
        watchChanges: true
        onFileChanged: reload()
    }
}
