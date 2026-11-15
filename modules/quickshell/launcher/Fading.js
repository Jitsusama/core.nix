// How much the launcher's picks still count as they fade (Picks.qml keeps
// them), kept apart from the QML so Qt's own engine holds it
// (tests/ranking.qml).
.pragma library
.import "Rank.js" as Rank

// How much each row's been picked lately, whatever for: key -> picks.
function lately(picks, _now, _halfLife) {
    const out = {};
    for (const asked of Object.keys(picks)) {
        if (Rank.isStepped(asked)) {
            continue;
        }
        for (const key of Object.keys(picks[asked])) {
            out[key] = (out[key] ?? 0) + picks[asked][key].c;
        }
    }
    return out;
}
