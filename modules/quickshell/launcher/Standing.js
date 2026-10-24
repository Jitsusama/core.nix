// How the launcher's foot fits how things stand into the room it has.
// Each fact (the time, the date, the battery, the network, the volume, the
// day's AI spend, the agents) reads first as it glances, then as the least
// that tells Joel something; short of room still, the facts shrink to
// their glyph alone by collapse rank, lowest first, so each still shows
// it's there; and only once every one is its glyph do they give way, in
// the same order. A fact with no glyph, the date, gives way where it would
// have shrunk, and one with no rank, the time, never does. Apart from the
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

// The facts as the foot draws them, each with what it `said`, given
// whether facts so told `fit` the room.
function fitted(facts, fits) {
    const whole = told(reading(facts, GLANCE));
    if (fits(whole)) {
        return whole;
    }
    const order = ranks(facts);
    let left = reading(facts, LEAST);
    for (const rank of order) {
        const now = told(left);
        if (fits(now)) {
            return now;
        }
        left = left
            .filter((fact) => rankOf(fact) !== rank || fact.sign !== "")
            .map((fact) => (rankOf(fact) === rank ? Object.assign({}, fact, { way: GLYPH }) : fact));
    }
    for (const rank of order) {
        const now = told(left);
        if (fits(now)) {
            return now;
        }
        left = left.filter((fact) => rankOf(fact) !== rank);
    }
    return told(left);
}
