.pragma library

// Wi-Fi's own row's drawer, at home or where Network lists it
// (Wireless.qml's placePlate), worked out here so tests/network-drawers.qml
// holds it in Qt's own engine. It leads with the network joined, then how
// strong its signal is and how long it's been joined, then the rest of what
// wifid serves in that network's drawer, in wifid's order; before that's
// read, how the network's row stands; joined to none, the networks in reach;
// off, that it's off (VKCF3Z, 2026-11-12: every row says a little more, its
// eyebrow saying where and what, "on this machine · wireless").

// `heard` is how Wi-Fi stands: `on`, whether the radio is; `networks`, the
// networks Wireless lists, by their `name`; and `joined`, the one joined as
// `{ name, side, drawn }`, its side as its row says it and `drawn` its
// drawer as ReadyRows draws it, or null until it's read, or `joined` null
// on none. `words` are the place's own `said` and `leading`, the facts'
// labels said first, in their order.
function plate(heard, words) {
    const { on, networks, joined } = heard;
    const { said, leading } = words;
    const eyebrow = said.eyebrow ?? "";
    if (joined === null) {
        return {
            eyebrow,
            title: (on ? said.unjoined : said.off) ?? "",
            body: networks.map(network => network.name).join(", "),
            facts: []
        };
    }
    if (!joined.drawn) {
        return { eyebrow, title: joined.name, body: joined.side ?? "", facts: [] };
    }
    const rank = fact => {
        const at = leading.indexOf(fact.label);
        return at < 0 ? leading.length : at;
    };
    // Sorted with each fact's place as the tie, so the rest keep wifid's
    // order whatever the engine's sort.
    const facts = (joined.drawn.facts ?? []).map((fact, at) => ({ fact, at }))
        .sort((a, b) => rank(a.fact) - rank(b.fact) || a.at - b.at)
        .map(each => each.fact);
    return Object.assign({}, joined.drawn, { eyebrow, title: joined.name, facts });
}
