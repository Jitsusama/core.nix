// Agents' windows folded into one row of Windows (VKCF3Z's ruling, Joel
// driving its mock, 2026-11-11): every window an agent's session runs in
// gives way to the one row, which sits where the agents' window used last
// would, so the list still reads as used last, and Return on it goes to
// that window, as Super+Tab then Return always did. Tab steps into it,
// where the agents' windows are listed by the orchestration each session
// is under. Which windows are agents' the agents daemon says, by the
// windows daemon's own key for each (its `agent-window` records), so
// nothing here guesses. Windows.qml lists by it, and tests/agent-windows.qml
// runs it in Qt's own engine.
.pragma library

// The key the fold stands under among the windows' keys, said by a
// function, as a library's constants aren't read from QML.
const FOLD = "agents";
function foldKey() {
    return FOLD;
}

// The keys of `order`, used last first, as Windows lists them: every
// window not an agent's as it was, and the fold in the slot of the
// agents' window used last, `agents` saying by a window's key whose it
// is. `inside` is the agents' windows in the order used, `lead` the
// first of them, where Return goes, or null with no agents' window.
function folded(order, agents) {
    const listed = [];
    const inside = [];
    for (const key of order) {
        if (!(key in agents)) {
            listed.push(key);
            continue;
        }
        if (inside.length === 0) {
            listed.push(FOLD);
        }
        inside.push(key);
    }
    return { listed, inside, lead: inside.length > 0 ? inside[0] : null };
}

// The agents' windows `inside`, used last first, by the orchestration
// each session is under, each orchestration where its window used last
// would be, its orchestrator's own window first (`lead`, or null where
// it has none open) and the rest in the order used, and those under none
// together where theirs would. Each is `{ orchestration, lead, keys }`,
// its orchestration as the agents daemon says it, or null, as Agents
// lists a session under the one orchestrating it.
function byOrchestration(inside, agents) {
    const groups = [];
    const at = Object.create(null);
    for (const key of inside) {
        const over = agents[key].orchestration ?? null;
        const name = over === null ? "" : `:${over.address}`;
        if (!(name in at)) {
            at[name] = groups.length;
            groups.push({ orchestration: over, lead: null, keys: [] });
        }
        const group = groups[at[name]];
        if (over !== null && agents[key].address === over.address) {
            group.lead = key;
            group.keys.unshift(key);
        } else {
            group.keys.push(key);
        }
    }
    return groups;
}

// Each agents' window inside the fold with how far in it sits: a step in
// under its orchestrator's own window, and at none where there's none.
function depths(groups) {
    const out = [];
    for (const group of groups) {
        for (const key of group.keys) {
            out.push({ key, depth: group.lead !== null && key !== group.lead ? 1 : 0 });
        }
    }
    return out;
}

// What the fold's note says: the session of the window Return goes to,
// and how many more there are, in `said`'s words, as Words.qml has them,
// `{title}` and `{more}` in them.
function noted(inside, agents, said) {
    if (inside.length === 0) {
        return "";
    }
    const title = String(agents[inside[0]].title ?? "");
    if (inside.length === 1) {
        return title;
    }
    return said.replace("{title}", title).replace("{more}", String(inside.length - 1));
}

// What Windows' rows fold to, worked out whole: the rows by key, the
// keys listed, the lead's window id, how many inside need Joel, the
// note in `more`'s words, and a signature that moves only as what the
// row says does. Null when no agents' window is among the rows.
function foldOf(rows, agents, more) {
    const byKey = {};
    for (const row of rows) {
        byKey[`window:${row.id}`] = row;
    }
    const made = folded(Object.keys(byKey), agents);
    if (made.lead === null) {
        return null;
    }
    const needing = made.inside.filter((key) => byKey[key].waiting).length;
    const side = noted(made.inside, agents, more);
    const lead = byKey[made.lead].id;
    const said = JSON.stringify([made.inside, side, needing, lead]);
    return { byKey, listed: made.listed, inside: made.inside, lead, needing, side, said };
}

// How many of the fold's sessions need Joel, as Agents' own row counts
// them: nothing when none does, then "1 needs you", "2 need you", in
// `said`'s words (VKCF3Z, 2026-11-11 [e2a2ac1c]).
function needed(count, said) {
    if (count === 0) {
        return "";
    }
    return count === 1 ? said.needsCount : said.needCount.replace("{count}", String(count));
}

// The agents' windows by the windows daemon's key, from the agents
// daemon's `agent-window` records as held: a session whose window it
// couldn't pair (`window` null) is left out, never guessed at.
function agentsOf(records) {
    const agents = Object.create(null);
    for (const record of records) {
        if (typeof record?.window === "string" && record.window !== "") {
            agents[record.window] = record;
        }
    }
    return agents;
}
