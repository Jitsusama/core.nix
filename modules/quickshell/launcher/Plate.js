.pragma library
// What a thing's drawer says of its parts, the rows a step into it, so a
// device or a screen whose source holds more than its line shows all of it
// beside the card, as a time server's drawer does (Clocks.qml): each part's
// name as a fact's label and what it says as the value, a level's bar as
// Meter's stroke after it and a history's spark as itself. An act or a
// choice (its key `verb:`) is something to do, not a figure, so it isn't
// said; a part holding figures of its own, as a sound device's Hardware or
// Channels does, is opened in place, its figures each a fact of their own
// or its items joined into its value. Plain JavaScript, so a check runs it
// in Qt's own engine (tests/plate.qml), as the shell does.

// Two items of one value, as a figure's runs are set apart
// (docs/design.md, The Drawer: "38 s · 4.1k out").
const APART = " \u00b7 ";
// What a row says while it's still being read, which the drawer shows as
// its fixed placeholder rather than as words.
const READING = "\u2026";

// Whether a row is something to do rather than a part: an act or a choice.
function acts(row) {
    return String(row?.key ?? "").startsWith("verb:");
}

// What a row says at its right, as words, or null while it's being read.
function said(row) {
    const side = row?.side ?? "";
    const words = typeof side === "function" ? side() : side;
    const text = String(words ?? "");
    return text === READING ? null : text;
}

// A row's graph as a fact's figure: a level's bar as Meter's stroke, a
// history's spark as it is; a strip of members has no figure in a fact.
function figureOf(graph) {
    if (!graph) {
        return null;
    }
    if (graph.shape === "bar") {
        return { shape: "stroke", values: graph.values ?? [], warn: graph.warn === true };
    }
    return graph.shape === "spark" ? graph : null;
}

// A row's items as one value: each its name and what it says, a run
// apart, after what the row itself says, when it says anything.
function joined(lead, rows) {
    const items = rows.filter(row => !!row && !acts(row))
        .map(row => [String(row.name ?? ""), said(row) ?? ""].filter(part => part !== "")
            .join(" "))
        .filter(item => item !== "");
    return [lead ?? ""].filter(part => part !== "").concat(items).join(APART);
}

// The facts a thing's parts make, in their order. `how(row)` says, for a
// part, how it's said where that isn't its name and side alone, or null
// for that: { leave: true } to leave it out, { value } for its value,
// { facts: rows } for its rows a step in, each a fact of its own in its
// place, or { items: rows } for those joined into its value. A part that
// says nothing is left out, and so is one only being read that has no
// name to hold its place.
function facts(parts, how) {
    const out = [];
    for (const row of parts ?? []) {
        if (!row || acts(row)) {
            continue;
        }
        const told = how ? how(row) : null;
        if (told?.leave) {
            continue;
        }
        if (told?.facts) {
            out.push(...facts(told.facts, how));
            continue;
        }
        const own = said(row);
        const value = told?.value !== undefined ? told.value
            : told?.items ? joined(own, told.items) : own;
        const label = String(row.name ?? "");
        if (label === "" || value === "") {
            continue;
        }
        const fact = { label, value };
        const figure = told?.figure !== undefined ? told.figure : figureOf(row.graph);
        if (figure) {
            fact.figure = figure;
        }
        out.push(fact);
    }
    return out;
}

// A drawer's facts with those naming what identifies its thing, the
// labels given, moved to its end and marked `identifies`, so they're set
// in the Name face, as identifiers end every drawer (VKCF3Z, 2026-11-08).
// Two sources saying one, as niri and the EDID both say a screen's
// serial, are said once, the first.
function identified(drawn, labels) {
    const known = drawn.filter(fact => labels.includes(fact.label))
        .filter((fact, at, all) => all.findIndex(each => each.label === fact.label) === at)
        .map(fact => Object.assign({}, fact, { identifies: true }));
    return drawn.filter(fact => !labels.includes(fact.label)).concat(known);
}

// Each value of a list, a run apart, the empty ones left out.
function runs(values) {
    return values.map(value => String(value ?? "")).filter(value => value !== "").join(APART);
}
