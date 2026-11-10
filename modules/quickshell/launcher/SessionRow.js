.pragma library
// A pi session as the launcher shows it, said once, and named apart from
// Session.qml, which is the desktop session: how it stands, the acts it
// offers in their order, and its drawer's figures (SessionDrawer.js says
// the rest of its drawer), wherever the launcher shows it (Agents, an orchestrator's row, a quest's Go To Its Session).
// Joel, 2026-10-28: "we need to not have different versions of these sort
// of elements. they need to be treated the same everywhere." The providers
// turn what this says into rows, each through its own `thing`.

// What a session is, as the launcher's components name their kinds.
const KIND = "session";

// How a running session can stand, in the one order every list of
// sessions keeps: what only Joel can move first, then what's under way,
// then what waits for work, then what he put down, then what gave its
// memory back until it's wanted (the harness's park-workflow). One idle
// is listening, never waiting, which is only ever said of something
// waiting on Joel.
const STATES = ["needs", "working", "inLine", "listening", "paused", "parked"];

// How a session stands, from its presence, whether its want is Joel's
// (`asks`) and whether it waits in the queue (`queued`); "off" with none.
// A want not his reads as what it does; one parked stays parked, its want
// For You's (Sessions.wanting), since Wake is its only act.
function state(presence, asks, queued) {
    if (!presence) {
        return "off";
    }
    if (presence.state === "parked") {
        return "parked";
    }
    if (asks) {
        return "needs";
    }
    if (presence.state === "paused") {
        return "paused";
    }
    if (queued) {
        return "inLine";
    }
    return presence.busy || presence.state === "busy" ? "working" : "listening";
}

// Where a state stands in that order; one not running after them all.
function rank(standing) {
    const at = STATES.indexOf(standing);
    return at < 0 ? STATES.length : at;
}

// The deepest a row sits under another (docs/design.md keeps no layer
// more than two deep).
const DEEPEST = 2;

// The fewest waiting members an orchestration folds.
const FOLDS_AT = 2;

