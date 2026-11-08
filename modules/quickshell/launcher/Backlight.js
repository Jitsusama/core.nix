// What the backlight daemon says of the screen's level, in plain
// JavaScript so a check can run it in Qt's own engine, as the shell does:
// the level it last settled, what its curve asks for the room it heard
// and whether it follows the room, from backlight-now; and its curve's
// points, from its learned file. Displays.qml reads the files and draws
// the screen's drawer.
.pragma library
.import "Plate.js" as Plate

// A fact a line, its name, a tab and its value, as the daemon writes them.
const FACT = /^([a-z]+)\t(.*)$/;
// A curve's point a line, its lux and its level, as the daemon writes it;
// a blank line or one opening with # is no point.
const SPACES = /\s+/;
const COMMENT = "#";
const FIELDS = 2;
const PERCENT = 100;
// A level still stepping to what the room asks is a point or more from it.
const STEPPING = 1;

// The level as a share of the whole, rounded as Brightness says it.
function share(level) {
    return Math.round(level * PERCENT);
}

// A vocabulary's words with each {name} in them filled from the values.
function fill(words, values) {
    return Object.keys(values).reduce((filled, name) =>
        filled.replace(`{${name}}`, values[name]), String(words ?? ""));
}

// What backlight-now says: { level, asks, following }, the levels shares
// of the whole and asks null while no light is heard; or null while it
// says nothing, as when the daemon has stopped and emptied it, or says
// it in a way the daemon never writes. `following` is the daemon's: light
// heard and the lid open, the curve setting the level; a pick is learned
// into the curve at once, so there's no pick held apart from it.
function now(text) {
    const said = {};
    for (const line of String(text ?? "").split("\n")) {
        const found = FACT.exec(line);
        if (found) {
            said[found[1]] = found[2];
        }
    }
    const level = Number(said.level);
    const asks = Number(said.asks);
    const known = said.level !== undefined && Number.isFinite(level)
        && (said.following === "yes" || said.following === "no");
    if (!known) {
        return null;
    }
    return {
        level,
        asks: said.asks !== undefined && Number.isFinite(asks) ? asks : null,
        following: said.following === "yes"
    };
}

// The curve's points, dimmest first: [{ lux, level }]. A file the daemon
// would refuse, any line of it not two figures, gives none, as the daemon
// falls back to its standard curve then.
function curve(text) {
    const lines = String(text ?? "").split("\n").map(line => line.trim())
        .filter(line => line !== "" && !line.startsWith(COMMENT));
    const points = lines.map(line => line.split(SPACES)).map(fields => ({
        whole: fields.length === FIELDS,
        lux: Number(fields[0]),
        level: Number(fields[1])
    }));
    const read = points.every(point => point.whole && Number.isFinite(point.lux)
        && Number.isFinite(point.level));
    return !read ? [] : points.map(point => ({ lux: point.lux, level: point.level }))
        .sort((one, other) => one.lux - other.lux);
}

// What sets the level, in the vocabulary's words: the room while the
// curve follows it, and what it asks while the level is still stepping
// there; "" while no light is heard or the lid is shut, where nothing does.
function follows(settled, said) {
    if (settled === null || !settled.following) {
        return "";
    }
    const stepping = settled.asks !== null
        && Math.abs(share(settled.asks) - share(settled.level)) >= STEPPING;
    return Plate.runs([said("the-room", {}),
        stepping ? said("on-its-way", { share: `${share(settled.asks)}%` }) : ""]);
}

// What the Room Light says: the room as the sensor hears it, or, where
// the daemon says it follows nothing, that no light is heard, so the
// drawer says why Follows is gone (VKCF3Z, 2026-11-08); "" where neither
// says anything.
function room(settled, heard, said, lux) {
    if (settled !== null && !settled.following) {
        return said("no-light", {});
    }
    return heard === null ? "" : lux(heard);
}

// Each of the curve's points as its share at its lux, a run apart,
// dimmest first: "15% at 0 lx · 38% at 6 lx".
function curveSaid(points, said, lux) {
    return Plate.runs(points.map(point =>
        said("at", { share: `${share(point.level)}%`, lux: lux(point.lux) })));
}
