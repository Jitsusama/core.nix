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
// A figure as Rust's parse takes one: decimal, never hex as Number takes.
const DECIMAL = /^[+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?$/;
// The curve's levels lie between these, and its lux are never below none,
// as Curve::through keeps them.
const DIMMEST = 0.02;
const BRIGHTEST = 1;
const NO_LUX = 0;
const PERCENT = 100;
// The daemon moves the level only for a gap this wide or wider
// (steer.rs's SLIGHT), so a narrower one is where it rests.
const SLIGHT = 0.03;

// The level as a share of the whole, rounded as Brightness says it.
function share(level) {
    return Math.round(level * PERCENT);
}

// A vocabulary's words with each {name} in them filled from the values.
function fill(words, values) {
    return Object.keys(values).reduce((filled, name) =>
        filled.replace(`{${name}}`, values[name]), String(words ?? ""));
}

// A figure the daemon would read, or NaN.
function figure(text) {
    return DECIMAL.test(String(text ?? "")) ? Number(text) : Number.NaN;
}

// What backlight-now says: { level, lux, asks, following }, the levels
// shares of the whole, lux and asks null while no light is heard; or null while it
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
    const level = figure(said.level);
    const known = Number.isFinite(level)
        && (said.following === "yes" || said.following === "no");
    if (!known) {
        return null;
    }
    const finite = value => Number.isFinite(value) ? value : null;
    return {
        level,
        lux: finite(figure(said.lux)),
        asks: finite(figure(said.asks)),
        following: said.following === "yes"
    };
}

// The curve's points, dimmest first: [{ lux, level }], each kept where
// the daemon keeps it. A file the daemon would refuse, any line of it not
// two figures, gives none, as the daemon falls back to its standard curve
// then.
function curve(text) {
    const lines = String(text ?? "").split("\n").map(line => line.trim())
        .filter(line => line !== "" && !line.startsWith(COMMENT));
    const points = lines.map(line => line.split(SPACES)).map(fields => ({
        whole: fields.length === FIELDS,
        lux: figure(fields[0]),
        level: figure(fields[1])
    }));
    const read = points.every(point => point.whole && Number.isFinite(point.lux)
        && Number.isFinite(point.level));
    return !read ? [] : points.map(point => ({
        lux: Math.max(point.lux, NO_LUX),
        level: Math.min(Math.max(point.level, DIMMEST), BRIGHTEST)
    })).sort((one, other) => one.lux - other.lux);
}

// What sets the level, in the vocabulary's words: the room while the
// curve follows it, and what it asks while the level is still stepping
// there; "" while no light is heard or the lid is shut, where nothing does.
function follows(settled, said) {
    if (settled === null || !settled.following) {
        return "";
    }
    // Compared raw, as the daemon compares them.
    const stepping = settled.asks !== null
        && Math.abs(settled.asks - settled.level) >= SLIGHT;
    return Plate.runs([said("the-room", {}),
        stepping ? said("on-its-way", { share: `${share(settled.asks)}%` }) : ""]);
}

// What the Room Light says: the room as the sensor hears it now, or as
// the daemon last heard it; or, where the daemon says it heard no light,
// that none is, so the drawer says why Follows is gone (VKCF3Z,
// 2026-11-08). A shut lid still hears the room, so it's said then too.
// "" where nothing says anything.
function room(settled, heard, said, lux) {
    if (heard !== null) {
        return lux(heard);
    }
    if (settled === null) {
        return "";
    }
    return settled.lux !== null ? lux(settled.lux) : said("no-light", {});
}

// Each of the curve's points as its share at its lux, a run apart,
// dimmest first: "15% at 0 lx · 38% at 6 lx".
function curveSaid(points, said, lux) {
    return Plate.runs(points.map(point =>
        said("at", { share: `${share(point.level)}%`, lux: lux(point.lux) })));
}