// Sessions laid out as Agents shows them, from [{ address, under, state }]
// in the order Sessions keeps: each under the session its `under` names,
// when that one's listed, as [{ at, depth, under, orchestrates }], `at` its
// place in what was given, `under` the address of the row it's drawn under
// or "", and `orchestrates` whether any row is under it. Siblings stand in
// the order of how they stand, then the order given; one with rows under
// it stands where its most urgent row would, so an orchestration moves as
// one. Orchestrators naming each other are cut where the first is met.
//
// With `folds`, the members of one orchestration that only wait, those
// listening, paused or parked with nothing under them more urgent, fold
// into one entry after its own rows (VKCF3Z, 2026-11-06), so a tree of
// sixty sessions shows what moves: `{ fold: true, depth, under, members,
// counts }`, `members` laid as the tree is, from depth 0, and `counts` how
// many of each state it holds, in the one order, as [{ state, count }]. A
// lone one stays a row, since folding it saves none and hides its name.
function tree(sessions, folds) {
    const byAddress = {};
    for (const [at, one] of sessions.entries()) {
        if (byAddress[one.address] === undefined) {
            byAddress[one.address] = at;
        }
    }
    const parent = sessions.map((one, at) => {
        const found = one.under ? byAddress[one.under] : undefined;
        return found === undefined || found === at ? -1 : found;
    });
    for (const at of parent.keys()) {
        const seen = new Set([at]);
        for (let up = parent[at]; up >= 0; up = parent[up]) {
            if (seen.has(up)) {
                parent[at] = -1;
                break;
            }
            seen.add(up);
        }
    }
    const children = sessions.map(() => []);
    for (const [at, up] of parent.entries()) {
        if (up >= 0) {
            children[up].push(at);
        }
    }
    // Whether one [rank, place] is more urgent than another.
    const before = (one, other) => one[0] < other[0] || one[0] === other[0] && one[1] < other[1];
    const compared = (one, other) => {
        if (before(one, other)) {
            return -1;
        }
        return before(other, one) ? 1 : 0;
    };
    // The most urgent of a row and those under it, as [rank, place].
    const best = [];
    const bestOf = at => {
        if (best[at] === undefined) {
            best[at] = children[at].map(bestOf).reduce((most, other) =>
                before(other, most) ? other : most, [rank(sessions[at].state), at]);
        }
        return best[at];
    };
    const inOrder = list => list.slice().sort((a, b) => compared(bestOf(a), bestOf(b)));
    const waits = at => bestOf(at)[0] >= rank("listening");
    // Every session a fold holds, one folded within it too.
    const held = members => members.reduce((all, one) =>
        all.concat(one.fold ? held(one.members) : [one.at]), []);
    const counted = members => STATES.map(each => ({ state: each,
        count: held(members).filter(at => sessions[at].state === each).length }))
        .filter(one => one.count > 0);
    const lay = (into, at, depth) => {
        const up = parent[at];
        into.push({ at, depth: Math.min(depth, DEEPEST),
            under: up >= 0 ? sessions[up].address : "", orchestrates: children[at].length > 0 });
        const under = inOrder(children[at]);
        const waiting = folds ? under.filter(waits) : [];
        const folded = waiting.length >= FOLDS_AT ? waiting : [];
        for (const child of under.filter(one => !folded.includes(one))) {
            lay(into, child, depth + 1);
        }
        if (folded.length > 0) {
            const members = [];
            for (const child of folded) {
                lay(members, child, 0);
            }
            into.push({ fold: true, depth: Math.min(depth + 1, DEEPEST),
                under: sessions[at].address, members, counts: counted(members) });
        }
    };
    const laid = [];
    for (const at of inOrder([...parent.keys()].filter(root => parent[root] < 0))) {
        lay(laid, at, 0);
    }
    return laid;
}


// The acts a running session offers, by their vocabulary's ids, in the
// order every kind keeps (docs/design.md, the order of actions): what
// leaves it as it is before what changes it, and what ends it last. Every
// session on the mesh reads its steer inbox, so each can be talked to,
// paused, resumed and stopped; one with no window offers no Go To, and
// only one under way can be stopped. One parked has no process to pause
// or stop and no window known to go to: it can be woken, or talked to,
// which wakes it too. Only one here can be closed, last, since the
// harness ends a session on a close Joel steers it from this machine and
// drops one a node relays (harness #1023).
function acts(standing, windowed, here) {
    if (standing === "parked") {
        return ["wake", "talk"];
    }
    const ids = windowed ? ["switch"] : [];
    ids.push("talk");
    ids.push(standing === "paused" ? "resume" : "pause");
    if (standing === "working" || standing === "inLine") {
        ids.push("stop");
    }
    if (here) {
        ids.push("close");
    }
    return ids;
}

// What Close's ask says stays, so ending a session never reads as losing
// its work (VKCF3Z, 2026-11-06, in to-dos since 2026-11-08: "Close
// 8IDU4E? Its worktree and 1 to-do stay."): its worktree, or its folder
// where it works outside one, and its quest's to-dos, by their count, in the agents'
// words (`said`).
function leaves(inTree, needs, said) {
    const place = inTree ? said.tree : said.folder;
    if (needs === 0) {
        return String(said.stays ?? "").replace("{kept}", place);
    }
    const items = String((needs === 1 ? said.toDo : said.toDos) ?? "")
        .replace("{count}", needs);
    return String(said.stay ?? "").replace("{kept}", `${place} ${said.and} ${items}`);
}

