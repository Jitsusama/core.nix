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

// What's said between a side's items, as the status line joins its facts.
const ITEMS_APART = " \u00b7 ";

// What a row says at its right, as items, given way whole: the least
// needed first, by each one's `keep`, until what's left fits `room`, and
// the rest said in their own order between dots. The one needed most is
// said whole even where it doesn't fit, so a side is never cut mid-word
// or slid from its left, and only then does the name give up letters
// (VKCF3Z, 2026-11-04: a session's tree goes, then its address, then
// its state, and what it cost stays).
function wholeItems(items, room, width) {
    return keptItems(items, room, width).join(ITEMS_APART);
}

// The items `wholeItems` keeps, each as it's said, in their own order.
function keptItems(items, room, width) {
    const kept = items.filter(item => item.said !== "");
    const joined = () => kept.map(item => item.said).join(ITEMS_APART);
    while (kept.length > 1 && width(joined()) > room) {
        const least = kept.reduce((low, item) => (item.keep < low.keep ? item : low));
        kept.splice(kept.indexOf(least), 1);
    }
    return kept.map(item => item.said);
}

// What a styled text needs written as its entity, so a side's words are
// drawn as they read.
const MARKUP = /[&<>]/g;
const ENTITIES = { "&": "&amp;", "<": "&lt;", ">": "&gt;" };

// Kept items as styled text, their dots in `dotColour` while the words
// keep the side's own ink: the status line's facts are joined so, and an
// agent row's "working \u00b7 $4.20" too, its dot muted on the cursor row
// (VKCF3Z, 2026-11-06).
function styledItems(said, dotColour) {
    const dot = ` <font color="${dotColour}">\u00b7</font> `;
    return said.map(words => words.replace(MARKUP, mark => ENTITIES[mark])).join(dot);
}

// What stands for whatever a crumb leaves out.
const FOLDED = "\u2026";

// The ways a folder's path reads, longest first, each shorter than the
// one before and each of whole folder names: whole, its middle folders
// as one mark, all but its last folder as one, and the mark alone, but
// for the place Joel is in, which keeps its last folder's name.
function pathWays(path, width, here) {
    const [whole, ...candidates] = pathTries(path, here);
    const ways = [whole];
    for (const way of candidates) {
        if (width(way) < width(ways[ways.length - 1])) {
            ways.push(way);
        }
    }
    return ways;
}

// Every way a folder's path may be said, whole first, before any is
// measured: down to its root and last folder, its last folder, and, where
// it isn't `here`, the mark alone.
function pathTries(path, here) {
    const parts = path.split("/").filter((part, at) => part !== "" || at === 0);
    const last = parts[parts.length - 1];
    const tries = [path];
    if (parts.length > 2) {
        tries.push(`${parts[0]}/${FOLDED}/${last}`);
    }
    tries.push(`${FOLDED}/${last}`);
    if (!here) {
        tries.push(FOLDED);
    }
    return tries;
}

// Every text crumbs and crumbsLeast may measure for `steps`, so each can
// be measured as it's drawn: each step's ways of giving way, the mark, and
// the first step in fewer words.
function crumbTexts(steps) {
    const texts = [FOLDED];
    for (const [at, step] of steps.entries()) {
        texts.push(...step.yields ? pathTries(step.text, at === steps.length - 1) : [step.text]);
    }
    if (steps.length > 1) {
        texts.push(...wordCuts(steps[0].text));
    }
    return Array.from(new Set(texts));
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

// How wide the path is at the least it says before its first step gives
// way: every step at its least, and the steps between the first and the
// last folded into one mark. The ask and where Joel is stay whole down to
// this, and what's typed scrolls in what's left (VKCF3Z, 2026-11-06).
function crumbsLeast(steps, width, between) {
    if (steps.length === 0) {
        return 0;
    }
    const least = (step, at) => (step.yields
        ? pathWays(step.text, width, at === steps.length - 1).pop() : step.text);
    const said = [least(steps[0], 0)];
    if (steps.length > 2) {
        said.push(FOLDED);
    }
    if (steps.length > 1) {
        said.push(least(steps[steps.length - 1], steps.length - 1));
    }
    return said.reduce((sum, text) => sum + width(text), 0) + (said.length - 1) * between;
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
    for (const cut of wordCuts(words)) {
        if (width(cut) <= room) {
            return cut;
        }
    }
    return "";
}

// Words cut at each whole word but the last, longest first, an ellipsis
// after each.
function wordCuts(words) {
    const parts = words.split(" ");
    const cuts = [];
    for (let count = parts.length - 1; count > 0; count -= 1) {
        cuts.push(`${parts.slice(0, count).join(" ").replace(DANGLING, "")}\u2026`);
    }
    return cuts;
}
