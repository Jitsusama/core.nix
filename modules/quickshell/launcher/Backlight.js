// What the backlight daemon says of the screen's level, in plain
// JavaScript so a check can run it in Qt's own engine, as the shell does:
// the level it last settled, the room it heard, what its curve asks for
// that room and whether it follows it, from backlight-now; and the curve's
// points it has learned from Joel's picks, from its learned file.
// Displays.qml reads the files and draws the screen's drawer.
.pragma library

// A fact a line, its name, a tab and its value, as desk-facts writes them.
const FACT = /^([a-z]+)\t(.*)$/;
// A learned point, its lux, a space and its level: "28 0.950".
const POINT = /^\s*([0-9.]+)\s+([0-9.]+)\s*$/;
const PERCENT = 100;

// The level as a share of the whole, rounded as Brightness says it.
function share(level) {
    return Math.round(level * PERCENT);
}

// What backlight-now says: { level, lux, asks, following }, each level a
// share of the whole, or null while it says nothing, as when the daemon
// has stopped and emptied it.
function now(text) {
    const said = {};
    for (const line of String(text ?? "").split("\n")) {
        const found = FACT.exec(line);
        if (found) {
            said[found[1]] = found[2];
        }
    }
    const level = Number(said.level);
    if (said.level === undefined || !Number.isFinite(level)) {
        return null;
    }
    const lux = Number(said.lux);
    const asks = Number(said.asks);
    return {
        level,
        lux: said.lux !== undefined && Number.isFinite(lux) ? lux : null,
        asks: said.asks !== undefined && Number.isFinite(asks) ? asks : null,
        following: said.following === "yes"
    };
}

// The points the daemon has learned, dimmest first: [{ lux, level }], or
// none where it has learned nothing. A line it would refuse is left out.
function learned(text) {
    return String(text ?? "").split("\n")
        .map(line => POINT.exec(line))
        .filter(found => found !== null)
        .map(found => ({ lux: Number(found[1]), level: Number(found[2]) }))
        .sort((one, other) => one.lux - other.lux);
}

// What sets the level, in the vocabulary's words: the room while the
// curve does, or Joel's pick and what the room asks while one holds.
function follows(settled, said) {
    if (settled === null) {
        return "";
    }
    if (settled.following || settled.asks === null) {
        return said(settled.following ? "the-room" : "your-pick", {});
    }
    return [said("your-pick", {}), said("asks", { share: `${share(settled.asks)}%` })]
        .join(" \u00b7 ");
}

// Each learned point as its share at its lux, a run apart, dimmest first:
// "38% at 6 lx \u00b7 95% at 28 lx".
function learnedSaid(points, said, lux) {
    return points.map(point => said("at", { share: `${share(point.level)}%`, lux: lux(point.lux) }))
        .join(" \u00b7 ");
}