// The act Return does, one of `acts`: the one its state makes harmless
// (VKCF3Z, 2026-11-06). Go To one at work or waiting on Joel, Talk to one
// listening, Resume one paused and Wake one parked; one with no window
// to go to is talked to. Stop and Close end it, so they are never
// Return's.
function returns(standing, windowed) {
    if (standing === "parked") {
        return "wake";
    }
    if (standing === "paused") {
        return "resume";
    }
    return windowed && standing !== "listening" ? "switch" : "talk";
}

// The theme's role its glyph is drawn in: a program's, as a live one
// is, and muted for one paused, parked or closed, which never looks live
// (#2052; VKCF3Z, 2026-11-07), colour going on glyphs while its name
// stays in text and the far-end word says how it stands.
function hue(stands, warns) {
    if (["paused", "parked", "off"].includes(stands)) {
        return "muted";
    }
    // A live one with a tier 2 figure is nearing a limit, in the warning's
    // hue, as a budget near its allowance is; action would say a thing to do
    // (VKCF3Z, 2026-11-07).
    return warns ? "warning" : "program";
}

// When the session a quest resumes by was last live, in milliseconds, from
// what the quest index says of it ({ id, cwd, at }): its `at`, the harness's
// own word for it, else the start its id holds as a UUIDv7's first 48
// bits, which is when it began rather than when it was last live, else 0.
const DASHES = /-/g;
const UUID_V7 = /^[0-9a-f]{12}7/i;
const STAMP_DIGITS = 12;
const HEX = 16;
function liveAt(resume) {
    const said = Date.parse(resume?.at ?? "");
    if (!Number.isNaN(said)) {
        return said;
    }
    const hex = String(resume?.id ?? "").replace(DASHES, "");
    return UUID_V7.test(hex) ? Number.parseInt(hex.slice(0, STAMP_DIGITS), HEX) : 0;
}

// Addresses from [{ address, at }], the one last live first, those
// equally live in the order given (VKCF3Z, 2026-11-07 [9a4b1dd0]: the one
// Joel last used is the one he's likeliest to want).
function lastLiveFirst(ones) {
    return ones.map((one, given) => ({ one, given }))
        .sort((a, b) => b.one.at - a.one.at || a.given - b.given)
        .map(each => each.one.address);
}

// What a session's row says at its right, each item with how much it's
// needed, so where room runs short they give way whole (Choice.qml): how
// it stands, then what it spent today, last and kept longest, and nothing
// more (VKCF3Z, 2026-11-06: "working \u00b7 $4.20"). Its address and
// folder are its drawer's eyebrow, and typing its address still finds it.
// A tier 2 figure leads while it's true, "84% full \u00b7 working \u00b7
// $4.20", and is kept longest, since it's why the row asks for a look;
// beside it how the session stands outlasts the money, which gives way
// first, since the drawer and Today say the spend (VKCF3Z, 2026-11-07).
function side(doing, spent, filled) {
    const tier = filled ?? "";
    const [stands, costs] = tier !== "" ? [4, 3] : [3, 4];
    return [{ said: tier, keep: 5 }, { said: doing, keep: stands }, { said: spent, keep: costs }]
        .filter(item => item.said !== "");
}

// How full a session's context is, as a whole percent of its window, from
// its record's `usage.context` ({ used, limit }), the harness's count of
// what the next turn sends; 0 where it isn't said. Past FULL_AT it's tier 2
// on its row (the design's figures, 2026-11-06): a compaction is near, and
// each turn sends most of a window.
const FULL_AT = 80;
const PERCENT = 100;
function fullness(usage) {
    const used = Number(usage?.context?.used ?? 0);
    const limit = Number(usage?.context?.limit ?? 0);
    return limit > 0 && used > 0 ? Math.floor(used / limit * PERCENT) : 0;
}

// What its row says of how full it is, in the vocabulary's words, only past
// FULL_AT; "" below it.
function full(usage, said) {
    const share = fullness(usage);
    return share >= FULL_AT ? String(said.full ?? "").replace("{share}", share) : "";
}

