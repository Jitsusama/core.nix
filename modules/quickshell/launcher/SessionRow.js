.pragma library
// A pi session as the launcher shows it, said once, and named apart from
// Session.qml, which is the desktop session: how it stands, the acts it
// offers in their order, and what its drawer says, wherever the launcher
// shows it (Agents, an orchestrator's row, a quest's Go To Its Session).
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

// How a session stands, from its presence, whether its want is truly
// Joel's (`asks`, Sessions.asks) and whether it waits its turn in the
// machine's queue (`queued`): "off" with no presence at all. A want that
// isn't his reads as what the session does meanwhile.
function state(presence, asks, queued) {
    if (!presence) {
        return "off";
    }
    if (asks) {
        return "needs";
    }
    if (presence.state === "parked") {
        return "parked";
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
// its work (VKCF3Z, 2026-11-06: "Close 8IDU4E? Its worktree and 1 Needs
// You item stay."): its worktree, or its folder where it works outside
// one, and its quest's Needs You items, by their count, in the agents'
// words (`said`).
function leaves(inTree, needs, said) {
    const place = inTree ? said.tree : said.folder;
    if (needs === 0) {
        return String(said.stays ?? "").replace("{kept}", place);
    }
    const items = String((needs === 1 ? said.needItem : said.needItems) ?? "")
        .replace("{count}", needs);
    return String(said.stay ?? "").replace("{kept}", `${place} ${said.and} ${items}`);
}

// A change as a landing names it, `repo#number`, never a quest document's
// item, `PLAN-20261106-2ZPAI5#14`, and the only link a landing may open,
// one on the web (the harness's landings.ts).
const CHANGE = /^[\w.-]+#\d+$/;
const DOCUMENT = /^[A-Z]+-\d{8}-[0-9A-Z]{6}(#\d+)?$/;
const WEB = /^https:\/\//;

function isChange(ref) {
    return CHANGE.test(ref) && !DOCUMENT.test(ref);
}

// A landing's key, which Joel's reading it is kept by: its session's, its
// ref and when it landed, so the same change landed again reads anew.
function landingKey(session, one) {
    return `${session.presence?.session ?? session.address} ${one.ref} ${one.at}`;
}

// What sessions landed that Joel hasn't read, for Now's Landed (VKCF3Z,
// 2026-11-06), newest first: each change merged, and anything else that
// says what he should try (`person`). A plan item ticked, which a session
// lands too, is its own progress, 27 of the 77 landings on 2026-11-07,
// so it's left out unless it says what to try. Each is { key, at, ref,
// what, person, url, session }, `url` only a web link.
function landed(sessions, read) {
    const rows = [].concat(...sessions.map(session => (session.presence?.landings ?? [])
        .filter(one => isChange(String(one.ref ?? "")) || String(one.person ?? "") !== "")
        .map(one => ({
            key: landingKey(session, one),
            at: String(one.at ?? ""),
            ref: String(one.ref),
            what: String(one.what || one.ref),
            person: String(one.person ?? ""),
            url: WEB.test(String(one.url ?? "")) ? String(one.url) : "",
            session
        }))));
    return rows.filter(one => !read.includes(one.key)).sort((a, b) => b.at.localeCompare(a.at));
}

// A landing's drawer: who landed it and when as its eyebrow, what landed
// as its title, its ref and its session as facts, and what to try as its
// words, since a sentence never fits beside the row's ref. `said` gives
// when (`at`), the session's title and how a fact is named; what isn't
// known isn't said.
function landingDrawer(one, said) {
    const eyebrow = [one.session.address, said.at]
        .filter(part => (part ?? "") !== "").join(" \u00b7 ");
    const facts = [
        { label: said.named("landed"), value: one.ref },
        { label: said.named("session"), value: said.session }
    ].filter(fact => (fact.value ?? "") !== "");
    return { eyebrow, eyebrowNames: true, title: one.what, facts, body: one.person };
}

// What of Joel's reading is still worth keeping: the keys of landings a
// session still holds, the harness keeping each a day, so what's kept
// never grows past what's there.
function stillRead(sessions, read) {
    const held = [].concat(...sessions.map(session => (session.presence?.landings ?? [])
        .map(one => landingKey(session, one))));
    return read.filter(key => held.includes(key));
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
function hue(stands) {
    return ["paused", "parked", "off"].includes(stands) ? "muted" : "program";
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
function side(doing, spent) {
    return [{ said: doing, keep: 3 }, { said: spent, keep: 4 }]
        .filter(item => item.said !== "");
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

// What its drawer says: its address, where it works and since when it's
// run as the eyebrow over its title, "8IDU4E \u00b7 core.nix \u00b7 since
// 21:52" (VKCF3Z, 2026-11-06); then what it works on, its step, what it
// waits on Joel for, what it's spent today (an orchestrator's whole
// orchestration, then its own and its sessions') and the machine it runs
// on, each as a name and a figure. Nothing it hasn't said is guessed, and
// what the eyebrow says isn't said again. The eyebrow names things, so the
// drawer sets it as they're written, never in capitals (Drawer.qml).
function drawer(title, presence, said) {
    const known = {
        works: presence?.worksOn,
        step: presence?.step,
        wants: presence?.wants?.said,
        spent: said.spent,
        own: said.own,
        under: said.under,
        machine: said.machine
    };
    const facts = Object.keys(known).filter(id => (known[id] ?? "") !== "")
        .map(id => ({ label: said.named(id), value: known[id] }));
    const eyebrow = [said.address, said.place, said.since]
        .filter(part => (part ?? "") !== "").join(" \u00b7 ");
    // Short of room, it gives way by whole items from its end, as the
    // held foot gives its position way whole: when it started goes first,
    // then where it works, and its address never (VKCF3Z, 2026-11-07:
    // "since 0\u2026" tells nothing).
    const eyebrowItems = [
        { said: said.address ?? "", keep: 3 },
        { said: said.place ?? "", keep: 2 },
        { said: said.since ?? "", keep: 1 }
    ];
    return { eyebrow, eyebrowItems, eyebrowNames: true, title, facts, body: "" };
}
