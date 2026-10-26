// The power profile, while it isn't balanced: power saver or full
// performance is a mode Joel set and might forget, so Now lists it among
// the modes that are on and the margin wears its glyph (Joel, 2026-10-26:
// Now in its four groups, a power profile that isn't balanced among the
// modes). Balanced, the usual, says nothing. Its words are the launcher's
// own for each profile, so Now and Machine › Power say it the same way.
import qs
import Quickshell.Services.UPower
import QtQuick

Piece {
    readonly property bool saving: PowerProfiles.profile === PowerProfile.PowerSaver
    readonly property bool full: PowerProfiles.profile === PowerProfile.Performance
    readonly property string used: saving ? "session:power:Power Saver"
        : full ? "session:power:Performance" : ""
    readonly property string balanced: "session:power:Balanced Power"

    active: used !== ""
    // The profile's own glyph, which the margin and Now draw in place of
    // the entry's mark, so the mark says which one is on.
    markGlyph: used !== "" ? Words.of(used).glyph ?? "" : ""
    value: used !== "" ? Words.of(used).part : Words.of(balanced).part
    title: "Power Profile"
    hint: `${value} is in use`
    context: saving ? "the machine runs slower to last longer"
        : full ? "the fans and the battery work harder" : ""
    actions: used === "" ? [] : [{
        glyph: Words.glyph(balanced),
        name: Words.of(balanced).name,
        verb: Words.verb(balanced),
        run: () => PowerProfiles.profile = PowerProfile.Balanced
    }]
}