// A count of tokens as a figure reads it: whole under a thousand, then
// thousands and millions, one decimal while that's under ten ("4.1k",
// "142k", "1.2M", "1M").
const THOUSAND = 1000;
const MILLION = 1000000;
const TENS = 10;
const ZERO_DECIMAL = /\.0$/;
function tokens(count) {
    const many = Number(count ?? 0);
    const scaled = (by, unit) => {
        const value = many / by;
        const shown = value < TENS ? value.toFixed(1).replace(ZERO_DECIMAL, "")
            : String(Math.round(value));
        return `${shown}${unit}`;
    };
    if (many < THOUSAND) {
        return String(Math.round(many));
    }
    return Math.round(many / THOUSAND) < THOUSAND ? scaled(THOUSAND, "k") : scaled(MILLION, "M");
}

// How long a turn took, from milliseconds: tenths of a second under ten,
// whole seconds under a minute, then minutes.
const SECOND_MS = 1000;
const MINUTE_S = 60;
function took(ms) {
    const seconds = Number(ms ?? 0) / SECOND_MS;
    if (seconds < TENS) {
        return `${seconds.toFixed(1).replace(ZERO_DECIMAL, "")} s`;
    }
    return seconds < MINUTE_S ? `${Math.round(seconds)} s` : `${Math.round(seconds / MINUTE_S)} min`;
}

// The drawer's figures for a session, by OpenTelemetry's gen_ai names as
// its record's `usage` keeps them (the design's figures, tier 3): how
// full its context is and its last turn first, the two that move as it
// works, then its model and today's turns, tokens and cache, those only while the record's day is `today`, since the
// harness starts them over each day. No amount is among them, so sharing
// the screen hides none; what it spent is spend-watch's, said beside them.
// `said` is the vocabulary's agents words, `named` names each figure.
const GEN_AI = "gen_ai.usage.";
function figures(usage, today, said, named) {
    if (!usage) {
        return [];
    }
    const fill = (key, parts) => Object.keys(parts)
        .reduce((text, part) => text.replace(`{${part}}`, parts[part]), String(said[key] ?? ""));
    const used = (spent, kind) => Number(spent?.[`${GEN_AI}${kind}`] ?? 0);
    const model = usage["gen_ai.response.model"] ?? usage["gen_ai.request.model"] ?? "";
    const context = usage.context?.limit > 0 && usage.context?.used > 0
        ? fill("contextOf", { used: tokens(usage.context.used), limit: tokens(usage.context.limit) })
        : "";
    const last = usage.turns?.last;
    const lastOut = used(usage.last, "output_tokens");
    const turn = last?.duration_ms > 0
        ? fill("turnTook", { took: took(last.duration_ms), out: tokens(lastOut) })
            + (last.first_token_ms > 0
                ? ` \u00b7 ${fill("firstToken", { took: took(last.first_token_ms) })}` : "")
        : "";
    const day = usage.day === today ? usage.today : null;
    const sent = used(day, "input_tokens") + used(day, "cache_read.input_tokens")
        + used(day, "cache_creation.input_tokens");
    const read = sent > 0 ? used(day, "cache_read.input_tokens") / sent * PERCENT : 0;
    const known = {
        context,
        "last-turn": turn,
        model,
        turns: usage.day === today && usage.turns?.today > 0 ? String(usage.turns.today) : "",
        tokens: day && sent > 0 ? fill("tokensOf", {
            sent: tokens(sent), out: tokens(used(day, "output_tokens"))
        }) : "",
        cache: day && sent > 0 ? fill("cacheRead", { share: read.toFixed(1) }) : ""
    };
    // Context carries Meter's stroke of how full, warning from FULL_AT as
    // its row's glyph does (VKCF3Z, 2026-11-07).
    const stroke = { shape: "stroke", values: [usage.context?.used / usage.context?.limit],
        warn: fullness(usage) >= FULL_AT };
    return Object.keys(known).filter(id => known[id] !== "").map(id => Object.assign(
        { label: named(id), value: known[id] }, id === "context" ? { figure: stroke } : {}));
}

