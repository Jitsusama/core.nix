// How much the launcher's picks still count as they fade (Picks.qml keeps
// them), kept apart from the QML so Qt's own engine holds it
// (tests/ranking.qml).
.pragma library
.import "Rank.js" as Rank

// What one pick entry ({ n: its fading count when last picked, t: when })
// counts at `now`, halved every `halfLife` milliseconds since, so habits
// that change are followed and old ones fade.
function fade(entry, now, halfLife) {
    return entry.n * 0.5 ** ((now - entry.t) / halfLife);
}

// How much each row's been picked lately, whatever for: key -> picks,
// each faded as `fade` has it. A pick inside a step was of the step's own
// row too, which is counted for what opened it, as Picks.qml's counts()
// has it, so it's left out.
function lately(picks, now, halfLife) {
    const out = {};
    for (const asked of Object.keys(picks)) {
        if (Rank.isStepped(asked)) {
            continue;
        }
        for (const key of Object.keys(picks[asked])) {
            out[key] = (out[key] ?? 0) + fade(picks[asked][key], now, halfLife);
        }
    }
    return out;
}
