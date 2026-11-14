.pragma library
// A pi session's drawer as Agents.qml shows it, from the record the
// agents daemon made ready (modules/agents/src/drawer.rs), which says
// everything in it but its times: those it sends as `{ at, as }`, so a
// drawer left open never changes on the daemon's side as a minute turns
// (Y2YBTS's ready rows, "Times"), and they're worded here by the clock.
// Its eyebrow names things, so the drawer sets it as it's written, never
// in capitals (Drawer.qml).

const DOT = " \u00b7 ";

// A time in a form the drawer names: `clock`, the time itself as
// `words.clock` says it; `since`, when it began, in `words.since`'s
// words ("since {at}"); or `dated`, the whole moment a landing's drawer
// says, "Saturday 7 November, 11:19", as `words.dated` says it.
function worded(time, words) {
    if (time.as === "dated") {
        return words.dated(time.at);
    }
    const clock = words.clock(time.at);
    return time.as === "since" ? words.since.replace("{at}", clock) : clock;
}

// What it says, the daemon's record with its times worded: each eyebrow
// item that's a time, and each fact's time after its value, and after its
// items too where it's given as items, an item whole of its own, so its
// lines break only between them. The eyebrow gives way by whole items,
// those it keeps least first (Reading.js).
function shown(drawer, words) {
    const eyebrowItems = (drawer.eyebrow ?? []).map(item => ({
        said: item.at ? worded(item.at, words) : String(item.said ?? ""),
        keep: item.keep
    }));
    const facts = (drawer.facts ?? []).map(fact => {
        if (!fact.at) {
            return fact;
        }
        // Its other fields as they are, its time said in its value.
        const told = {};
        for (const name of Object.keys(fact)) {
            if (name !== "at") {
                told[name] = fact[name];
            }
        }
        const when = worded(fact.at, words);
        told.value = [String(fact.value ?? ""), when].filter(part => part !== "").join(DOT);
        if (fact.valueItems) {
            told.valueItems = fact.valueItems.concat([{ said: when, whole: true }]);
        }
        return told;
    });
    return {
        eyebrow: eyebrowItems.map(item => item.said).filter(said => said !== "").join(DOT),
        eyebrowItems,
        eyebrowNames: true,
        title: String(drawer.title ?? ""),
        facts,
        body: String(drawer.body ?? ""),
        // Whether its words are prose, and whether its title and prose
        // are shown whole, as a landing asking Joel something is
        // (modules/agents/src/landed.rs).
        prose: drawer.prose === true,
        whole: drawer.whole === true,
        // Its identifiers, which Drawer.qml folds under z a.
        nerd: drawer.nerd ?? []
    };
}