// How the sessions stand together, from how each stands (`states`) and
// what they all spent today as spend-watch sums it (`spent`, "" while
// it's unsaid or the screen is shared): those needing Joel first, as what
// waits on him comes first everywhere it's said, then those working, then
// the day's dollars, "1 needs you \u00b7 3 working \u00b7 $214 today", as
// Home's Agents row and the row found by name both say it (VKCF3Z,
// 2026-11-07). With none needing him nor working, it names how the rest
// stand in the fold's words and order, "3 listening, 1 paused", the parked
// not among them, and nothing for what isn't there. `said` is the
// vocabulary's agents words.
function summary(states, spent, said) {
    const count = standing => states.filter(one => one === standing).length;
    const filled = (key, fill) => String(said[key] ?? "").replace("{count}", fill);
    const working = count("working");
    const needing = count("needs");
    const rest = working === 0 && needing === 0 ? ["inLine", "listening", "paused"]
        .filter(standing => count(standing) > 0)
        .map(standing => standing === "inLine" ? filled("inLineCount", count(standing))
            : `${count(standing)} ${said[standing] ?? standing}`)
        .join(", ") : "";
    const parts = [
        needing === 1 ? filled("needsCount", 1) : needing > 1 ? filled("needCount", needing)
            : "",
        working > 0 ? filled("workingCount", working) : "",
        rest,
        spent !== "" ? String(said.today ?? "").replace("{spent}", spent) : ""
    ];
    return parts.filter(part => part !== "").join(" \u00b7 ");
}

// Agents' Recent: the sessions that closed this boot (Sessions.closed),
// as Recent.js lists them, by id and when each ended (RSCH-20261030-GB56HX),
// its `ended` in milliseconds as Sessions keeps it.
function closedRecent(closed) {
    return (closed ?? []).map(one => ({ key: `session:${one.id}`, at: one.ended, row: one }));
}

// Those closed Agents shows already, as closedRecent keys them: a closed
// orchestrator or one running again by address, one to reopen by id, or
// one whose quest a live session works on, which reopening would double.
function shownElsewhere(closed, toReopen, addresses, quests) {
    const reopening = new Set((toReopen ?? []).map(one => String(one.id)));
    const listed = new Set(addresses ?? []);
    const working = new Set((quests ?? []).filter(quest => quest));
    return (closed ?? []).filter(one => reopening.has(String(one.id))
        || listed.has(one.address) || working.has(one.quest)).map(one => `session:${one.id}`);
}

// A closed session's note, which never says its name again: its quest's
// title where that differs from its name, else the machine and repository
// it ran in, as a session's drawer names them (VKCF3Z, 2026-11-08).
function closedNote(one) {
    const quest = String(one.presence?.questInfo?.title ?? "");
    if (quest !== "" && quest !== one.title) {
        return quest;
    }
    return [one.machine ?? "", one.presence?.repo || one.folder || ""]
        .filter(part => part !== "").join(" \u00b7 ");
}

// When a closed session ended, at its right: "closed 23:41" today, and
// with its day before, "closed Fri 23:41", as `day` names a date. `said`
// is the vocabulary's agents words, and `endedAt` when it ended, in
// milliseconds.
function closedAt(endedAt, now, said, day) {
    if (typeof endedAt !== "number" || !Number.isFinite(endedAt)) {
        return String(said.closed ?? "");
    }
    const ended = new Date(endedAt);
    const time = [ended.getHours(), ended.getMinutes()].map(part => String(part).padStart(2,
        "0")).join(":");
    const today = ended.toDateString() === now.toDateString();
    return String(said[today ? "closedAt" : "closedOn"] ?? "").replace("{time}", time)
        .replace("{day}", today ? "" : day(ended));
}
