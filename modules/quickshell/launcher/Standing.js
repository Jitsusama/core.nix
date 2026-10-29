// How the launcher's foot fits how things stand into the room it has.
// Each fact (the time, the date, the battery, the network, the volume, the
// day's AI spend, the agents) reads first as it glances, then as the least
// that tells Joel something; short of room still, the facts shrink to
// their glyph alone by collapse rank, lowest first, so each still shows
// it's there. A glyph never gives way: the marks have first claim on the
// foot, and what else is there makes room for them (Joel, 2026-10-28: "i
// want icons to be able to be there as much as possible"). A fact with
// no glyph, the date, gives way where it would have shrunk, and one with
// no rank, the time, never does. Apart from the
// launcher, so tests/answers.qml can hold it in Qt's own engine.
.pragma library

// The ways a fact reads, longest first.
const GLANCE = "glance";
const LEAST = "least";
const GLYPH = "glyph";

// Each fact as it reads its own way: a glyph that the fact before wears
// too is drawn once, and a fact left saying nothing is left out.
function told(facts) {
    return facts
        .map((fact, index) => {
            const shared = index > 0 && fact.sign !== "" && facts[index - 1].sign === fact.sign;
            const words = fact.way === GLYPH ? "" : fact[fact.way];
            const said = [shared ? "" : fact.sign, words].filter((part) => part !== "").join(" ");
            return Object.assign({}, fact, { said });
        })
        .filter((fact) => fact.said !== "");
}

function reading(facts, way) {
    return facts.map((fact) => Object.assign({}, fact, { way }));
}

function rankOf(fact) {
    return fact.entry.collapse ?? null;
}

// The collapse ranks the facts have, each once, lowest first.
function ranks(facts) {
    return facts
        .map(rankOf)
        .filter((rank, at, all) => rank !== null && all.indexOf(rank) === at)
        .sort((a, b) => a - b);
}

// Each way the facts can read, longest first: at a glance, then the
// least, then with each rank shrunk to its glyph, the last of which is
// the floor.
function stages(facts) {
    const order = ranks(facts);
    const out = [told(reading(facts, GLANCE))];
    let left = reading(facts, LEAST);
    for (const rank of order) {
        out.push(told(left));
        left = left
            .filter((fact) => rankOf(fact) !== rank || fact.sign !== "")
            .map((fact) => (rankOf(fact) === rank ? Object.assign({}, fact, { way: GLYPH }) : fact));
    }
    out.push(told(left));
    return out;
}

// The facts as the foot draws them, and which way they read, from the
// `from`th way on: the first that `fits` the room, or the shortest. The
// launcher starts each from the shortest it has had to read since it
// opened, so a fact growing by a digit flips every fact to its shortest
// once, and back never while it's open: "3 working" becoming "12
// working" and back moved the whole foot twice.
function fittedFrom(facts, fits, from) {
    const all = stages(facts);
    const start = Math.min(Math.max(0, from), all.length - 1);
    for (let stage = start; stage < all.length; stage += 1) {
        if (fits(all[stage])) {
            return { told: all[stage], stage };
        }
    }
    return { told: all[all.length - 1], stage: all.length - 1 };
}

// The facts as the foot draws them, each with what it `said`, given
// whether facts so told `fit` the room.
function fitted(facts, fits) {
    return fittedFrom(facts, fits, 0).told;
}
