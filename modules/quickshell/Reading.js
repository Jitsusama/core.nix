// How a name too long for its row slides to show its end (Choice.qml): a
// reading motion, steady with soft ends, not an arrival. It holds one
// reading pace from end to end but for a short ramp at each, where it
// gathers speed from rest and settles back to it, so it never starts or
// stops hard and the eye follows one speed (Joel, 2026-10-29: the slide
// was too slow and started and stopped abruptly; VKCF3Z's [e804ce8d]).
// Tested as it is by tests/reading.qml.
.pragma library

// What a cut leaves dangling at its end, a separator or a pause, which
// goes before the ellipsis does.
const DANGLING = /[\s\u00b7,;:]+$/u;

// The longest each ramp lasts, in seconds. A slide the pace alone would
// take less than two ramps for ramps for half that time, so the pace
// still holds between them.
const RAMP = 0.2;

// The slide over `over` pixels at `pace` pixels a second: how long it
// lasts, in seconds, and the easing curve QML's BezierSpline takes, as
// three pieces: speeding up evenly, the pace held, and slowing evenly.
// Speeding up evenly from rest to the pace covers half the ground the
// pace would in that time, so the slide lasts a ramp longer than the
// pace alone would take, and the pieces meet at the same speed.
function slide(over, pace) {
    if (over <= 0 || pace <= 0) {
        return { seconds: 0, curve: [1 / 3, 1 / 3, 2 / 3, 2 / 3, 1, 1] };
    }
    const steady = over / pace;
    const ramp = Math.min(RAMP, steady / 2);
    const seconds = steady + ramp;
    // The share of the time each ramp takes, and of the ground.
    const time = ramp / seconds;
    const ground = pace * ramp / 2 / over;
    const third = (from, to, part) => from + (to - from) * part;
    return {
        seconds,
        curve: [
            time / 3, 0, 2 * time / 3, ground / 3, time, ground,
            third(time, 1 - time, 1 / 3), third(ground, 1 - ground, 1 / 3),
            third(time, 1 - time, 2 / 3), third(ground, 1 - ground, 2 / 3),
            1 - time, 1 - ground,
            1 - 2 * time / 3, 1 - ground / 3, 1 - time / 3, 1, 1, 1
        ]
    };
}

// What a row says at its right, as items, given way whole: the least
// needed first, by each one's `keep`, until what's left fits `room`, and
// the rest said in their own order between dots. The one needed most is
// said whole even where it doesn't fit, so a side is never cut mid-word
// or slid from its left, and only then does the name give up letters
// (VKCF3Z, 2026-11-04: a session's tree goes, then its address, then
// its state, and what it cost stays).
function wholeItems(items, room, width) {
    const said = items.filter(item => item.said !== "");
    const kept = said.slice();
    const joined = () => kept.map(item => item.said).join(" \u00b7 ");
    while (kept.length > 1 && width(joined()) > room) {
        const least = kept.reduce((low, item) => (item.keep < low.keep ? item : low));
        kept.splice(kept.indexOf(least), 1);
    }
    return joined();
}

// What stands for whatever a crumb leaves out.
const FOLDED = "\u2026";

// The ways a folder's path reads, longest first, each shorter than the
// one before and each of whole folder names: whole, its middle folders
// as one mark, all but its last folder as one, and the mark alone, but
// for the place Joel is in, which keeps its last folder's name.
function pathWays(path, width, here) {
    const parts = path.split("/").filter((part, at) => part !== "" || at === 0);
    const last = parts[parts.length - 1];
    const ways = [path];
    const candidates = parts.length > 2 ? [`${parts[0]}/${FOLDED}/${last}`] : [];
    candidates.push(`${FOLDED}/${last}`);
    if (!here) {
        candidates.push(FOLDED);
    }
    for (const way of candidates) {
        if (width(way) < width(ways[ways.length - 1])) {
            ways.push(way);
        }
    }
    return ways;
}

// The field's path, its steps given way at whole names so it fits `room`
// with `between` beside each step after the first, never a piece of a
// name (VKCF3Z, 2026-11-05: "~\u2026e"). A folder's path, a step that
// `yields`, gives way first, a folder at a time; then the steps between
// the first and the last fold into one mark, those nearest the first
// going first; then the first says fewer words, or joins the mark. The
// last, where Joel is, is said whole, or a folder's path there down to
// its last folder. Gives what each drawn step
// `said`, and whether each step is said `whole`.
function crumbs(steps, room, width, between) {
    if (steps.length === 0) {
        return { said: [], whole: [] };
    }
    const wide = said => said.reduce((sum, text) => sum + width(text), 0)
        + Math.max(0, said.length - 1) * between;
    const ways = steps.map((step, at) => (step.yields
        ? pathWays(step.text, width, at === steps.length - 1) : [step.text]));
    const deepest = Math.max(...ways.map(way => way.length));
    // Each step as it reads at `depth`, then with `folded` steps after the
    // first folded into one mark, merged with a mark beside it.
    const read = (depth, folded, first) => {
        const texts = ways.map(way => way[Math.min(depth, way.length - 1)]);
        if (first !== undefined) {
            texts[0] = first;
        }
        const said = [];
        const whole = [];
        texts.forEach((text, at) => {
            const fold = text === FOLDED || (at > 0 && at <= folded && at < texts.length - 1);
            whole.push(!fold && text === steps[at].text);
            if (!(fold && said[said.length - 1] === FOLDED)) {
                said.push(fold ? FOLDED : text);
            }
        });
        return { said, whole };
    };
    for (let depth = 0; depth < deepest; depth += 1) {
        const tried = read(depth, 0);
        if (wide(tried.said) <= room) {
            return tried;
        }
    }
    const least = deepest - 1;
    for (let folded = 1; folded < steps.length - 1; folded += 1) {
        const tried = read(least, folded);
        if (wide(tried.said) <= room) {
            return tried;
        }
    }
    // The first step in fewer words, in the room the rest leave it.
    const rest = read(least, steps.length - 2, "");
    const others = rest.said.slice(1);
    const left = room - wide(others) - (others.length > 0 ? between : 0);
    const fewer = steps.length > 1 ? wholeWords(steps[0].text, left, width) : steps[0].text;
    return read(least, steps.length - 2, fewer === "" ? FOLDED : fewer);
}

// Words too long for their room, given way at the last whole word that
// fits, an ellipsis after it, so what's said is never cut mid-word
// (docs/design.md, Collapsing; VKCF3Z, 2026-11-02: "Open-Meteo didn't
// a\u2026"). `width` measures a text in the face it's drawn in. Where not
// even the first word fits, nothing is said, rather than a piece of one.
function wholeWords(words, room, width) {
    if (width(words) <= room) {
        return words;
    }
    const parts = words.split(" ");
    for (let count = parts.length - 1; count > 0; count -= 1) {
        const cut = `${parts.slice(0, count).join(" ").replace(DANGLING, "")}\u2026`;
        if (width(cut) <= room) {
            return cut;
        }
    }
    return "";
}
