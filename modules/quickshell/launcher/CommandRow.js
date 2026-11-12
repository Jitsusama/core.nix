.pragma library

// What a command's row says in its drawer, beside the line that names its
// form (Joel, 2026-11-11: "side drawers for just about every item giving
// me more details, not as much as if i opened it up"): what typing it
// answers, in a sentence, then one or two worked examples, each what's
// typed as its label and what comes back as its value (Y2YBTS, f7ea4559).
// Both are the vocabulary's fixed words, never worked out or looked up
// while it's shown, so the drawer costs a key nothing, and an example
// naming a network takes a documentation address (RFC 5737, RFC 3849),
// whose answer no owner can change.

// A command's drawer's eyebrow, place then thing as every drawer's is: the
// place that lists it, from the vocabulary's categories, filled into
// `said.in`, or `said.anywhere` for one offered wherever it's typed
// (VKCF3Z on #3781, 2026-11-12).
function eyebrow(command, categories, said) {
    const place = categories.find(category => (category.commands ?? []).includes(command.id));
    return place ? String(said.in ?? "").replace("{place}", place.name) : said.anywhere ?? "";
}

// A command's drawer from its entry in the vocabulary's commands, under
// `eyebrow`, or null for one the vocabulary gives nothing more of.
function drawer(command, eyebrow) {
    const examples = command.examples ?? [];
    const reads = command.reads ?? "";
    if (examples.length === 0 && reads === "") {
        return null;
    }
    return {
        eyebrow,
        title: `${command.start ?? ""}${command.form ?? ""}`.trim(),
        body: reads,
        facts: examples.map(one => ({ label: one.typed, value: one.answers }))
    };
}
