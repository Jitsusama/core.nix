// pi's sessions, a collection: every pi running in a window, the one used
// last first, each named by the quest it has loaded and beside it its
// address, whether it's working, waiting on Joel or its turn in the
// machine's queue (Jobs.qml), what it spent today as spend-watch says
// (SpendWatch.qml), never while the screen is shared, and the folder it
// works in.
// Return goes to its window; the right arrow steps into what's known of it,
// each fact a row Return copies: its quest, its addresses, when it started,
// what it spent today and what of that its councils and fleets cost, the
// heavy work it has running or waiting, what it works on and what it
// hears. What runs, and
// how each is doing, is Sessions.qml's.
//
// Sessions on the realm's other machines are in the same list, drawn the
// same, their machine leading the note (VKCF3Z, 2026-10-29, G8 of
// mycelium's plan), in the one order every list of sessions keeps
// (the agents daemon's standing.rs): those needing Joel, then those working, in line,
// listening and paused, then those parked, this machine's first in each,
// then the others by when each last changed. Go To joins one elsewhere
// through its node; one on a machine out of reach stays, dimmed, saying
// so, and does nothing. One parked has given its memory back until it's
// wanted (Sessions.parked), so Return wakes it in its own window, and
// talking to it wakes it too.
//
// Each session sits under the one orchestrating it, a step in, and two
// under that one's orchestrator, with no line or box: the step alone says
// it, and an orchestration stands where its most urgent row would
// (QEST-20261026-15J7N3's agents-tree-look.md). Only typing ranks, so
// while Joel types every row is at no depth, its orchestrator's address
// leading its note, as a row that arrives while the list is open is
// (Steady.js). An orchestrator says what its whole orchestration spent
// today, as spend-watch sums it by quest, and its drawer what of that was
// its own and its sessions'. Those closed stand at the tree's end,
// dimmed, the one last live first, and Return reopens one where it left
// off (VKCF3Z, 2026-11-07 [9a4b1dd0]), so the tree is the one place an
// orchestrator is listed. After the tree, Budgets steps in to how each
// budget the sessions draw on stands, as AI spend lists them.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import KeyTimes
import "../Fuzzy.js" as Fuzzy
import "../Places.js" as Places
import "../SessionTalks.js" as Said
import "../When.js" as When
import "ActsSaid.js" as ActsSaid
import "CarryBack.js" as CarryBack
import "Cloud.js" as Cloud
import "Landed.js" as Landed
import "Moment.js" as Moment
import "Numbers.js" as Numbers
import "Openings.js" as Openings
import "Order.js" as Order
import "ReopenTally.js" as ReopenTally
import "SessionDrawer.js" as SessionDrawer
import "SessionRow.js" as SessionRow
import "SpendDrawer.js" as SpendDrawer
import "Sprouting.js" as Sprouting
import "StartSaid.js" as StartSaid
import "StartsStopped.js" as StartsStopped
import "TalkAnswers.js" as TalkAnswers
import "TalkTo.js" as TalkTo
import "Wheres.js" as Wheres
import "Wanted.js" as Wanted

Provider {
    id: provider

    what: Words.of("agents").what

    // Named, and found, as the vocabulary says (Words.qml): its tasks,
    // helpers and spend are its parts there.
    collection: Words.collection("agents")

    // Every session's row, in its place in the tree, as the agents daemon
    // makes them ready (modules/agents): how each stands, its words, its
    // depth under its orchestrator and its place in the one order every
    // list of sessions keeps are all the daemon's, so nothing here sorts,
    // words or lays out a session (Joel, 2026-11-10: "the daemons need to
    // be the data layer, and they need to serve the launcher"). Its rows
    // are held by key as the daemon tells each change (rowsKind), each
    // wrapped with its acts and drawer only as it changes, and laid out
    // only while they're wanted (Provider.qml's wanted).
    readonly property var all: opened() && provider.wanted ? (topServed ? top.rows() : laidOut())
        : []

    // The tree's rows as they were before the daemon served the top level:
    // laid out only while wanted and open, so a shut launcher makes none.
    function treeRows() {
        return opened() && provider.wanted ? laidOut() : [];
    }

    // Agents' top level as the agents daemon serves it, ready: one row per
    // orchestration, named for its quest, saying how its sessions stand
    // and what they spent, what needs Joel first and then the newest, with
    // Closed one row at the end; Talk to It first, then Peek, then a step
    // in to its members, each served under it (VKCF3Z, 2026-11-11
    // [3a742b67], after Joel drove the mock: "for agents, definitely
    // improvement"; KP0L6A's place "agents-top"). While the daemon serves
    // none, as before its half is switched in, the tree below stands as
    // it was. Typing still finds every session, leaves included, from the
    // tree's rows, until the daemon serves those ranked too. Shut, it's
    // watched at the same pace as the tree's rows.
    ReadyRows {
        id: top

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/agents.sock`
        place: "agents-top"
        // Its drawers are its sessions', read where the tree's are
        // (topDrawer), not watched again here: each one told here remade
        // its row, so arrows that only moved made 45 to 509 rows (keeps-up,
        // 2026-11-13).
        watching: provider.opened() ? ({}) : ({ every: provider.shutPace })
        // Unpaced as the launcher opens, it's watched again once the
        // opening's frame is drawn, as the tree's rows are.
        paced: provider.launcher
        make: (row, key) => provider.topStood(top.thing(row, key,
            Object.assign(provider.topReopens(row), provider.topDrawer(row),
                provider.topNetwork(row))), row)
        actMade: (key, one, row) => provider.topTalk(key, one, row)
            ?? provider.topGoTo(one)
            ?? provider.topCarryBack(one, row)
            ?? provider.topLetStart(key, one)
        // Its served acts (Pause, Stop, Close) go straight to the daemon,
        // which takes the shell's asks as Joel's, so one a walk chose is
        // withheld and only the walk and the journal told why, as joelsOwn tells the
        // tree's: told on Joel's next opening, the walk's refusal would stand over
        // whatever he opened for.
        stamp: (key, ask) => provider.joelsAsk(ask)
        whyWithheld: () => provider.actWalked()
        withheldToWalk: true
        maker: provider
        drawn: provider.launcher?.open ?? false
    }
    readonly property bool topServed: top.keys.length > 0

    // The realm's node, asked only Carry Back (CarryBack.js), on a
    // connection of the shell's own, since the node carries a quest only
    // for the person's own login and tells that by who asks. Nothing of it
    // is watched.
    Daemon {
        id: node

        socket: Quickshell.env("XDG_RUNTIME_DIR") && Sessions.realm !== ""
            ? `${Quickshell.env("XDG_RUNTIME_DIR")}/mycelium/${Sessions.realm}.sock` : ""
        kind: "carry"
        live: false
        connects: socket !== ""
    }

    // Yet its sessions' rows are wrapped as the shell starts, in Agents'
    // turn of it (Launcher.qml's warmNext), and kept by key (wrapped), so
    // the first letter typed at home finds them wrapped and a change since
    // wraps only the row it changed. Wrapped by that letter instead, 85
    // rows held it on Joel's laptop (his journal, 2026-11-11 23:10;
    // RCTRF8's d181771a).
    onWarmingChanged: {
        if (!provider.warming)
            return;
        laidOut();
        landedWarming.from = 0;
        landedWarming.start();
    }

    // Landed's rows are kept as the shell starts too, a few milliseconds a
    // frame while the launcher's shut, so the first step through Landed
    // finds them kept, as every later one does. Made by that step instead,
    // each of Joel's first To Try Returns after a switch made all 2,600 of
    // them, 350 to 376 ms of its work against 34 to 50 for each later one
    // (his journal, 2026-11-15). The launcher open, it waits: what it hasn't
    // kept by then is made as it's wanted, as before.
    Timer {
        id: landedWarming

        property int from: 0
        readonly property real budget: 8

        interval: 16
        repeat: true
        onTriggered: {
            if (provider.launcher?.open)
                return;
            const began = KeyTimes.now();
            const all = provider.landings;
            const kept = provider.landedKept(String(Calendars.today ?? ""));
            let at = from;
            while (at < all.length && KeyTimes.now() - began < budget) {
                kept(all[at]);
                at += 1;
            }
            from = at;
            if (at < all.length)
                return;
            stop();
            Report.say(`landings kept as the shell started: ${all.length}`);
        }
    }

    // Talk to It on a served row, asking for Joel's words as the tree's
    // Talk does, then sending the daemon's ask with them; null for any
    // other act, which is served whole.
    function topTalk(key, one, row) {
        if (one.key !== "verb:agent:talk" || !one.ask)
            return null;
        // One asking Joel something he can answer here gets the tree's own Talk, which shows
        // what it asks and answers it, as its inbox goes unread until its ask ends.
        const asking = provider.sessionAt(one.ask.address);
        const act = TalkAnswers.topTalkAct(provider.isVoid(one.ask.address),
            !!asking && provider.answerable(asking));
        if (act === "line")
            return provider.lineOpened(one.key);
        if (act === "answer")
            return provider.sessionAct(asking, "talk");
        const talk = provider.talking(one.ask.address, row.name ?? "");
        return thing({
            key: one.key,
            verb: Words.verb(one.key),
            name: Words.name(one.key),
            glyph: Words.glyph(one.key),
            hue: "action",
            ask: Words.of(one.key).ask ?? "",
            chip: row.name ?? "",
            state: () => provider.told,
            talks: talk.talks,
            heard: talk.heard,
            given: words => {
                const text = words.trim();
                if (text === "" || !provider.joelsOwn("talk"))
                    return;
                top.asked(key, Object.assign({}, one, {
                    ask: Object.assign({}, one.ask, { text })
                }));
                talk.said(text);
                provider.told = (Words.of("orchestrators").said ?? {}).sent
                    ?.replace("{name}", row.name ?? "") ?? "";
            }
        });
    }

    // Starts Stopped's Return steps in among its acts, as the right arrow
    // does, where its drawer is no step this launcher takes
    // (StartsStopped.stepsIn), as PastNotices.qml's Return steps in for the
    // same reason. Any other row is as made.
    function topStood(one, row) {
        if (one === null || !StartsStopped.stepsIn(row, provider.launcher))
            return one;
        return Object.assign(one, { verb: "more", now: false, later: false, stays: true,
            targets: one.verbs, wordsId: undefined, returns: undefined,
            run: () => provider.launcher?.enter(one) });
    }

    // Let Them Start lifts Joel's stop on agents' starts, so only his own
    // Return in the launcher asks it (StartsStopped.mayLift). Null for any
    // other act.
    function topLetStart(key, one) {
        if (!StartsStopped.lifts(one))
            return null;
        return thing({
            key: one.key,
            verb: Words.verb(one.key),
            name: Words.name(one.key),
            glyph: Words.glyph(one.key),
            hue: "action",
            stays: true,
            run: () => provider.letThemStart(key, one)
        });
    }

    function letThemStart(key, one) {
        const launcher = provider.launcher ?? null;
        if (!StartsStopped.mayLift(launcher)) {
            const outside = (Words.of("agents:starts-stopped").said ?? {}).outside ?? "";
            launcher?.walkRefused(outside);
            // Only the journal says it, as a refused sprout's is: a notice
            // would let any program put one on his screen.
            console.info(`launcher: agents' starts weren't let start: ${outside}`);
            return;
        }
        top.asked(key, one);
    }

    // Carry Back on a carry under Carried is the shell's, asked of the
    // realm's node, never the daemon (CarryBack.js): its row reads as
    // carrying back until the node says the carry has ended, and a refusal
    // is said in place in the node's own words. Null for any other act.
    function topCarryBack(one, row) {
        const ask = one.key === "verb:carry:back" ? CarryBack.asked(one.ask) : null;
        if (ask === null)
            return null;
        return thing({
            key: one.key,
            verb: Words.verb(one.key),
            name: Words.name(one.key),
            glyph: Words.glyph(one.key),
            hue: "action",
            stays: true,
            run: () => provider.carryBack(row, ask)
        });
    }

    // Joel's alone to choose: the node weighs a carry's starts by who asks, and the shell
    // asks as him, and the top rows' stamp never sees it, since it's made here (round
    // 000036 on #6796, F137).
    function carryBack(row, ask) {
        if (!provider.joelsOwn("carry back"))
            return;
        const said = Words.of("verb:carry:back").said ?? {};
        const shown = row.thing ?? row.key;
        let answered = false;
        provider.becomes(shown, {
            side: said.carrying ?? "",
            sideItems: [{ said: said.carrying ?? "", keep: 4 }]
        }, row.name ?? "", "carry back", { done: () => answered });
        node.find(ask, answer => {
            const ended = CarryBack.ended(answer, said);
            if (ended.done)
                answered = true;
            else
                provider.refused(shown, ended.why);
        });
    }

    // Go To on a served orchestration is the shell's, as a session's own is
    // (sessionAct), since only the shell moves niri's focus (acts.rs): asked
    // of the daemon, it answered nothing and said it couldn't
    // (moves-keep-their-window, 2026-11-13). Close is the daemon's, served
    // saying what stays (ReadyRows.js's asking). Null for any other act, or
    // a session not here.
    function topGoTo(one) {
        const session = one.key === "verb:agent:switch" ? sessionAt(one.ask?.address) : null;
        return session ? sessionAct(session, "switch") : null;
    }

    // A served row naming its session's drawer, "drawer:agents-top:<address>",
    // is drawn as the tree's session rows draw it (drawerOf, SessionDrawer.js),
    // from the drawer the tree's watch holds: the daemon serves a session's
    // eyebrow as items and its facts as labels and values, not as
    // ready-rows.md's drawer, so read as one it said "[object" over blank
    // facts (finds-sessions, 2026-11-13). Its acts are named as served,
    // Return's by ↵, except where they step into its members. What y takes
    // is its quest, else its address, as the tree's session rows give it
    // (sessionRow's operands): read for the foot from its acts instead,
    // each row an arrow first chose made its four act rows, 89 in a frozen
    // turn of 90 keys (keeps-up, 2026-11-13). A network's drawer,
    // "drawer:agents-top:network:<name>", is no session's: it's served in
    // ready-rows.md's own shape and drawn as any ready-rows drawer is, as
    // Starts Stopped's is, "drawer:agents-top:starts-stopped".
    function topDrawer(row) {
        const address = /^drawer:agents(?:-top)?:(?!network:|starts-stopped$)(.+)$/u
            .exec(String(row?.drawer ?? ""))?.[1];
        if (address === undefined)
            return {};
        return {
            drawer: () => {
                const acts = top.actsOf(row);
                const steps = acts.some(one => one.step === "rows");
                return top.actsNamed(provider.drawerOf(address), steps ? [] : acts, null);
            },
            operands: () => {
                const owns = provider.sessionAt(address)?.presence?.owns ?? [];
                const quest = owns.find(owned => /^QEST-/u.test(owned)) ?? "";
                return [quest !== "" ? provider.copyOperand("fact:agent:quest", quest)
                    : provider.copyOperand("fact:agent:address", address)];
            }
        };
    }

    // A network's row in Networks, whose y copies its short name, the one
    // its cells are told by (VKCF3Z, 2026-11-14 [e1eb0ebe]): its acts step
    // into its cells and peek, so y is read from what it holds instead.
    function topNetwork(row) {
        const short = row?.kind === "network" ? String(row?.held?.short ?? "") : "";
        if (short === "")
            return {};
        return { operands: () => [provider.copyOperand("fact:network:short", short)] };
    }

    // A closed orchestrator the daemon serves under Closed, with no acts
    // of its own: reopened as the tree's are (closedRow), from the record
    // it holds, until the daemon reopens through the node (KP0L6A,
    // 2026-11-11 [155d2b08]).
    function topReopens(row) {
        const one = row?.held?.closed;
        if (row?.kind !== "closed-orchestrator" || !one)
            return {};
        const orchestrators = source("orchestrators");
        const acts = () => provider.closedActs(one, orchestrators);
        return {
            verb: Words.verb("verb:orchestrator:reopen"),
            run: () => provider.reopenNow(one, orchestrators),
            verbs: acts,
            targets: acts
        };
    }

    function opened() {
        const launcher = provider.launcher;
        return !!launcher && (launcher.open || launcher.cataloguing);
    }

    // Whether its own rows after its sessions' are read now: at home, which
    // shows them, walked from outside, or held in Agents, or in one of the
    // `others` places that read them. A stopgap until the agents daemon
    // makes them ready (Fri 13 Nov), so a chord into another place makes
    // none.
    function readable(others) {
        const launcher = provider.launcher;
        if (!launcher || typeof launcher.wants !== "function")
            return opened();
        return Wanted.readAt({ cataloguing: launcher.cataloguing, open: launcher.open,
            held: launcher.held, becoming: Object.values(launcher.becoming ?? {}),
            typing: launcher.typing, entering: launcher.entering ?? null,
            viewing: launcher.viewing ?? false },
            [provider].concat(others));
    }
    // Held as a bool, so a step made again while it stays readable remakes
    // none of them: each change to Agents remakes the steps held
    // (Launcher.qml's relive), and read as the function, every new step
    // made Landed, Recent and Start again (LR3WMN, 2026-11-12).
    readonly property bool readNow: readable([])
    // Sessions to Reopen is read in Now too, whose For You draws it; the
    // rest aren't, since Now shows none of them. Now is the source named
    // "on" (home.nix): asked for as "now", it was never found, so held in
    // Now none were read, and each step out to home made all of them
    // again, 47 to 72 ms of Joel's (2026-11-21, 13:13).
    readonly property bool reopenRead: readable([source("on")])
    // Asleep in a kept opening's waking until its turn, like any source
    // home's rows read (Launcher.qml's wakingOrder), so its own are read
    // as it wakes, after the open's turn: in a slice of its own when Now's
    // takes up the slice's budget, in Now's when Now's is short.
    readsHome: true

    // Agents' own rows after its sessions' (toReopen, landed, recent, start
    // and budgets), as each was last made while anything could read it
    // (readable): at home, in Agents, or its catalogue walked, and Sessions
    // to Reopen in Now too (reopenRead). Shut,
    // or opened into another place, nothing does, so
    // each gives what it last made and reads nothing else, and a change
    // nobody sees makes none of them. Joel's shut launcher made 312
    // of Agents' rows in 240 s (YQZGPC, 2026-11-10). Read again, each is
    // kept while everything it's made from is as it was (keptAlike), so
    // an opening or a step out of Now makes one again only if that changed
    // while nothing read it: each - out of Now made 4 to 6 of them anew as
    // home was held, 47 to 72 ms of Joel's (2026-11-21, 13:13). Budgets
    // is kept by AI spend's row it's made of, and Spec by what
    // cadence-spec's head says, since each is another place's. For
    // You's (forYou)
    // are made shut too, since the tray counts what waits there, but only
    // as a landing comes, goes or changes. A keeper (Provider.qml's), so
    // keeping one notifies nothing.
    readonly property var ownMade: keeper()
    function madeBefore(name) {
        return ownMade.rows[name]?.row ?? null;
    }
    // Made here and kept by its name while `parts` are as they were.
    function ownKept(name, parts, make) {
        return keptAlike(ownMade, name, parts, make);
    }

    // The daemon's rows of Agents, each record held by its key, its fields
    // under `row` (Y2YBTS's ready-rows.md), watched whether or not the
    // launcher's open, so it opens on rows already held. Shut, the watch
    // is paced (`shutPace`): the daemon holds back what moves and tells
    // it all in one line a pace, since a spend side ticking or a session
    // turning to listen moved a row about every three seconds, each line
    // waking the shut shell for nothing it shows (RO9YDD, 21 lines in 60 s
    // on 2026-11-11). Paced by the launcher, its Daemon asks again once the
    // opening's next frame is drawn,
    // the same watch is told at once what it held back, never every row
    // whole, so an opening draws from rows at most a pace old and is
    // brought up to now a frame later, its fill not waiting on the line.
    readonly property var rowsKind: rowsLink.item
    readonly property int shutPace: 10000
    LazyLoader {
        id: rowsLink
        active: !!Quickshell.env("XDG_RUNTIME_DIR")

        Daemon {
            id: agentsRows

            socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/agents.sock`
            kind: ["row", "summary", "recent", "landed", "landed-days", "try", "closed",
                "want", "talks", "start"]
            watching: provider.opened() ? ({ place: "agents" })
                : ({ place: "agents", every: provider.shutPace })
            paced: provider.launcher
            ahead: provider.drawersWatched.concat(provider.landingDrawersAhead)
            answersWhole: true
            onHeard: (changed, gone) => provider.placed(changed, gone)
            // Told as it answers, and again as sharing changes, whether the
            // screen is shared, which only the shell knows from niri's
            // casts, so the daemon says no amount while it is.
            onStateChanged: {
                provider.tellSharing();
                // The starts kept are looked at again with the one saying
                // hello now, so a told it never answered is said again: the
                // records it serves come alike to those held through the
                // gap, so tell no change that would.
                if (state === "answering")
                    provider.sproutsKept(held);
            }
        }
    }
    // The day Joel is in under Landed, watched alone, each landing it
    // tells moving that day's rows (landingsOn); no watch at all while
    // he's in none, so what it held goes with it.
    readonly property var landedDayKind: landedDayLink.item
    LazyLoader {
        id: landedDayLink
        active: !!Quickshell.env("XDG_RUNTIME_DIR") && provider.landedDayIn !== ""

        Daemon {
            socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/agents.sock`
            kind: ["landed"]
            watching: ({ place: provider.landedDayIn })
            answersWhole: true
            onHeard: () => provider.landedDayMoves += 1
        }
    }
    readonly property string summaryKey: "summary:agents"

    function tellSharing() {
        if (rowsKind?.state === "answering")
            rowsKind.askTillTaken(() => ({ ask: "sharing", sharing: Quiet.sharing }));
    }
    Connections {
        target: Quiet

        function onSharingChanged() {
            provider.tellSharing();
        }
    }

    // The keys of its rows in the order the daemon gives them, each put in
    // its place by its `order` as it changes, never the list sorted; and
    // each row as last wrapped, by key. Plain, so keeping them notifies
    // nothing; `placings` says they moved.
    readonly property var placing: ({ keys: [] })
    readonly property var wrapped: ({})
    // What each row was wrapped from, all but its order, so a row the
    // daemon tells again only for its place, as a session turning moves
    // every row after it, keeps its wrap: 96 of the 124 rows told in 90 s
    // on Joel's laptop moved only their order (2026-11-12).
    readonly property var wrappedFrom: ({})
    function wrapSign(served) {
        return served ? JSON.stringify(Object.assign({}, served, { order: 0 })) : "";
    }
    property int placings: 0
    // Says Recent's records moved, so only Recent is made again.
    property int recentMoves: 0
    // Says the closed orchestrators' records moved.
    property int closedMoves: 0
    // Says a session's talks moved, as the field showing them reads them.
    property int talksMoves: 0

    // Takes what the daemon said moved: each row's key gone or changed let
    // go, and each changed put back where its order places it. A drawer or
    // the summary moves no row, so places nothing. Its lists come through
    // the signal converted, so they're read by index.
    function placed(changed, gone) {
        const held = rowsKind?.held ?? {};
        const placedKeys = placing.keys;
        const rowKey = key => String(key).startsWith("row:");
        const moved = [];
        for (let at = 0; at < changed.length; at += 1)
            moved.push(changed[at]);
        for (let at = 0; at < gone.length; at += 1)
            moved.push(gone[at]);
        if (moved.some(key => String(key).startsWith("recent:")))
            recentMoves += 1;
        // A landing's drawer told apart moves no landing, so it makes none
        // of Landed's rows again: its drawer, drawn, reads it as it comes.
        if (moved.some(key => /^landed(?:-days)?:/u.test(String(key))))
            landedMoves += 1;
        if (moved.some(key => String(key).startsWith("try:")))
            triesMoves += 1;
        if (moved.some(key => String(key).startsWith("closed:")))
            closedMoves += 1;
        if (moved.some(key => String(key).startsWith("talks:")))
            talksMoves += 1;
        if (changed.some(key => String(key).startsWith("start:")))
            sproutsKept(held);
        if (!moved.some(rowKey))
            return;
        for (const key of moved) {
            if (wrappedFrom[key] !== wrapSign(held[key]?.row)) {
                delete wrapped[key];
                delete wrappedFrom[key];
            }
            const at = placedKeys.indexOf(key);
            if (at >= 0)
                placedKeys.splice(at, 1);
        }
        for (let at = 0; at < changed.length; at += 1) {
            const key = changed[at];
            const order = held[key]?.row?.order;
            if (typeof order !== "number")
                continue;
            let low = 0;
            let high = placedKeys.length;
            while (low < high) {
                const middle = (low + high) >> 1;
                if ((held[placedKeys[middle]]?.row?.order ?? 0) < order)
                    low = middle + 1;
                else
                    high = middle;
            }
            placedKeys.splice(low, 0, key);
        }
        placings += 1;
        wrapAhead();
    }

    // While Agents' rows are shown, what moved is wrapped the frame after
    // it's drawn rather than by the next key. Unseen, as at home, which
    // reads only Agents' count, nothing is, since a row made for nobody is
    // work a held key waits on (vm-desktop-launcher-keeps-up, ZINAWZ,
    // 2026-11-12). The first letter at home makes only the rows whose words
    // changed meanwhile, as a row moved only in order keeps its wrap: it
    // once made 176 to 185 of them, 11 to 17 ms of letters that took 56 to
    // 81 (RCTRF8, 2026-11-12).
    property bool wrapAsked: false
    function wrapAhead() {
        const launcher = provider.launcher;
        if (wrapAsked || !provider.wanted || !launcher?.open
            || typeof launcher.afterFrame !== "function")
            return;
        wrapAsked = true;
        // Felt as it runs, a frame on, where an opening's stall can hold it.
        launcher.afterFrame(() => provider.felt("wrapping", () => {
            provider.wrapAsked = false;
            if (provider.opened() && provider.wanted)
                provider.laidOut();
        }));
    }

    // As the launcher opens at home, what typing will find is wrapped the
    // frame after home is drawn, so a session that came or changed while
    // it was shut is made then, not by the first letter: its row, its
    // Talk and its top level's row were that letter's five (KP0L6A,
    // 2026-11-13). The rows that kept their words keep their wraps, so
    // this makes only those. Opened into a place, typing searches that
    // place alone, so nothing is.
    function wrapForTyping() {
        const launcher = provider.launcher;
        const home = () => !!launcher?.open && (launcher.held?.length ?? 0) === 0
            && !launcher.entering;
        if (!home() || typeof launcher.afterFrame !== "function")
            return;
        launcher.afterFrame(() => provider.felt("wrapping", () => {
            if (!home())
                return;
            provider.laidOut();
            if (provider.topServed)
                top.rows();
        }));
    }

    // Every row at the tree's top, in the daemon's order, a fold holding
    // its members, then the closed orchestrators.
    function laidOut() {
        void placings;
        const held = rowsKind?.held ?? {};
        const rows = placing.keys.filter(key => !held[key]?.row?.held?.fold)
            .map(key => rowOf(key, held)).filter(row => row !== null);
        const orchestrators = source("orchestrators");
        if (!orchestrators)
            return rows;
        return rows.concat(shutOrchestrators.map(one => closedKept(one, orchestrators)));
    }

    // A closed orchestrator's row, kept while it and its folder say the
    // same, as its sessions' are; kept by a key of its own, since the
    // daemon's record of it is told as moved whenever any closed one is.
    function closedKept(one, orchestrators) {
        const key = `shut:${one.address}`;
        const gone = orchestrators.goneFrom(one);
        const sign = JSON.stringify([one, gone?.side ?? null, !!gone?.afresh]);
        if (wrappedFrom[key] !== sign) {
            wrapped[key] = closedRow(one, orchestrators);
            wrappedFrom[key] = sign;
        }
        return wrapped[key];
    }

    // The closed orchestrators, as the daemon serves them (closed.rs): each
    // the harness lists that no running or parked session answers to, the
    // one last live first, with what reopening it needs.
    readonly property var shutOrchestrators: {
        void closedMoves;
        const held = rowsKind?.held ?? {};
        return Order.sorted(Object.keys(held).filter(key => key.startsWith("closed:"))
            .map(key => held[key]), Order.smaller(one => one.order ?? 0))
            .map(one => one.closed);
    }

    // The session a row is of, by its address, running or parked.
    function sessionAt(address) {
        return Sessions.running.find(one => one.address === address)
            ?? Sessions.parked.find(one => one.address === address) ?? null;
    }

    // A row of the daemon's, wrapped with what doing it needs: a fold's
    // Return steps in to its members, each wrapped as it changed; a
    // session's acts and drawer are read from its session as they're
    // shown. None for a session Sessions doesn't hold yet.
    function rowOf(key, held) {
        const served = held[key]?.row;
        if (!served)
            return null;
        if (served.kind === "fold") {
            const members = (served.held?.members ?? []).map(one => rowOf(one, held))
                .filter(row => row !== null);
            // Kept as its members are, and handed them afresh, which makes
            // no row.
            if (key in wrapped) {
                wrapped[key].held = members;
                return wrapped[key];
            }
            const row = thingRow({
                key: served.row,
                name: served.name,
                glyph: Theme.glyph[served.glyph] ?? Theme.glyph.agent,
                hue: served.hue,
                copiesNothing: true,
                depth: served.depth
            }, () => row.held);
            row.held = members;
            wrapped[key] = row;
            wrappedFrom[key] = wrapSign(served);
            return row;
        }
        if (!(key in wrapped)) {
            const session = sessionAt(String(served.row).replace(/^agent:/, ""));
            if (session === null)
                return null;
            wrapped[key] = sessionRow(session, served);
            wrappedFrom[key] = wrapSign(served);
        }
        return wrapped[key];
    }

    // The names its row says after its own, its rows' names in their
    // order, read from the daemon's rows, never from what's made of them,
    // so Home names them without laying any out.
    function memberNames() {
        if (topServed)
            return top.keys.filter(key => !top.served(key)?.parent)
                .map(key => top.served(key).name)
                .concat([toReopen, landed, recent].filter(row => row !== null)
                    .map(row => row.name));
        void placings;
        const held = rowsKind?.held ?? {};
        return placing.keys.filter(key => !held[key]?.row?.held?.fold)
            .map(key => held[key].row.name)
            .concat(shutOrchestrators.map(one => one.name),
                [toReopen, landed, recent].filter(row => row !== null).map(row => row.name));
    }

    // A closed orchestrator's row: its glyph muted, as one paused is, its
    // name in text (the harness's short name first), "closed" at its right
    // with what its whole orchestration spent when the daemon says it, which
    // it doesn't while the screen is shared, its quest's title as its note,
    // and Return reopening it where it left off. One whose folder is gone
    // says so in alert, and starts afresh in its quest's start folder
    // (Orchestrators.qml's goneFrom), or, with none, steps in to its talks.
    function closedRow(one, orchestrators) {
        const items = SessionRow.side(word("closed"),
            one.spent ? word("allOf", { spent: one.spent }) : "");
        const gone = orchestrators.goneFrom(one);
        const row = member({
            thing: true,
            key: `orchestrator:${one.address}`,
            kind: SessionRow.KIND,
            name: one.name,
            also: Fuzzy.prepare(`${one.address} ${one.quest ?? ""} ${one.title ?? ""}`),
            glyph: Theme.glyph.agent,
            hue: SessionRow.hue("off"),
            side: items.map(item => item.said).join(" \u00b7 "),
            sideItems: items,
            note: one.title ?? "",
            depth: 0,
            verb: Words.verb("verb:orchestrator:reopen"),
            run: () => provider.reopenNow(one, orchestrators),
            verbs: () => provider.closedActs(one, orchestrators)
        });
        if (gone !== null)
            Object.assign(row, {
                side: gone.side,
                sideItems: [{ said: gone.side, keep: 3 }],
                alerts: true,
                hue: "alert",
                verb: gone.afresh ? Words.verb("verb:orchestrator:afresh") : "more",
                run: gone.afresh ? gone.afresh : () => provider.launcher?.enter(row)
            });
        return row;
    }

    // A closed orchestrator's acts: Reopen, or Start Afresh where its
    // folder is gone, then its talks, newest first.
    function closedActs(one, orchestrators) {
        const act = (id, run) => thing({
            key: `verb:orchestrator:${id}`,
            verb: Words.verb(`verb:orchestrator:${id}`),
            name: Words.name(`verb:orchestrator:${id}`),
            glyph: Words.glyph(`verb:orchestrator:${id}`),
            hue: "action",
            run: run
        });
        const gone = orchestrators.goneFrom(one);
        const reopen = () => provider.reopenNow(one, orchestrators);
        const first = gone === null ? [act("reopen", reopen)]
            : gone.afresh ? [act("afresh", gone.afresh)] : [];
        return first.concat(orchestrators.talksOf(one.address));
    }
    // Said as every row it shows moves, the sessions' and those after
    // them, so a step held in Landed loses a landing as it's read; and
    // only while they're wanted, so a change nobody sees refreshes
    // nothing. Wanted again, it says whatever moved meanwhile. The parts
    // it makes itself, Budgets and Spec, are said too, though listed as
    // parts, so a new head or axes from their daemons shows where it's held.
    readonly property var everyRow: provider.wanted
        ? everything().concat([budgets, spec].filter(row => row !== null)) : null
    onEveryRowChanged: {
        if (everyRow !== null)
            changedTo(everyRow);
    }

    // Every session's row, those folded too, as typing searches them.
    function sessionRows(rows) {
        return rows.reduce((every, row) => every.concat(row.held ? sessionRows(row.held)
            : [row]), []);
    }

    // How a session stands as the agents daemon's row says it, in the one
    // rule and order every list keeps (modules/agents/src/standing.rs), so
    // nothing here works it out a second way; "off" before it's said.
    function stateOf(session) {
        void placings;
        const served = rowsKind?.held?.[`row:agents:${session.address}`]?.row;
        return String(served?.held?.state ?? "off");
    }

    // Where a session stands in that order, as the daemon's row says it,
    // so Ask's pick keeps the order Agents lists in; after them all before
    // it's said (SessionRow.rankOf).
    function rankOf(session) {
        void placings;
        return SessionRow.rankOf(rowsKind?.held?.[`row:agents:${session.address}`]?.row);
    }

    // One of the vocabulary's words for a session, filled in.
    function word(key, fill) {
        let said = String((Words.of("agents").said ?? {})[key] ?? "");
        for (const name of Object.keys(fill ?? {}))
            said = said.replace(`{${name}}`, fill[name]);
        return said;
    }

    // What's said of a session elsewhere, in the vocabulary's words.
    function elsewhere(key, session, why) {
        return String((Words.of("agents:machine").said ?? {})[key] ?? "")
            .replace("{machine}", session.machine).replace("{why}", why ?? "");
    }

    // Says why a Go To, a reopen or a start Joel pressed opened nothing, by
    // the session's name and `why` in the alert colour: in the foot at once
    // while the launcher's open, else by opening it on Agents to say so
    // (Launcher.qml's toldHere), since every act shuts the launcher before
    // it runs, and a why held for its next opening was told to nobody
    // (KP0L6A, 2026-11-24 [00902e94]): a press that did nothing, unsaid.
    // Only a Go To asked for this session moments ago opens it; any other
    // waits in the foot, and one a wake reopens is unsaid
    // (SessionRow.refusalTold). Not Provider's refused, which takes back a
    // row by its key.
    function refusedGoing(session, why) {
        const told = provider.refusalTold(session);
        if (told === "unsaid")
            return;
        const name = provider.footName(session);
        const said = provider.elsewhere(why, session);
        provider.refusedWhy = `${name}: ${said}`;
        provider.launcher?.tellOnOpen([[name, said, Theme.alert]]);
        provider.openedForPress(told);
    }
    // How a refusal of `session` is told (SessionRow.refusalTold).
    function refusalTold(session) {
        return SessionRow.refusalTold(session.address, { waking: Sessions.waking,
            goneTo: Sessions.goneTo, opened: provider.refusalOpened }, Date.now());
    }
    // Opens the launcher on what the foot was just told, where `told` says a
    // Go To Joel pressed moments ago asked for it, once for that press.
    function openedForPress(told) {
        if (told === "opens" && provider.launcher?.toldHere?.())
            provider.refusalOpened = Sessions.goneTo.at;
    }
    // The Go To press a refusal last opened the launcher for, by when it
    // was pressed, so that press opens it once (SessionRow.refusalTold).
    property real refusalOpened: 0
    // The last refusal said, as refusedGoing says it, so a Go To asked from
    // outside answers with the why it was given (goToFor), never one read
    // back from the launcher's foot, which its opening has taken.
    property string refusedWhy: ""

    // What the foot names a session by (SessionRow.footNameOf): its row's
    // name, else its quest's title, its address only when neither is known.
    function footName(session) {
        return SessionRow.footNameOf(provider.standingOf(session.address)?.name, session,
            presence => Sessions.named(presence).title);
    }

    // A session elsewhere that couldn't be joined, a hosted one with no
    // window to go to, or any the node couldn't tell, is said in the foot,
    // by its name (footName) in the alert colour, but for a viewer asked for
    // a moment ago, which nothing refused, where the act was pressed: at
    // once while the launcher's open, or else once it opens next, never as
    // a notice (VKCF3Z, 2026-11-07 [4a00987e]).
    Connections {
        target: Sessions

        // New work Joel started opens as soon as its session is up.
        function onRunningChanged() {
            provider.openSprouted();
        }

        function onUnjoined(session) {
            provider.refusedGoing(session, session.reachable ? "unheld" : "unreached");
        }

        // Said by why it held, in words of its own (Sessions' unhosted),
        // plainly where it tells of a viewer opening or viewed, the alert
        // kept for a press that didn't view (SessionRow.viewedWhy). A
        // viewer opening or viewed, or opened beside a stray window holding
        // its name ("unowned"), went, so it's told without opening the
        // launcher over the window Joel went to.
        function onUnhosted(session, why) {
            const viewed = SessionRow.viewedWhy(why);
            if (viewed || why === "unowned")
                provider.launcher?.tellOnOpen([[provider.footName(session),
                    provider.elsewhere(why, session),
                    viewed ? Theme.text : Theme.alert]]);
            else
                provider.refusedGoing(session, why);
        }

        function onUnopened(session) {
            provider.refusedGoing(session, "unopened");
        }

        // A Wake refused or never answered whose row was becoming woken is
        // taken back at once, said as not done, rather than reading done for
        // as long as Becoming waits (a refused Wake read listening for
        // 30 s). Only a Wake: another steer refused meanwhile, a talk say, is
        // said as itself and leaves the Wake's row be (SessionRow.takesBack,
        // SessionRow.unsteered).
        function onUnsteered(session, why, act) {
            const called = provider.footName(session);
            const told = SessionRow.unsteered(provider.launcher,
                Words.of("agents:machine").said ?? {}, {
                act, actName: SessionRow.actName(act, Words.name), why,
                address: session.address, name: called === session.address ? "" : called,
                called, held: Boolean(provider.launcher?.becoming?.[`agent:${session.address}`])
            }, Theme.alert);
            // A Wake a Go To asked for is refused only once goTo has gone
            // back, so Sessions' waking no longer holds it: the press that
            // woke it is answered as any Go To's refusal is, rather than
            // reading done with the launcher shut (#6519's third review).
            // Its words are kept as refusedWhy too, so a link's Go To whose
            // wake was refused at once answers why, not "" (#6519's F147).
            if (act === "wake") {
                if (told !== null)
                    provider.refusedWhy = `${told[0]}: ${told[1]}`;
                provider.openedForPress(provider.refusalTold(session));
            }
        }

        // A Go To's wake refused while its press decided, held then, is
        // answered once that press didn't go (Sessions' wakeRefusedAtOnce).
        function onWakeRefusedAtOnce(session) {
            provider.openedForPress(provider.refusalTold(session));
        }

        // A parked session Wake would reopen with no fence it can say,
        // which might bring it back looser than it ran, is left asleep and
        // said so, rather than as the node's refusal, which blames its
        // missing holder (#5901's F16). A Wake pressed here, whose row was
        // becoming woken, is taken back at once, as a refused Wake is, so
        // the row doesn't read listening until Becoming gives up on it.
        function onUnfenced(session) {
            const key = `agent:${session.address}`;
            if (provider.launcher?.becoming?.[key]) {
                provider.refused(key, provider.elsewhere("unfencedWhy", session));
                return;
            }
            provider.launcher?.tellOnOpen([[provider.footName(session),
                provider.elsewhere("unfenced", session), Theme.alert]]);
            provider.openedForPress(provider.refusalTold(session));
        }
    }

    // What a session spent today as spend-watch says, and, for one that
    // `orchestrates`, its whole orchestration's as spend-watch sums its
    // quest and those under it, or nothing to say either by while sharing.
    function spendOf(presence, quest, place) {
        const spent = presence && presence.session && !Quiet.sharing
            ? SpendWatch.sessions[presence.session] ?? "" : "";
        const family = place?.orchestrates && quest !== "" && !Quiet.sharing
            ? SpendWatch.families[quest] ?? null : null;
        return { spent, family };
    }

    // A session's row as the daemon made it ready, its fields as given,
    // with what its acts, drawer and copies do. Typing ranks it flat, as
    // the daemon made it read at no depth too.
    function sessionRow(session, served) {
        const held = served.held ?? {};
        const presence = session.presence;
        const quest = presence && presence.owns
            ? presence.owns.find(owned => /^QEST-/.test(owned)) ?? "" : "";
        const place = { depth: served.depth, orchestrates: !!held.orchestrates };
        const said = items => (items ?? []).map(item => item.said).join(" \u00b7 ");
        const returning = String(held.returns ?? "").replace(/^verb:agent:/, "");
        const row = member({
            thing: true,
            key: served.row,
            kind: SessionRow.KIND,
            // The session is its window, which the windows and its quest
            // also find, and it says the most of the three, so it's the
            // one row left of them (Launcher.qml's compose).
            is: Sessions.isOf(session),
            rich: 2,
            name: served.name,
            // Its address, folder and quest's id, as the daemon words them,
            // made ready to match, since a member keeps its words as given.
            also: Fuzzy.prepare(String(held.also ?? "")),
            glyph: Theme.glyph[served.glyph] ?? Theme.glyph.agent,
            // The mark and hue it wears not waiting on Joel, which For You
            // keeps; none served, For You keeps the row's own.
            mark: Theme.glyph[held.mark],
            markHue: held.markHue,
            hue: served.hue,
            // Waiting on Joel, parked with his want open too, so a wake
            // keeps his mark's hue (wakeNow).
            waitsOnJoel: held.waitsOnJoel === true,
            side: said(served.side),
            sideItems: served.side ?? [],
            note: String(held.note ?? ""),
            depth: served.depth,
            gone: !!held.gone,
            // Return does the act its state makes harmless, one of those
            // its step in holds.
            returns: returning !== "" ? provider.sessionAct(session, returning) : null,
            verbs: () => {
                const { spent, family } = provider.spendOf(presence, quest, place);
                return provider.verbs(session, quest, spent, family);
            },
            // Its parts, its acts and facts, are read as they're made, the
            // queue's jobs and its talks among them, and found only from
            // inside it, so the walk for where rows live never makes them
            // (Catalogue.js's followsInto): walked on each change, it made
            // every session's, about nine rows each, 6,733 in twelve seconds
            // at home over 300 sessions (2026-11-09).
            parted: true,
            // What y takes: the first of its facts, as its verbs list them.
            operands: () => [quest !== "" ? copyOperand("fact:agent:quest", quest)
                : copyOperand("fact:agent:address",
                    (session.presence?.addresses ?? [session.address]).join(", "))],
            // Its drawer reads the session as it stands, since its row is
            // kept while only its step moves.
            drawer: () => provider.actsNamed(provider.drawerOf(session.address), session)
        });
        // Itself as it reads ranked flat: at no depth, whom it's under
        // leading its note and what it says at its right (15J7N3,
        // 2026-11-04). It isn't named flat, which Quickshell's JavaScript
        // would read as the arrays' method it lacks.
        const ranked = held.ranked ?? {};
        row.ranked = () => Object.assign({}, row, {
            depth: 0,
            note: String(ranked.note ?? ""),
            side: said(ranked.side),
            sideItems: ranked.side ?? [],
            belongs: String(ranked.belongs ?? "")
        });
        return row;
    }

    // How a session stands as its row serves it, its name and how the
    // daemon words its state without its figures, as For You's heading over
    // the cell says it (Now.qml's cellHeading), or none before it's told.
    // Read from the record, never a made row, since the headings are heard
    // with the launcher shut, where a row made for nobody is work a held
    // key waits on (vm-desktop-launcher-keeps-up, ZINAWZ).
    function standingOf(address) {
        void placings;
        const served = rowsKind?.held?.[`row:agents:${address}`]?.row;
        return served ? { name: String(served.name ?? ""),
            doing: String(served.held?.doing ?? "") } : null;
    }

    // A session's row at no depth, as Now and Ask show one, or none while
    // the daemon hasn't said it.
    function made(session) {
        void placings;
        const row = rowOf(`row:agents:${session.address}`, rowsKind?.held ?? {});
        return row === null ? null : Object.assign({}, row, { depth: 0 });
    }

    // A session's drawer as the agents daemon made it ready
    // (modules/agents/src/drawer.rs), what matters first: watched by its
    // key only while it's shown or beside the one shown (drawersWatched),
    // since its figures move every turn, so the drawer names its key just
    // after it first asks, and fills as the daemon answers. Only its times are worded here, by the clock
    // (SessionDrawer.js), so one left open never waits on the daemon to
    // turn a minute.
    property string drawerWanted: ""
    // The session drawers watched: the one drawn, then those of the
    // sessions' rows beside it, watched ahead as Landed's are
    // (landingDrawerOf), so an arrow to either finds its drawer held and
    // draws it with its key. Watched only once drawn, each session's came a
    // round trip after its row, 23 to 71 ms after its key (Joel's journal,
    // 18 and 19 November). Set whole, so an arrow tells the daemon one watch.
    property var drawersWatched: []
    function watchDrawers() {
        const drawn = drawerWanted.slice("drawer:agents:".length);
        const watched = drawerWanted === "" ? []
            : [drawerWanted].concat(sessionDrawersBeside(drawn));
        if (watched.join("\n") !== drawersWatched.join("\n"))
            drawersWatched = watched;
    }

    function drawerOf(address) {
        const key = `drawer:agents:${address}`;
        if (key !== drawerWanted)
            Qt.callLater(() => {
                if (provider.drawerWanted === key)
                    return;
                provider.drawerWanted = key;
                provider.watchDrawers();
                staying.stop();
                staying.address = "";
            });
        void rowsKind?.revision;
        const made = rowsKind?.held?.[key]?.drawer ?? null;
        // The stay is counted from when the drawer shows what it holds,
        // which the daemon serves a moment after it's asked for. The cursor
        // may have left the row by the time this runs, as keys come in a
        // burst, and a stay begun then would run on to the row's return.
        if (made !== null && staying.address !== address)
            Qt.callLater(() => {
                if (provider.drawerWanted !== key || staying.address === address
                    || String(provider.launcher?.selected?.key ?? "") !== `agent:${address}`)
                    return;
                staying.address = address;
                staying.restart();
            });
        return made === null ? null : SessionDrawer.shown(made, {
            clock: at => started(at),
            since: word("since")
        });
    }

    // The drawers' keys of the sessions' rows either side of the session at
    // `address` in the list the launcher shows (SessionDrawer.beside), each
    // a session here.
    function sessionDrawersBeside(address) {
        return SessionDrawer.beside(launcher?.matches ?? [], launcher?.current ?? -1, address)
            .filter(beside => sessionAt(beside) !== null)
            .map(beside => `drawer:agents:${beside}`);
    }

    // A session's drawer with its acts by name, Return's by ↵, read from
    // which its state holds rather than made: the drawer's own list of a
    // row's acts makes its verbs, and a session's are its facts and talks
    // besides, about nine rows for each arrow resting on one, 891 in a
    // held arrow's turns (keeps-up, 2026-11-12), each fact named an act.
    function actsNamed(drawer, session) {
        if (drawer === null)
            return null;
        // Return marked on the act it does, which for one paused is Resume,
        // not the first.
        const { ids, marked } = offeredOf(session);
        return Object.assign({}, drawer, {
            acts: ids.map(id => ({
                name: Words.name(`verb:agent:${id}`),
                key: id === marked ? "\u21b5" : ""
            }))
        });
    }

    // A drawer kept on its session's row is Joel reading what that session
    // answered him, which the daemon says in the talk's own file. The
    // drawer follows the cursor, so one only passed over on the way down
    // isn't read: it must stay two seconds, as a window must before the
    // sorter holds it (VKCF3Z, 2026-11-12, on #3547).
    Timer {
        id: staying

        property string address: ""

        interval: 2000
        onTriggered: {
            const launcher = provider.launcher;
            if (launcher?.open && provider.drawerWanted === `drawer:agents:${address}`
                && String(launcher.selected?.key ?? "") === `agent:${address}`)
                provider.heard(address);
        }
    }
    // The cursor leaving the row ends the stay, so coming back counts
    // two seconds afresh.
    Connections {
        target: provider.launcher

        function onSelectedChanged() {
            const launcher = provider.launcher;
            if (staying.address !== ""
                && String(launcher?.selected?.key ?? "") !== `agent:${staying.address}`) {
                staying.stop();
                staying.address = "";
            }
        }
    }

    // Joel has read what a session answered him.
    // Only his own reading counts: a walk's Switch goes to the session but
    // marks nothing read, and says nothing of it, since the walk did what it
    // asked.
    // It says when he opened it as `seen`, asked again as it was, so a retry taken late marks
    // read no answer that came since, which he never saw (round 000035 on #6796, F19).
    function heard(address) {
        if (Sprouting.fromJoel(launcher))
            rowsKind?.askTillTaken({ ask: "heard", address, seen: Date.now() });
    }

    // What the foot says once words have gone to a session.
    property string told: ""

    // The talks said to each session from the launcher, by its address in capitals, newest
    // first, until its talks say each was delivered (SessionTalks.js's toSession), so Return shows
    // the words sent at once.
    property var saidTo: ({})

    // What a talk to the session at `address` asks with its words, for the launcher to show
    // the talk under the field as Super+S's line does: the talks shown, newest first
    // (SessionTalks.js's spoken), named by `name`; the words sent once he's said them; and, once
    // it shows an answer he hasn't heard, that he has, so the daemon keeps it no longer open.
    function talking(address, name) {
        const to = String(address).toUpperCase();
        const served = () => Said.sessionTalks(saidTo[to] ?? [], servedTalks(to));
        const stages = Words.of("orchestrators:talk").said ?? {};
        const words = Object.assign({}, Words.of("agents:line").said ?? {}, {
            unsent: stages.failed ?? "",
            stages
        });
        const days = { yesterday: String(words.yesterday ?? ""),
            date: (date, format) => Qt.formatDate(date, format) };
        return {
            talks: () => served().map(one => Said.spoken(one, name, words,
                since => When.day(since, Date.now(), days))),
            said: text => {
                saidTo = Object.assign({}, saidTo, { [to]: [Said.toSession(text, Date.now())]
                    .concat((saidTo[to] ?? []).slice(0, 2)) });
            },
            heard: () => {
                if (served().some(Said.unheard))
                    provider.heard(to);
            }
        };
    }

    // Joel's answers to what each session asks, given in its Talk, by its address in capitals,
    // newest first (TalkAnswers.js's said), so the Talk shows each as sent, then answered.
    property var answeredTo: ({})
    // What each session's Talk last drew it asking that his words answer, by its address in
    // capitals, null where it asked nothing he can answer: his Return answers what he was
    // shown, not what's held by the time he presses it. Changed in place, so drawing it
    // notifies nothing.
    readonly property var answersShown: ({})

    // What the session `session` asks Joel that its Talk answers (TalkAnswers.js's asked): its
    // want's answers while it asks and isn't parked, as Now's rows read it (Now.qml's asking).
    function answersOf(session) {
        const asking = SessionRow.asks(session.presence) && !Sessions.parkedAs(session.address);
        return { want: wantOf(session.address), asking };
    }

    // Whether `session` asks Joel something his words in a Talk answer (TalkAnswers.asked).
    function answerable(session) {
        const now = answersOf(session);
        return TalkAnswers.asked(now.want, now.asking) !== null;
    }

    // What a Talk to `session` shows before its talks: what it asks in full while it asks, first
    // so it has the first exchange's eight lines (TalkSpoken.qml), then Joel's answers to it,
    // newest first (Joel at 13:15 on 2026-11-24, of a cell asking him from the realm's map: "no
    // feedback at all"), each in TalkLine.js's exchange shape.
    function answerTalks(session) {
        // A want that comes while the Talk is open draws it again (wantOf reads the held rows).
        void rowsKind?.revision;
        const to = String(session.address).toUpperCase();
        const said = Words.of("verb:agent:talk").said ?? {};
        const stages = Words.of("orchestrators:talk").said ?? {};
        const words = Object.assign({}, said, { sent: stages.sent ?? "",
            unsent: stages.failed ?? "" });
        const days = { yesterday: String((Words.of("agents:line").said ?? {}).yesterday ?? ""),
            date: (date, format) => Qt.formatDate(date, format) };
        const dayOf = since => When.day(since, Date.now(), days);
        const working = stateOf(session) === "working";
        const now = answersOf(session);
        const asked = TalkAnswers.question(now, session.title, words, dayOf);
        const answers = TalkAnswers.asked(now.want, now.asking);
        // His Return answers what the Talk drew (TalkAnswers.answering), which question draws
        // whenever it asks him something he can answer here, if only what it takes.
        answersShown[to] = answers;
        // While it asks, only answers to what it asks show, and those not delivered in the last
        // quarter hour.
        const want = TalkAnswers.shownWant(now, answers);
        // Words given back for an ask it no longer asks go, as the refusal may have come before
        // the want that replaced it; later, since this runs as the Talk draws. The Talk's own
        // opening lets them go first (letGoStale), so this is for a want that comes while open.
        if (letsGo(session, now, answers).stale.length > 0)
            Qt.callLater(() => letGoStale(session));
        return (asked === null ? [] : [asked]).concat(TalkAnswers
            .standing(answeredTo[to] ?? [], want, Date.now())
            .map(one => TalkAnswers.shown(one, working, words, dayOf)));
    }

    // What of `session`'s answers is stale, given what it asks `now` (answersOf's) and
    // `answers` (TalkAnswers.asked's): `stale`, the words given back for an ask it no longer
    // asks, and `standsFor`, the want it asks. While it asks and its want isn't held, as for a
    // moment after the daemon restarts, which want it asks isn't known, so none are stale.
    function letsGo(session, now, answers) {
        const to = String(session.address).toUpperCase();
        const standsFor = answers?.want ?? "";
        const stale = now.asking === true && answers === null ? []
            : TalkAnswers.stale(answeredTo[to], standsFor);
        return { stale, standsFor };
    }

    // Lets go of the words given back to `session` for an ask it no longer asks, at once: its
    // field's draft of them too (Drafts.js's settled), and, open, the field holding just them
    // (Launcher.qml's wordsLetGo), so they never answer the ask that replaced theirs. Run as
    // its Talk opens, before the field is given what's kept, and as it draws (answerTalks).
    function letGoStale(session) {
        const to = String(session.address).toUpperCase();
        const now = answersOf(sessionAt(session.address) ?? session);
        const going = letsGo(session, now, TalkAnswers.asked(now.want, now.asking));
        if (going.stale.length === 0)
            return;
        launcher?.drafts?.settle({ [to]: going.stale
            .map(typed => ({ words: typed, stage: "answered" })) });
        launcher?.wordsLetGo?.(to, going.stale);
        answeredTo = Object.assign({}, answeredTo,
            { [to]: TalkAnswers.letGo(answeredTo[to], going.standsFor) });
    }

    // Joel's `text` to `session`, which asks `answers`, answering it: shown sent at once, then
    // answered once the node has put it in the session's inbox, or why not (answerWant), naming
    // the want he was shown so a newer ask is never answered by it. His words stay kept under
    // its address, as a talk's do until delivered (Drafts.qml), so the field gives them back
    // where they never went while their ask stands, the foot saying why. Words naming none of
    // the choices it takes only are let go, the foot saying what it takes, so they're never
    // given back to answer a later ask.
    function answerIn(session, text, answers) {
        const to = String(session.address).toUpperCase();
        const said = Words.of("verb:agent:talk").said ?? {};
        const answer = TalkAnswers.answerOf(text, answers);
        if (answer === null) {
            told = TalkAnswers.filled(said.takesOnly ?? "", {
                choices: answers.choices.map(one => String(one.label ?? one.value))
                    .join(" \u00b7 ") });
            launcher?.drafts?.settle({ [to]: [{ words: text.trim(), stage: "answered" }] });
            // A walk from a terminal is told so, as nothing went (Launcher.qml's walkedRun).
            launcher?.walkRefused?.(told);
            return;
        }
        told = TalkAnswers.filled((Words.of("orchestrators").said ?? {}).sent ?? "",
            { name: session.title });
        const fields = Object.assign({ want: answers.want }, answer.value !== undefined
            ? { value: answer.value } : { other: answer.other });
        const chose = { label: answer.label, typed: text, answers };
        answerKept(session.address, fields, chose, (put, why, unheard, refused) => {
            // A second Return to an ask he answered goes nowhere, as it takes his first
            // (answerKept), so his words go and the foot says why rather than sent.
            if (refused === true) {
                launcher?.drafts?.settle({ [to]: [{ words: text.trim(), stage: "answered" }] });
                told = why;
                return;
            }
            // His words stay only for the ask they were meant for: once it asks something else,
            // given back they'd answer the new ask with words for the old one, so they show
            // only as not delivered, and the foot says why. Words not known to have arrived
            // go too, as they may have, so a second Return isn't laid out for him.
            const now = answersOf(sessionAt(session.address) ?? session);
            if (!TalkAnswers.keepsWords(put || unheard === true,
                TalkAnswers.asked(now.want, now.asking), answers))
                launcher?.drafts?.settle({ [to]: [{ words: text.trim(), stage: "answered" }] });
            if (unheard)
                told = TalkAnswers.filled(said.unheardWhy ?? "", { name: session.title });
            else if (!put && (why ?? "") !== "")
                told = why;
        });
    }

    // Who hears a line said from anywhere (TalkLine.qml), `{ to, name }`: the one standing at
    // Agents' top, the void, which passes it to whoever owns it (QCXZEB, 2026-11-13
    // [ae5dfdb6]); null while none stands there.
    function voidOf() {
        const top = Sessions.running.concat(Sessions.parked)
            .find(one => !!one.presence?.standsAtTop?.name);
        return top ? { to: String(top.address), name: String(top.presence.standsAtTop.name) }
            : null;
    }

    // Super+S's line, which TalkLine.qml sets here, so talking to the void from its row opens
    // that line rather than an ask of its own: one place to say anything to it (QCXZEB,
    // 2026-11-13 [ae5dfdb6], d).
    property var line: null

    // Whether the session at `address` is the void.
    function isVoid(address) {
        const heard = voidOf();
        return heard !== null && String(address ?? "").toUpperCase() === heard.to.toUpperCase();
    }

    // Talk to It on the void: the launcher goes, and the line comes up over the window it
    // leaves, once niri has given that window back the keys, so the line attaches it.
    function lineOpened(key) {
        return thing({
            key,
            verb: Words.verb(key),
            name: Words.name(key),
            glyph: Words.glyph(key),
            hue: "action",
            later: true,
            run: () => provider.line?.open()
        });
    }

    // The session in `window`, `{ to, name }`, as the daemon's tell answers without words
    // (modules/agents/src/tell.rs), or null where it's none's: what a line attaches of the
    // window it's said over, for the void to route it by.
    function sessionIn(window, then) {
        if (!rowsKind || window === null || window === undefined) {
            then(null);
            return;
        }
        // tell says the window's session as `in` beside the void it answers (KP0L6A's #4043),
        // and as the answer itself, `why` "window", before it.
        rowsKind.ask({ ask: "tell", window }, answer => {
            const session = answer?.in ?? (answer?.why === "window" ? answer : null);
            then(typeof session?.to === "string" && session.to !== ""
                ? { to: session.to, name: String(session.name || session.to) } : null);
        });
    }

    // Says `text` to whom a line goes, as a talk does. The talk line says it went and what
    // came back inside itself, never on a card elsewhere (VKCF3Z, 2026-11-13 [092d52ca]).
    // Said `now`, it goes into the run under way after its tool rather than waiting for the
    // turn to end (modules/agents/src/acts.rs).
    function say(heard, text, now) {
        if (!joelsOwn("talk"))
            return;
        const said = Object.assign({ ask: "talk", address: heard.to, text },
            now ? { now: true } : {});
        if (rowsKind)
            rowsKind.ask(said, answer => Sessions.steered(heard.to, answer));
        else
            Sessions.steered(heard.to, null);
    }

    // Joel's talks with the session at `address`, as Orchestrators.qml folds them.
    function talksOf(address) {
        return provider.source("orchestrators")?.talks?.[String(address).toUpperCase()] ?? [];
    }

    // The talks of the session at `address` as the agents daemon picks and orders them for the
    // field (modules/agents/src/inboxes.rs), read again as they move.
    function servedTalks(address) {
        void talksMoves;
        return (rowsKind?.held ?? {})[`talks:agents:${String(address).toUpperCase()}`]?.talks ?? [];
    }

    // Says `text` to the session at `address` as Talk does: what Take It Back tells the void.
    function talkTo(address, text) {
        provider.actOn({ address }, "talk", { text });
    }

    // Talk to It for whom `said` names, as `qs ipc call launcher talkTo` asks it (Launcher.qml's
    // talkTo): `whom`, as TalkTo.js finds it, and `act`, the session's own Talk, as its row's
    // is, or the void's line; none for a network, which serves no Talk, or for nobody here.
    function talkFor(said) {
        const held = heldAt;
        const whom = TalkTo.whom(said, {
            isVoid: address => provider.isVoid(address),
            session: address => held(address) !== null,
            network: name => networkKey(name) in (top.held ?? {})
        });
        const act = whom.to === "session" ? sessionAct(held(whom.address), "talk")
            : whom.to === "void" ? lineOpened("verb:agent:talk") : null;
        return { whom, act };
    }

    // The session running or parked at `address`, in capitals, as a link names it; null for
    // none here.
    function heldAt(address) {
        return Sessions.running.concat(Sessions.parked)
            .find(one => String(one.address).toUpperCase() === address) ?? null;
    }

    // Goes to the session whom `said` names, as `qs ipc call launcher goTo` asks it
    // (Launcher.qml's goToCell), as its row's Switch does: raises its window, or opens one on it
    // where it has none, held here, hosted or on another machine (Sessions.goTo), waking it
    // first where it's parked, as a look's Go To does. Only the first word said names one
    // (TalkTo.goesTo), and nothing is typed or sent. Gives the session and whether it went
    // (Sessions.wakeToGo), with the why it was told where it didn't (refusedGoing), or null for
    // nobody here: the void and a network have no window to go to.
    function goToFor(said) {
        const address = TalkTo.goesTo(said, {
            isVoid: address => provider.isVoid(address),
            session: address => heldAt(address) !== null,
            network: () => false
        });
        if (address === "")
            return null;
        const session = heldAt(address);
        heard(session.address);
        provider.refusedWhy = "";
        const went = Sessions.wakeToGo(Sessions.parkedAs(session.address),
            sessionAt(session.address) ?? session);
        return { session, went, why: went ? "" : provider.refusedWhy };
    }

    // The key the daemon serves a network's row by, among the rows the Networks row holds
    // (modules/agents' networks.rs).
    function networkKey(name) {
        return `row:${top.place}:network:${name}`;
    }

    // The rows a walk into Agents takes to the network `said` names, as `qs ipc call launcher
    // network` asks it (Launcher.qml's showNetwork): Networks, the fold it's under where its
    // heading holds more than a dozen, and its own row last, each as the place made it. Only the
    // first word said names one (TalkTo.js). `name` is that word, and `rows` is empty for a
    // name no network has.
    function networkWalk(said) {
        const name = TalkTo.named(said);
        const rows = [];
        for (let key = networkKey(name); top.served(key) !== null;
            key = top.served(key).parent ?? "")
            rows.unshift(top.rowAt(key));
        return { name, rows: rows.includes(null) ? [] : rows };
    }

    // What the foot says when talkTo names a network: that it didn't talk, and why, since a
    // network has no one lead to say anything to (VKCF3Z, [e1eb0ebe]).
    function leadless() {
        const said = Words.of("verb:agent:talk").said ?? {};
        return { what: said.didNot ?? "", why: said.leadless ?? "" };
    }

    // Words a talk delivered, or anything after, are let go of the field's
    // keeping (Drafts.qml); one that failed stays, for Joel to send again.
    Connections {
        target: provider.source("orchestrators")
        function onTalksChanged() {
            provider.launcher?.drafts?.settle(provider.source("orchestrators").talks);
        }
    }

    // The acts a session's row offers and the one its Return does, as the
    // agents daemon serves its row (SessionRow.offered), so its drawer and
    // its step in list the same; while its row isn't held, SessionRow.js's
    // own for how it stands.
    function offeredOf(session) {
        const row = rowsKind?.held?.[`row:agents:${session.address}`]?.row ?? null;
        return SessionRow.offered(row?.acts, row?.held?.returns,
            SessionRow.acts(stateOf(session), windowedOf(session), session.here));
    }

    // A session's own acts, in the order its row offers them, each steering
    // it through its inbox but Go To; an orchestrator's row is its
    // session's, with its talks after them.
    function sessionActs(session) {
        return offeredOf(session).ids.map(id => sessionAct(session, id));
    }

    // What Close leaves of a session, its worktree or folder and its quest's
    // to-dos, as the agents daemon serves it with its row's Close (row.rs's
    // act); "" before its row is told.
    function leaves(session) {
        return String(served(session)?.leaves ?? "");
    }

    // Whether Close costs enough to ask first, as the agents daemon serves
    // it by the work a session has in flight: costly while a turn is under
    // way, not while it listens, is paused or parked (row.rs's act). Before
    // its row is told, nothing, which asks as long as Close says what stays
    // (Operators.asks).
    function costly(session) {
        const said = served(session)?.costly;
        return typeof said === "boolean" ? { costly: said } : {};
    }

    // A session's Close as its row serves it, or undefined before it's told.
    function served(session) {
        const acts = rowsKind?.held?.[`row:agents:${session.address}`]?.row?.acts ?? [];
        return acts.find(one => one.key === "verb:agent:close");
    }

    // Whether a session has a window to go to: its own here, or one its
    // machine's node attaches elsewhere.
    function windowedOf(session) {
        return !!(session.window || session.kitty || (!session.here && session.attach));
    }

    // Wakes a parked session, its row reading as listening from this key's
    // frame (Joel, 2026-11-07: "it should show right away") until it's
    // parked no more, however it comes back, listening or at work.
    function wakeNow(session) {
        if (!joelsOwn("wake"))
            return;
        const key = `agent:${session.address}`;
        const id = String(session.presence?.session ?? "");
        const asleep = word("parked");
        const awake = word("listening");
        // Its row as Agents lays it out, folded or not, made as it's asked
        // for while Agents isn't shown, as at home or held in Now, so the
        // waking row keeps what it spent.
        const now = sessionRows(everything()).find(row => row.key === key);
        const items = now ? now.sideItems.map(item => (item.said === asleep
            ? Object.assign({}, item, { said: awake }) : item))
            : SessionRow.side(awake, "");
        const full = SessionRow.full(session.presence?.usage, Words.of("agents").said ?? {});
        provider.becomes(key, {
            side: items.map(item => item.said).join(" \u00b7 "),
            sideItems: items,
            // Woken, one waiting on Joel still waits on him, so his mark
            // keeps the alert's hue rather than a program's.
            hue: SessionRow.hue(now?.waitsOnJoel ? "needs" : "listening", full !== "")
        }, provider.footName(session), "wake", {
            done: () => !Sessions.parked.some(one => String(one.presence?.session ?? "") === id),
            within: Sessions.startsWithin
        });
        // One asleep in pi's host wakes as it's viewed, which is the
        // shell's to do, as Go To is; the daemon's wake reaches a holder.
        // One the node refuses, since no holder keeps it this boot, is
        // reopened where it left off, as Sessions' own Wake does.
        if (Sessions.wokenByViewing(session))
            Sessions.wake(session);
        else
            provider.actOn(session, "wake", null,
                answer => Sessions.reopensUnwoken(session, answer));
    }

    // Does one of a session's acts as the daemon serves it: its row's ask
    // for it, sent back as it is with what the act says, which the daemon
    // does by steering the session through the node (modules/agents/src/
    // acts.rs), so the shell steers none itself. The answer is said as a
    // steer's is, a refusal by Sessions' unsteered.
    // `took`, where given, may take the answer as its own, saying true,
    // before it's said as the steer's.
    function actOn(session, id, fields, took) {
        if (!joelsOwn(id))
            return;
        const acts = rowsKind?.held?.[`row:agents:${session.address}`]?.row?.acts ?? [];
        const ask = acts.find(one => one.key === `verb:agent:${id}`)?.ask
            ?? { ask: id, address: session.address };
        const said = Object.assign({}, ask, fields ?? {});
        if (!rowsKind) {
            Sessions.steered(session.address, null, id);
            return;
        }
        rowsKind.ask(said, answer => took?.(answer)
            || Sessions.steered(session.address, answer, id));
    }

    // Whether an act the daemon does as Joel (its AS_JOEL, main.rs) may go
    // now: only from his own Return, never a walk from outside (`qs ipc call
    // launcher runWith`, `launcher do`), which any program of his may ask, an
    // agent's among them, nor a link's goTo. The daemon takes the shell's asks as his and
    // can't tell the two apart, so this is their only guard and fails closed
    // (Sprouting.fromJoel); one a walk leaves to run once the launcher's gone keeps that it
    // was walked (Launcher.qml's runOnceGone). The shell's own steers to the node, which the
    // node judges as the person's by its peer (mycelium's asker::may_say), are each a row's
    // `joelsOnly` too, refused to a walk as it's chosen (Launcher.qml's joelsOnlyWhy): For
    // You's Deal With It, the disk guard's Talk, Ask's Talk It Over and Tokens' Pause. A
    // wake, as Go and a link's goTo send, says nothing as him. Refused, the walk is told why,
    // and only the journal says it, as a refused sprout's is: a notice would let any program
    // put one on his screen.
    function joelsOwn(act) {
        if (Sprouting.fromJoel(launcher))
            return true;
        const outside = actWalked();
        console.info(`launcher: agents' ${act} wasn't done from a walk: ${outside}`);
        launcher?.walkRefused?.(outside);
        return false;
    }
    function actWalked() {
        return String((Words.of("agents").said ?? {}).actWalked ?? "");
    }
    // The asks the agents daemon does only as Joel (main.rs's AS_JOEL),
    // sent as served from its top rows only when he chose them; any
    // other ask, and his own, goes as it is.
    readonly property var asJoel: ["pause", "resume", "stop", "close", "wake", "talk",
        "tell", "feedback", "not-now", "put-off", "forget", "read", "heard", "sharing",
        "keep-places"]
    function joelsAsk(ask) {
        return Sprouting.fromJoel(launcher) || !asJoel.includes(ask?.ask) ? ask : null;
    }

    // Reopens a closed orchestrator, its row reading as listening until its
    // session is up, so Return on it then goes to that session rather than
    // starting a second while the first is still loading. One with no
    // session to resume starts afresh on its quest, under a new address;
    // one up and parked since is up too.
    function reopenNow(one, orchestrators) {
        const its = session => session.address === one.address
            || (!!one.quest && session.presence?.quest === one.quest);
        const up = () => Sessions.running.find(its);
        const awake = word("listening");
        provider.becomes(`orchestrator:${one.address}`, {
            side: awake,
            sideItems: [{ said: awake, keep: 3 }],
            hue: SessionRow.hue("listening", false),
            verb: Words.verb("verb:agent:switch"),
            run: () => {
                const session = up();
                if (session)
                    Sessions.goTo(session);
            }
        }, one.name, "reopen", {
            done: () => up() !== undefined || Sessions.parked.some(its),
            within: Sessions.startsWithin
        });
        orchestrators.start(one);
    }

    // One of a session's acts, by its id in SessionRow.js's acts. Go To
    // and Wake take the session as it stands when pressed, since its row
    // is kept while its window moves, and Switch would otherwise go to
    // the old one (Y2YBTS's review of #2521).
    function sessionAct(session, id) {
        const current = () => provider.sessionAt(session.address) ?? session;
        const act = (id, fields) => thing(Object.assign({
            key: `verb:agent:${id}`,
            verb: Words.verb(`verb:agent:${id}`),
            name: Words.name(`verb:agent:${id}`),
            glyph: Words.glyph(`verb:agent:${id}`),
            hue: "action"
        }, fields));
        const steered = what => () => provider.actOn(current(), what);
        const talked = () => {
            const talk = provider.talking(session.address, session.title);
            const talkWords = Words.of("verb:agent:talk");
            // While it asks Joel something, Return answers it, as its inbox goes unread until
            // its ask ends, and the Talk shows what it asks first (answerTalks).
            const asks = () => {
                const now = provider.answersOf(current());
                return TalkAnswers.asked(now.want, now.asking);
            };
            const draft = String(session.address).toUpperCase();
            return act("talk", {
                ask: asks() !== null ? talkWords.said?.answerAsk ?? "" : talkWords.ask ?? "",
                chip: session.title,
                // Kept by whom it goes to until its talk is delivered.
                draft,
                // Words given back for an ask it no longer asks go before the field opens on
                // what's kept, so it never opens on them (letGoStale).
                opening: () => provider.letGoStale(current()),
                state: () => provider.told,
                talks: () => provider.answerTalks(current()).concat(talk.talks()),
                heard: talk.heard,
                given: words => {
                    const text = words.trim();
                    if (text === "")
                        return;
                    // What the Talk showed it asking when he pressed Return, while that Talk is
                    // the one held, else what it asks now. An answer goes as Joel's, so only
                    // his own Return gives one: a walk from outside, which any program of his
                    // may ask, is refused (TalkAnswers.onReturn).
                    const ret = TalkAnswers.onReturn(TalkAnswers.answering(
                        provider.launcher?.drafting, draft, provider.answersShown, asks()),
                        Sprouting.fromJoel(provider.launcher));
                    if (ret.act === "refused") {
                        provider.told = TalkAnswers.filled(talkWords.said?.answerWalked ?? "",
                            { name: session.title });
                        provider.launcher?.walkRefused?.(provider.told);
                        return;
                    }
                    if (ret.act === "answer") {
                        provider.answerIn(current(), text, ret.answers);
                        return;
                    }
                    // Refused, nothing's kept as said, so the Talk never shows it going.
                    if (!provider.joelsOwn("talk"))
                        return;
                    provider.actOn(session, "talk", { text });
                    talk.said(text);
                    provider.told = (Words.of("orchestrators").said ?? {}).sent
                        ?.replace("{name}", session.title) ?? "";
                }
            });
        };
        const made = {
            // Going to the session is reading its answer there.
            switch: () => act("switch", { later: true, run: () => {
                provider.heard(session.address);
                Sessions.goTo(current());
            } }),
            // The void is talked to on Super+S's line, its one place (lineOpened).
            talk: () => provider.isVoid(session.address) ? lineOpened("verb:agent:talk")
                : talked(),
            // Wake keeps the launcher, its row reading as it will be at once,
            // since the session wakes in its own window without the focus.
            wake: () => act("wake", { stays: true, run: () => provider.wakeNow(current()) }),
            pause: () => act("pause", { run: steered("pause") }),
            resume: () => act("resume", { run: steered("resume") }),
            // Stop ends the turn under way, which nothing puts back, so it's
            // an ending: Return writes :interrupt and a second runs it, as d
            // does. It doesn't ask, as :interrupt is guarded, since the turn
            // lost is the one stopped for going wrong.
            stop: () => act("stop", Object.assign({ run: steered("stop"),
                askedAs: session.address }, Words.ending("verb:agent:stop"))),
            // Close ends the session as /exit does, its window with it,
            // asked first, saying what stays (Confirm.question). The ask
            // names it by its address, since the step-in's path above
            // already says its title, which left no room for the ask.
            close: () => act("close", Object.assign({ run: steered("close"),
                leaves: leaves(session), askedAs: session.address }, costly(session),
                Words.ending("verb:agent:close")))
        };
        return made[id]();
    }

    // When a session started, as a clock says it, with its day when that
    // wasn't today.
    function started(at) {
        const when = new Date(at);
        const today = new Date().toDateString() === when.toDateString();
        return Qt.formatDateTime(when, today ? "HH:mm" : "ddd d MMM, HH:mm");
    }

    function verbs(session, quest, spent, family) {
        const presence = session.presence ?? {};
        // Each fact a part, named first (agents:task and the rest), saying
        // its value and copying it.
        const fact = (key, name, side) => copyRow({
            key: `fact:agent:${key}`,
            name: name,
            side: side,
            copy: side,
            glyph: Theme.glyph.agent
        });
        const named = (id, state) => Words.name(`agents:${id}`, state);
        const addresses = presence.addresses ?? [session.address];
        // What of its spend went on its councils and fleets, hidden with it,
        // each named by what it went on.
        const helpers = spent === "" ? []
            : (SpendWatch.helpers[presence.session] ?? [])
                .map(helper => fact(`on:${helper.on}`,
                    helper.on.charAt(0).toUpperCase() + helper.on.slice(1), helper.spent));
        // Its heavy work, what runs and then what waits, as the queue says.
        const jobs = presence.session ? Jobs.of(presence.session).map(job => copyRow({
            key: `fact:agent:job:${job.id}`,
            name: job.what,
            side: job.state === "waiting" ? `${Numbers.ordinal(job.place)} in line` : "running",
            glyph: Theme.glyph[job.state === "waiting" ? "queued" : "running"]
        })) : [];
        const facts = [
            quest !== "" ? fact("quest", named("task"), quest) : null,
            fact("address", named("address", addresses.length === 1 ? "" : "several"),
                addresses.join(", ")),
            presence.owner?.startedAt
                ? fact("started", named("started"), started(presence.owner.startedAt)) : null,
            family ? fact("spent", named("spent"), word("allOf", { spent: family.all }))
                : spent !== "" ? fact("spent", named("spent"), spent) : null,
            family ? fact("own", named("own"), family.own) : null,
            family ? fact("under", named("under"), family.under) : null
        ].concat(helpers, jobs, [
            presence.worksOn ? fact("works", named("works"), presence.worksOn) : null,
            (presence.groups ?? []).length > 0
                ? fact("hears", named("hears"), presence.groups.join(", ")) : null
        ]).filter(row => row !== null);
        // Its acts, then what's known of it, then what Joel said to it and
        // what came of each, newest first.
        const talks = source("orchestrators")?.talksOf(session.address) ?? [];
        return sessionActs(session).concat(facts, talks);
    }

    function search(typed) {
        const words = Fuzzy.words(typed);
        const need = Fuzzy.letters(words);
        const found = [];
        for (const row of sessionRows(topServed ? treeRows() : all)) {
            const byName = (row.prepared.mask & need) === need
                ? Fuzzy.match(row.prepared, words, false) : { score: -1 };
            const byWord = (row.also.mask & need) === need
                ? Fuzzy.match(row.also, words, false) : { score: -1 };
            const best = byName.score >= byWord.score ? byName : byWord;
            if (best.score >= 0)
                found.push({ score: best.score, grade: best.grade, row: row });
        }
        const grade = Math.min(3, ...found.map(f => f.grade));
        const kept = Order.sorted(found.filter(f => f.grade === grade),
            (a, b) => b.score - a.score);
        // Ranked, each reads flat, at no depth (the header says why).
        return { rows: kept.map(f => f.row.ranked ? f.row.ranked() : f.row), grade: grade };
    }

    // Starting a session, Agents' own last row, after the sessions running
    // (Joel, 2026-10-28: "i'm also not able to start an orchestrator agent
    // session from the launcher ... it is also a bit non-intuitive"): one
    // row listing the unfinished quests with no session, each the quest's
    // own row, whose Return starts one on it and whose step in holds Start
    // Here, then Orchestrate (Quests.qml), so whether it orchestrates is
    // an act, never a question asked first (VKCF3Z, 2026-11-07
    // [9a4b1dd0]). A closed orchestrator reopens on its own row. Found by
    // its name from anywhere, and folded, so home names the sessions,
    // never it.
    function source(name) {
        return (provider.launcher?.sources ?? []).find(one => one.settings.name === name) ?? null;
    }

    // Made of its words alone, what it holds read as it's stepped into.
    readonly property var start: readNow ? ownKept("start", [Words.all, provider.weight],
        startMade) : madeBefore("start")
    function startMade() {
        const quests = () => (source("quests")?.everything() ?? [])
            .filter(row => row.unfinished && row.verb !== "go to");
        // Its drawer says what it holds, as every row's does (Joel,
        // 2026-11-11 22:00), never the quests, which may be many; a thing,
        // so it's drawn, as Capture's acts are.
        const words = Words.of("agents:start");
        return Object.assign(thingRow({
            key: "agents:start",
            name: Words.name("agents:start"),
            thing: true,
            drawer: () => ({
                eyebrow: (Words.field.drawer ?? {}).place ?? "",
                title: Words.name("agents:start"),
                body: (words.tells ?? {}).holding ?? ""
            }),
            glyph: Theme.glyph.agent,
            hue: "action"
        }, quests), {
            folded: true
        });
    }

    // Start New Work, after Start a Session (Joel, 16 November: starting new
    // work was too hard to do at all): From a Link, then one row for each
    // repository the open quests work in, each asking for a link or a few
    // words and asking the agents daemon to sprout them (Sprouting.js), a
    // quest of its own whose session starts held, its window opened for him
    // once it's up (openSprouted). Folded, found by its name, as Start a Session is.
    // Its rows are made only when it's stepped into (inside), so it costs
    // the first letter typed at home one folded row.
    readonly property var startNewWork: readNow ? ownKept("startNewWork",
        [Words.all, provider.weight], startNewWorkMade) : madeBefore("startNewWork")
    function startNewWorkMade() {
        const words = Words.of("agents:start-new-work");
        const inside = () => {
            provider.askPlaces();
            return [newWorkAct("link", "", Words.name("verb:agents:start-new-work:link"))]
                .concat(provider.cloudOffers.length > 0 ? [inTheCloud()] : [])
                .concat(startWheres().map(one => newWorkAct("in", one.folder,
                    Words.name("verb:agents:start-new-work:in")
                        .replace("{folder}", () => one.name), one.start)));
        };
        return Object.assign(thingRow({
            key: "agents:start-new-work",
            name: Words.name("agents:start-new-work"),
            thing: true,
            drawer: () => ({
                eyebrow: (Words.field.drawer ?? {}).place ?? "",
                title: Words.name("agents:start-new-work"),
                body: (words.tells ?? {}).holding ?? ""
            }),
            glyph: Theme.glyph.agent,
            hue: "action"
        }, inside), {
            folded: true
        });
    }

    // In the Cloud, Start New Work's row for work on a machine that costs
    // money, which the node picks (Cloud.js), listed only while the node's
    // providers say they could make one (askOffers): a step in to the same
    // places as the In rows, those another machine knows by name, each
    // started by that name. Its drawer says the cheapest machine's price
    // by the hour, where a provider gives one.
    function inTheCloud() {
        const words = Words.of("agents:start-new-work:cloud");
        const cheapest = Cloud.cheapest(provider.cloudOffers);
        const tells = words.tells ?? {};
        const body = cheapest === null ? tells.holding ?? ""
            : String(tells.costs ?? "").replace("{hourly}", () => cheapest.hourly)
                .replace("{currency}", () => cheapest.currency);
        return thingRow({
            key: "agents:start-new-work:cloud",
            name: Words.name("agents:start-new-work:cloud"),
            thing: true,
            drawer: () => ({
                eyebrow: (Words.field.drawer ?? {}).place ?? "",
                title: Words.name("agents:start-new-work:cloud"),
                body: body
            }),
            glyph: Theme.glyph.elsewhere,
            hue: "action"
        }, () => Cloud.wheres(startWheres(), startPlaces).map(one => newWorkAct("cloud",
            one.folder, Words.name("verb:agents:start-new-work:cloud")
                .replace("{folder}", () => one.name), one.start)));
    }

    // One of Start New Work's acts: `id` link, in or cloud, at `folder` where
    // one is given, as `name`, sprouting in `start`, the folder unless said,
    // and `on` where it starts elsewhere: "cloud", or a machine by its name
    // (Cloud.ask). In the Cloud's acts start in the cloud, each at a place's
    // name, read again at Return as an In act's start is.
    // Keyed by its folder, never its start, so a list of places landing
    // while Joel is on a row that changes how it starts (a quest's folder
    // found to be a zone) leaves him on the same row, not a dimmed one. Its
    // start is read again at Return, so a field entered before the list
    // landed starts as the row now says.
    function newWorkAct(id, folder, name, start, on) {
        const key = folder === "" ? `verb:agents:start-new-work:${id}`
            : `verb:agents:start-new-work:${id}:${folder}`;
        const elsewhere = id === "cloud" ? "cloud" : String(on ?? "");
        return thing({
            key: key,
            verb: Words.verb(`verb:agents:start-new-work:${id}`),
            name: name,
            glyph: Words.glyph(`verb:agents:start-new-work:${id}`),
            hue: "action",
            ask: Words.of(`verb:agents:start-new-work:${id}`).ask ?? "",
            // A folder's field opens on words a start refused for want of
            // one carried here (askWhere).
            offered: () => id === "in" || id === "cloud" ? provider.sproutCarried : "",
            state: () => provider.sproutSaid[key] ?? "",
            // The mark its state wears under the words: a glyph's name, running
            // while it's under way, done once started, failed when refused.
            mark: () => provider.sproutMarked[key] ?? "",
            // Words to start work are a sentence or two, so they take lines
            // of their own under the path (Prompt's composes), and stay in
            // muted ink once given while the start is under way, with the
            // moment it began, so the launcher says it took and counts
            // the seconds since (Joel, 2026-11-25 14:33: "no confirmation
            // that it took my enter").
            composes: true,
            taken: () => provider.underWay(key),
            since: () => provider.underWay(key) ? provider.sproutSince : 0,
            given: said => provider.sprout(key, said, start === undefined ? folder
                : Wheres.startAt(id === "cloud" ? Cloud.wheres(startWheres(), startPlaces)
                    : startWheres(), folder, start), elsewhere)
        });
    }

    // New work a mycelium: link offers, as `qs ipc call launcher newWork`
    // hands it (Launcher.qml's newWork): From a Link's act, the words to
    // fill its field with and what the foot says over them, naming the
    // session it came from by its quest's name where one here has that
    // address (Sprouting.offered); or null where it offers no plain words.
    // Its act starts nothing until Joel's own Return (sprout's fromJoel).
    // `on` says where the link's page would start it: "" where it's left to
    // the node, "cloud", or a machine by its name, which its act's name says.
    function newWorkFor(words, from, on) {
        const offer = Sprouting.offered(words, from, address => {
            const session = heldAt(address);
            return session === null ? null : Sessions.named(session).title;
        });
        if (offer === null)
            return null;
        // A link's act opens over Agents' rows, never through Start New
        // Work's, so it asks for the places itself: refused for want of a
        // repository, it steps back onto them (askWhere).
        provider.askPlaces();
        const said = sproutWords();
        // A link that names nowhere starts where Start New Work's From a Link does.
        const where = String(on ?? "") === "" ? {} : Cloud.onOf(Cloud.linked(on));
        const elsewhere = where === null ? "" : where.needs !== undefined ? "cloud"
            : where.on ?? "";
        // Its Return asks the cloud, so the offers are asked now too rather
        // than only by the opening after it, which may be up to half a
        // minute behind (Cloud.refused says so until they come).
        if (elsewhere === "cloud")
            provider.askOffers();
        const suffix = elsewhere === "" ? "" : elsewhere === "cloud" ? said.inTheCloud ?? ""
            : String(said.onMachine ?? "").replace("{machine}", () => elsewhere);
        const link = Words.name("verb:agents:start-new-work:link");
        // A misshaped `on` still fills the field, so Joel sees what the page
        // offered, but its Return is refused (Cloud.ask refuses "-"), as is the cloud's
        // while it offers nothing this node can take.
        const act = newWorkAct("link", "", suffix === "" ? link : `${link}, ${suffix}`,
            undefined, where === null ? "-" : elsewhere);
        // Fresh words, so how the last start ended is no longer theirs to show,
        // unless one is still starting, or the page offers again the words of a
        // start still coming up: its Return keeps that start's receipt and its
        // window's keys (sproutHeardAgain), which a stamp here would take.
        const offered = { key: act.key, words: String(offer.words ?? ""), on: elsewhere };
        if (sprouting !== act.key
            && StartSaid.underWayAgain(sproutsUnderWay(), offered) === null) {
            sproutPressed = Openings.pressedAt(sproutPressed, act.key, Date.now());
            sproutSays(act.key, "");
        }
        return {
            act: act,
            words: offer.words,
            from: offer.from,
            said: Sprouting.offeredSaid(offer, Words.of("agents:start-new-work").said)
        };
    }

    // How each Start New Work act stands, by its key, as that act says it:
    // starting, how it ended (Sprouting.js), why it asked nothing, or that
    // the last is still starting. One runs at a time; `sprouting` is the
    // key of the act it's for, "" while none runs, and `sprouts` counts
    // them, so an answer heard after its patience ran out says nothing.
    property var sproutSaid: ({})
    property var sproutMarked: ({})
    property string sprouting: ""
    property int sprouts: 0
    // When each act was last pressed, by its key, so a start still coming
    // up never says over what a later Return on its act says (said, below).
    property var sproutPressed: ({})
    // When the start running began, as Date.now() said it.
    property real sproutSince: 0
    // Whether the start `key` asked is under way: asked of the node, or started and
    // waiting for its session to come up before its window opens, which took three
    // minutes on 25 November. A window opened sooner was blank all that while.
    function underWay(key) {
        return sprouting === key || openings.some(one => one.key === key);
    }
    // The starts made this hour, started or unanswered (StartSaid.made), so
    // the same words given again to the same act say when they started
    // rather than starting the work twice (Joel, 2026-11-25: "no clue if i
    // did it once, twice, thrice or not at all"). Kept for the shell's life
    // alone, since the daemon keeps none of Joel's words (starts.rs).
    property var sproutsMade: []
    function sproutSays(key, text, mark) {
        sproutSaid = Object.assign({}, sproutSaid, { [key]: String(text ?? "") });
        sproutMarked = Object.assign({}, sproutMarked,
            { [key]: text ? mark ?? "failed" : "" });
    }
    // The start `one` under way (the one asked, which holds no `held`, or
    // an opening) given again by Joel's Return: what's pressed and typed
    // counts from this Return, so its window still opens and takes the keys
    // once it's up (Openings.takesKeys) rather than being left for Go To.
    function sproutHeardAgain(one) {
        const launcher = provider.launcher ?? null;
        const asked = one.held === undefined;
        const mark = given => Object.assign({}, given, { keys: launcher?.keysTaken ?? -1,
            letters: launcher?.lettersAt ?? 0 });
        const marking = () => {
            if (asked)
                sproutGiven = mark(sproutGiven);
            else
                openings = openings.map(each => each.held === one.held
                    && each.machine === one.machine ? mark(each) : each);
        };
        marking();
        // Marked again once the Return is through (Launcher.qml's give), as a
        // start's first Return is.
        Qt.callLater(marking);
    }
    // The starts under way, each { key, words, on }: the one asked of the
    // node, then every opening whose session is still coming up.
    function sproutsUnderWay() {
        return (sprouting === "" ? []
            : [Object.assign({ key: sprouting }, sproutGiven)]).concat(openings);
    }
    // `on` is "" for work started here, or where it starts elsewhere:
    // "cloud", or a machine by its name (Cloud.ask).
    function sprout(key, said, folder, on) {
        const elsewhere = String(on ?? "");
        const words = Words.of("agents:start-new-work").said ?? {};
        // Asked from outside, or with no launcher to say whose Return it
        // was, it starts nothing (Sprouting.fromJoel).
        const launcher = provider.launcher ?? null;
        if (!Sprouting.fromJoel(launcher)) {
            sproutSays(key, words.outside);
            launcher?.walkRefused(words.outside);
            // The walk is told, and it isn't Joel's, so only the journal
            // says it: a notice would let any program put one on his screen.
            console.info(`launcher: new work wasn't started: ${words.outside}`);
            return;
        }
        const pressed = Date.now();
        // The same row's Return again while its own start is under way,
        // asked or its session still coming up, keeps the receipt it
        // already says rather than saying it's busy or started over it: a
        // start in the cloud can take minutes, and Joel must still see it
        // took (2026-11-25 14:33: "no clue if i did it once, twice,
        // thrice"). Other words on the same row, as another page's link,
        // or the same for another place, are new work (StartSaid.underWayAgain).
        const asked = { key, words: said, on: elsewhere };
        const same = StartSaid.underWayAgain(sproutsUnderWay(), asked);
        if (same !== null) {
            sproutHeardAgain(same);
            console.info("launcher: new work's Return was heard again while it starts");
            return;
        }
        if (sprouting !== "") {
            // The start still asked says on as it comes up, over the busy words its
            // answer takes the place of, so its own act is left unstamped; any other
            // is stamped, so an older start's late words don't write over them.
            if (key !== sprouting)
                sproutPressed = Openings.pressedAt(sproutPressed, key, pressed);
            sproutSays(key, words.busy, "pending");
            sproutReaches({ started: false, why: words.busy });
            return;
        }
        // A walk's Return, refused above, stamps nothing, so no walk from
        // outside quiets a start still coming up. A link offer, which any
        // program may make while the launcher is free, stamps its own act's
        // row, which it gives fresh words (newWorkFor); the start is still
        // told in the foot or as a notice.
        sproutPressed = Openings.pressedAt(sproutPressed, key, pressed);
        const again = StartSaid.again(sproutsMade, asked, Date.now());
        if (again !== null) {
            sproutsMade = StartSaid.told(sproutsMade, again, Date.now());
            const told = again.answered ? words.again : words.againUnanswered;
            sproutSays(key, String(told ?? "").replace("{at}", StartSaid.clock(again)),
                again.answered ? "done" : "pending");
            console.info("launcher: new work wasn't started again: the same words were "
                + `asked at ${StartSaid.clock(again)}`);
            return;
        }
        // A Return to the cloud with none offered asks again first
        // (Cloud.asksNow), in case what held them passed, and is then
        // said as what is known: the why the last ask came back with, or,
        // with nothing heard yet, that they haven't come (Cloud.offeredNow).
        // A daemon missing answers at once, so its why is said this Return
        // (Cloud.returning).
        const offersKept = () => ({ asking: provider.cloudAsking, at: provider.cloudOffersAt,
            offers: provider.cloudOffers, unoffered: provider.cloudUnoffered });
        const offered = Cloud.onOf(elsewhere)?.needs === undefined
            ? Cloud.offeredNow(provider.cloudOffersAt, provider.cloudOffers,
                provider.cloudUnoffered)
            : Cloud.returning(offersKept, () => provider.askOffers(true), Date.now());
        const ask = elsewhere === "" ? Sprouting.ask(said, folder)
            : Cloud.ask(said, folder, elsewhere, offered);
        if (ask === null) {
            const why = elsewhere === "" || Sprouting.ask(said, "") === null
                ? Sprouting.refused(said, elsewhere === "" ? folder : "", words)
                : Cloud.refused(folder, elsewhere, offered, words);
            sproutSays(key, why);
            sproutReaches({ started: false, why: why });
            return;
        }
        sprouting = key;
        sproutSince = Date.now();
        sprouts += 1;
        // Its Return already counted in keysTaken (Prompt.qml), so only a key after counts.
        sproutGiven = { words: String(said), on: elsewhere, letters: launcher.lettersAt,
            keys: launcher.keysTaken, at: pressed };
        sproutCarried = "";
        sproutOn = elsewhere;
        const sprout = sprouts;
        // Marked again once the Return that gave the words is through (Launcher.qml's
        // give), so only a key after it counts.
        Qt.callLater(() => {
            if (sprouts === sprout)
                sproutGiven = Object.assign({}, sproutGiven, { letters: launcher.lettersAt });
        });
        sproutSays(key, elsewhere === "" ? words.starting : elsewhere === "cloud"
            ? words.startingCloud : String(words.startingOn ?? "")
                .replace("{machine}", () => elsewhere), "running");
        // Work elsewhere may wait on a machine being made (Cloud.PATIENCE_MS).
        sproutPatience.interval = elsewhere === "" ? Sprouting.PATIENCE_MS : Cloud.PATIENCE_MS;
        sproutPatience.restart();
        const token = Sprouting.token(Date.now(), Math.random());
        sproutTokens[token] = "asking";
        sproutToken = token;
        const heard = answer => {
            // Dropped with the daemon as it asked: the daemon kept the start before it
            // asked the node, so the record it serves again is the answer (sproutsKept),
            // or the patience's, never one of each. Told here at once, this shell's
            // "unanswered" and the record's end would both reach Joel. So too one the
            // patience ended unanswered, whose record answers it late (sproutsKept):
            // said told here, the daemon would drop that record unserved.
            if (answer === null && (sproutTokens[token] === "asking"
                || Object.keys(sproutUnanswered).includes(token))) {
                console.info("launcher: the agents daemon went as a start asked; "
                    + "awaiting its record");
                return;
            }
            const how = Sprouting.ended(answer, sproutWords());
            sproutTokens[token] = "ended";
            const late = sproutUnanswered[token];
            delete sproutUnanswered[token];
            if (sprout === sprouts && sprouting !== "")
                sprouted(how);
            else if (late !== undefined)
                lateEnded(how, late);
            else
                // Its record ended it first (sproutsKept), and was told then.
                sproutJournal(`heard after its record, ${Sprouting.reach(how,
                    sproutWords()).journal}`);
            sproutTold([token]);
        };
        // With no daemon to send it to, nothing is kept, so none is to come.
        if (rowsKind && rowsKind.wire !== null)
            rowsKind.ask(Object.assign({ start: token }, ask), heard);
        else
            sprouted(Sprouting.ended(null, sproutWords()));
    }
    // No answer at all is none (Sprouting.PATIENCE_MS).
    Timer {
        id: sproutPatience
        interval: Sprouting.PATIENCE_MS
        onTriggered: {
            if (provider.sprouting === "")
                return;
            // Ended here unanswered, so it may yet end, late (lateEnded).
            provider.sproutUnanswered[provider.sproutToken] = { key: provider.sprouting,
                asked: provider.sproutGiven.at };
            provider.sprouted(Sprouting.ended(null, provider.sproutWords()));
        }
    }
    function sproutWords() {
        return Words.of("agents:start-new-work").said ?? {};
    }

    // The daemon keeps each start from its ask until this shell says it
    // told Joel how it ended (starts.rs), so a start outlives a shell or a
    // daemon started again under it. `sproutTokens` is how this shell
    // stands on each it named, by token: "asking", "ended" or "told". One
    // kept that it never heard the end of, asked by a shell before it or
    // cut off by a restart, is told as a start's end is, and opens no
    // window, since nobody waits on it, once it has stood kept a moment
    // (`sproutSeen`, when first seen), its asker has surely told it, and
    // the daemon, asked again, still keeps it (sproutsChecked); every one
    // but those still asked or waited on is then said told
    // (Sprouting.untold). One still asked whose answer went with the
    // daemon is answered by its record. A told the daemon didn't take is
    // said again (sproutAgain).
    property var sproutTokens: ({})
    // Those the patience ended unanswered (sproutPatience), by token, each the
    // act it was asked from (`{ key, asked }`): each is ended late, once, by
    // whichever of its answer and its record comes first (lateEnded). One the
    // daemon said unanswered, or its record ended, was told as it ended.
    property var sproutUnanswered: ({})
    property var sproutSeen: ({})
    // The tokens of those to tell being asked for again (sproutsChecked).
    property var sproutChecking: ({})
    // The token of the start running, "" while none runs.
    property string sproutToken: ""
    function sproutsKept(held) {
        const kept = Object.values(held).filter(one => one?.kind === "start")
            .map(one => one.start ?? {});
        const now = Date.now();
        const stood = Sprouting.untold(kept, sproutTokens, sproutSeen, now);
        const answeredLate = kept.filter(one => Object.keys(sproutUnanswered)
            .includes(one.token));
        answeredLate.forEach(one => {
            const act = sproutUnanswered[one.token];
            delete sproutUnanswered[one.token];
            lateEnded(Sprouting.ended(one.ended, sproutWords()), act);
        });
        stood.waiting.forEach(token => {
            if (!Object.keys(sproutSeen).includes(token))
                sproutSeen[token] = now;
        });
        if (stood.waiting.length > 0)
            sproutAgainIn(Sprouting.SETTLE_MS);
        const checking = token => Object.keys(sproutChecking).includes(token);
        const tell = stood.tell.filter(one => !checking(one.token));
        const telling = tell.map(one => one.token);
        sproutTold(stood.told.filter(token => !checking(token) && !telling.includes(token)));
        if (tell.length > 0 && rowsKind) {
            telling.forEach(token => {
                sproutChecking[token] = true;
            });
            rowsKind.ask({ ask: "sprout-kept", starts: telling },
                answer => sproutsChecked(tell, answer));
        }
        // The start still asked, whose answer went with the daemon, ends as it kept it: last,
        // since sprouted looks at the kept starts again, and those being checked are marked.
        const answered = stood.answered.find(one => one.token === sproutToken);
        if (answered !== undefined && sprouting !== "")
            sprouted(Sprouting.ended(answered.ended, sproutWords()));
    }
    // Tells those of `tell` the daemon still keeps by its `answer` to
    // sprout-kept (Sprouting.stillKept): one its asker told since is kept
    // no more. With no answer, or a refusal, they're asked again in a moment.
    function sproutsChecked(tell, answer) {
        tell.forEach(one => {
            delete sproutChecking[one.token];
        });
        if (!Array.isArray(answer?.kept)) {
            sproutAgainIn(Sprouting.SETTLE_MS);
            return;
        }
        const still = Sprouting.stillKept(tell, answer);
        tell.forEach(one => {
            sproutTokens[one.token] = "ended";
        });
        still.forEach(one => {
            const how = Sprouting.ended(one.ended, sproutWords());
            sproutReaches(how);
            watchFor(how, { key: `start:${one.token}` });
        });
        if (still.length > 0)
            openSprouted();
        sproutTold(still.map(one => one.token));
    }
    // Watches for the session of `how`, a start no act waits on, from `act`
    // (Openings.watching), so Joel is told when it's up or lost. Whether it does.
    function watchFor(how, act) {
        const one = Openings.watching(openings, how, act, Date.now());
        if (one === null)
            return false;
        openings = openings.concat([one]);
        return true;
    }
    // How a start the patience ended ended after all, asked from `act`
    // (`{ key, asked }`): no act waits on it, so it's said on its act unless
    // that was pressed since, refused or unheld it's told as a notice, and
    // held it's watched as a kept one is, so Joel hears when it's up; its
    // keys long given back, it's said as one that shows in Agents. Ended
    // unanswered again, it's journalled only: the patience said so.
    function lateEnded(how, act) {
        const reach = Sprouting.reach(how, sproutWords());
        sproutJournal(`late, ${reach.journal}`);
        if (how.unanswered === true)
            return;
        if (Openings.saysOnAct(sproutPressed, act.key, act.asked))
            sproutSays(act.key, Sprouting.told(Openings.promised(how, { keys: -1 }),
                sproutWords()), how.started ? "done" : "failed");
        if (reach.told !== null)
            sproutNoticed([reach.told]);
        if (watchFor(how, act))
            openSprouted();
    }
    function sproutTold(tokens) {
        if (tokens.length === 0 || !rowsKind)
            return;
        tokens.forEach(token => {
            sproutTokens[token] = "told";
        });
        rowsKind.ask({ ask: "sprout-told", starts: tokens }, answer => {
            if (answer !== null && answer.error === undefined)
                return;
            // The daemon's words can name a session, so they're said as errorShown
            // says a daemon's error.
            const why = answer?.error === undefined ? "no daemon answered"
                : ActsSaid.errorShown(answer.error, Options.reports);
            console.info(`launcher: a start's told wasn't taken: ${why}`);
            tokens.filter(token => sproutTokens[token] === "told").forEach(token => {
                sproutTokens[token] = "ended";
            });
            sproutAgainIn(Sprouting.TOLD_AGAIN_MS);
        });
    }
    // Looks at the kept starts again in `ms`, or sooner if already due.
    function sproutAgainIn(ms) {
        if (sproutAgain.running && sproutAgain.interval <= ms)
            return;
        sproutAgain.interval = ms;
        sproutAgain.restart();
    }
    Timer {
        id: sproutAgain

        onTriggered: provider.sproutsKept(provider.rowsKind?.held ?? {})
    }
    // How the running act ended, said on it and in the journal; another
    // may start from here. One started opens once its session is up.
    function sprouted(how) {
        const key = sprouting;
        sprouting = "";
        sproutPatience.stop();
        // Ended here, by its answer or by the patience running out, so the
        // daemon's record of it, heard already or to come, is said told.
        if (sproutTokens[sproutToken] === "asking") {
            sproutTokens[sproutToken] = "ended";
        }
        sproutToken = "";
        const where = Sprouting.asksWhere(how) && askWhere(key);
        const text = where ? Sprouting.toldWhere(how, sproutWords())
            : Sprouting.told(Openings.promised(how, sproutGiven), sproutWords());
        sproutSays(key, text, how.started ? how.held !== "" ? "running" : "done" : "failed");
        // Told Joel before the daemon is told it's told, so it's kept until he is.
        sproutReaches(how, where ? text : null);
        sproutsKept(rowsKind?.held ?? {});
        Report.say(how.started ? `sprouted new work on ${how.machine}`
            : `couldn't sprout new work: ${how.why}`);
        sproutsMade = StartSaid.made(sproutsMade,
            { key: key, words: sproutGiven.words, on: sproutGiven.on }, how, Date.now());
        if (how.started && how.held !== "") {
            openings = openings.concat([Openings.opening(how, key, sproutGiven, Date.now())]);
            openSprouted();
        }
    }

    // Says how a start ended beyond its own row (Sprouting.reach): in the
    // journal always, and told Joel when it opens no window, so no start
    // he gives ends unseen.
    function sproutReaches(how, toldAs) {
        const reach = Sprouting.reach(how, sproutWords());
        console.info(`launcher: ${reach.journal}`);
        const told = toldAs ?? reach.told;
        if (told !== null)
            sproutTell([told]);
    }

    // The words of the start running, and the launcher's lettersAt and
    // keysTaken and the time at its Return; and the words a start refused
    // for want of a repository carries to the folder Joel picks next, ""
    // when none does.
    property var sproutGiven: ({ words: "", letters: 0, keys: -1, at: 0 })
    property string sproutCarried: ""
    // Where the start running was asked to run: "" where the node picks,
    // "cloud", or a machine by its name (sprout's `on`).
    property string sproutOn: ""
    // Steps Joel out of the act `key`, whose start wanted a repository, back
    // onto Start New Work's folders, its words carried to whichever he picks
    // (Sprouting.asksWhere), only while he's still in its field to be
    // stepped back (Sprouting.stepsBack). Whether it did.
    function askWhere(key) {
        const launcher = provider.launcher ?? null;
        if (!Sprouting.stepsBack(launcher, key, sproutGiven.letters, startWheres().length))
            return false;
        // A link's From a Link (newWorkFor) opens over Agents' rows, not Start New Work's, so
        // stepping out of it steps into Start New Work too.
        // In the Cloud's acts step back to its own rows, so the words then
        // start in the cloud as he meant, not here. A link's act asked to
        // run elsewhere has no rows of its own there, so it stays where it
        // is, its refusal told, and carries no words to a local row.
        const under = launcher.held[launcher.held.length - 2]?.row?.key ?? "";
        if (provider.sproutOn !== "" && under !== "agents:start-new-work:cloud")
            return false;
        sproutCarried = sproutGiven.words;
        launcher.back();
        if (under !== "agents:start-new-work" && under !== "agents:start-new-work:cloud")
            launcher.enter(startNewWorkMade());
        console.info("launcher: new work wanted a repository; asking where it starts");
        return true;
    }
    // Where Start New Work can begin, as the realm's node last listed them
    // (the agents daemon's places.rs), asked again the first time Start New
    // Work is stepped into at an opening, so a zone or checkout made since is
    // there by the next, and read once as each list comes (Wheres.listable).
    // Never as it opens: a node's 2000 places are some 200 KB for the
    // shell's thread to read, which every opening paid, For You's among them,
    // though only Start New Work lists them (25 November). None until it first
    // answers, which leaves the quests' folders; where it can't, the last
    // list stays, why journalled once as each new why comes rather than at
    // every opening (Wheres.heard). One ask is out at a time, so a node
    // that's stuck isn't asked again at each opening; the daemon answers
    // every ask, with none when it goes (Daemon.qml's gone). A new list
    // says the rows changed, so one landing while Joel is inside Start New
    // Work is drawn there and then, not at the next opening.
    property var startPlaces: []
    property string startPlacesUnlisted: ""
    property bool startPlacesAsking: false
    // Whether they were asked since the launcher opened, so its rows made
    // again as the list lands don't ask again.
    property bool startPlacesAsked: false
    function askPlaces() {
        const daemon = rowsKind ?? null;
        if (daemon === null || provider.startPlacesAsking || provider.startPlacesAsked)
            return;
        provider.startPlacesAsking = true;
        provider.startPlacesAsked = true;
        daemon.ask({ ask: "places" }, answer => {
            provider.startPlacesAsking = false;
            const begun = Date.now();
            const heard = Wheres.heard(provider.startPlacesUnlisted, answer);
            const differs = heard.places !== null
                && Wheres.differs(provider.startPlaces, heard.places);
            // Read outside every opening's own measure, so a list that
            // holds the thread past a frame or three says so itself.
            const took = Date.now() - begun;
            if (took > 50)
                console.info(`launcher: places read in ${took} ms`);
            if (differs) {
                provider.startPlaces = heard.places;
                provider.changed();
            }
            provider.startPlacesUnlisted = heard.unlisted;
            if (heard.journal !== null)
                console.info(`launcher: no places to start in: ${heard.journal}`);
        });
    }
    // What the node's providers say they could make, as the agents daemon's
    // offers.rs last heard it (Cloud.heard), so In the Cloud is listed only
    // where a machine can be had. A provider's program runs for each ask,
    // for up to half a minute, so it's asked at an opening only once the
    // last answer is ten minutes old (Cloud.stale), one ask out at a time;
    // where it can't say, the last offers stay, why journalled once as each
    // new why comes. New offers, or none where there were some, draw
    // Start New Work's rows again.
    property var cloudOffers: []
    property real cloudOffersAt: 0
    property string cloudUnoffered: ""
    property bool cloudAsking: false
    // `pressed` where a start's Return asks (Cloud.asksNow).
    function askOffers(pressed) {
        const kept = {
            asking: provider.cloudAsking,
            at: provider.cloudOffersAt,
            offers: provider.cloudOffers
        };
        if (!Cloud.asksNow(kept, Date.now(), pressed === true))
            return;
        // With no daemon to ask, no answer comes, so it is heard as none
        // (Cloud.heard's NO_ANSWER), never left as not heard yet for good.
        const daemon = rowsKind ?? null;
        if (daemon === null) {
            provider.offersCame(null);
            return;
        }
        provider.cloudAsking = true;
        daemon.ask({ ask: "offers" }, answer => {
            provider.cloudAsking = false;
            provider.offersCame(answer);
        });
    }
    // What an offers `answer` changes (Cloud.answered), kept and journalled.
    function offersCame(answer) {
        const kept = Cloud.answered({
            offers: provider.cloudOffers,
            at: provider.cloudOffersAt,
            unoffered: provider.cloudUnoffered
        }, answer, Date.now());
        provider.cloudOffersAt = kept.at;
        provider.cloudUnoffered = kept.unoffered;
        if (kept.changed) {
            provider.cloudOffers = kept.offers;
            provider.changed();
        }
        if (kept.journal !== null)
            console.info(`launcher: the cloud offers nothing known: ${kept.journal}`);
    }
    // Start New Work's In rows (Wheres.wheres): his quests' folders, then the
    // node's places, a big repository's zones among them.
    function startWheres() {
        return Wheres.wheres(source("quests")?.folders() ?? [], startPlaces);
    }
    // Tells Joel how starts ended: in the foot while the card is up, and
    // as a notice while it's shut or if it shuts soon after the foot said
    // it (sproutFooted, Sprouting.noticedAtShut), since a shut launcher may
    // not open for hours. That's a notice where an act's end is otherwise the
    // foot's alone (Joel, 18 November, of a start he shut the launcher on
    // five seconds after: "things I send in must be seen through").
    function sproutTell(told) {
        const launcher = provider.launcher ?? null;
        const name = Words.name("agents:start-new-work");
        if (launcher !== null && (launcher.open || launcher.opening)) {
            const now = Date.now();
            const unread = Sprouting.noticedAtShut(sproutFooted, launcher.keysTaken, now)
                ? sproutFooted.told : [];
            const shown = unread.concat(told).map(text => [name, text]);
            launcher.tellOnOpen(shown, true);
            sproutFooted = { told: unread.concat(told), keys: launcher.keysTaken, at: now,
                shown };
        } else {
            sproutNoticed(told);
        }
    }
    property var sproutFooted: null
    function sproutNoticed(told) {
        const name = Words.name("agents:start-new-work");
        Niri.spawn(["notify-send", "--app-name=Agents", "--", name,
            Sprouting.noticeBody(told)]);
    }
    // The launcher shut, or its opening was let go: what the foot said of
    // a start, unread by Sprouting.noticedAtShut, is noticed, and the foot
    // forgets it, so the next opening doesn't say it again.
    function sproutShut() {
        const footed = sproutFooted;
        const launcher = provider.launcher ?? null;
        sproutFooted = null;
        sproutCarried = "";
        // No opening takes the keys once he's shut it, one still asked included, so
        // an act that promised a window says instead it shows in Agents once it's up.
        Openings.unpromised(openings).forEach(each => {
            if (Openings.saysOnAct(sproutPressed, each.one.key, Openings.askedOf(each.one)))
                sproutSays(each.one.key, Sprouting.told(each.how, sproutWords()), "pending");
        });
        openings = Openings.shutOn(openings);
        sproutGiven = Openings.spent(sproutGiven);
        if (launcher === null
            || !Sprouting.noticedAtShut(footed, launcher.keysTaken, Date.now()))
            return;
        sproutNoticed(footed.told);
        footCleared(launcher, footed.shown);
    }

    // The sprouts waiting for their sessions, each `{ key, held, machine,
    // since, asked, keys, letters, late, told }` (Openings.opening), `since`
    // the time of its answer, `asked`, `keys` and `letters` the time,
    // keysTaken and lettersAt at its Return (keys -1 once the launcher shut,
    // or with none, so it never takes the keys), `late` true once it's said to
    // be still coming up and `told` the words that said it (Openings.toldOf):
    // Joel asked for each, so each opens for him, on the workspace a session
    // goes to and focused (Sessions.goTo), rather than in no window (Joel,
    // 18 November: "start new work does not seem to work from the
    // launcher"). A list, so a second sprout never loses the first.
    property var openings: []
    // A minute before one still coming up is said to be, and a quarter hour,
    // near three times the slowest start seen (five and a half minutes, on
    // a loaded machine), before it's watched no more.
    readonly property var openingPatience: ({ patience: 60000, watch: 15 * 60000 })
    // Each that's up opens and goes, but takes the keys only while Joel
    // still waits on its act in the launcher (Sprouting.waitsOn): once he's
    // at something else, what he types could land in an agent's prompt, so
    // he's told it's up, by its name, and Go To in Agents opens it. One not
    // up within the minute is said to be still coming up, and told again as
    // it comes up, however late, or once it's watched no more
    // (Sprouting.settled): Joel's at 14:30 on 25 November came up five and a
    // half minutes after his Return on a loaded machine, and was said
    // unopened at the minute and never again. What a pass says goes to the
    // foot for the starts Joel still waits on, and as a notice for the rest
    // (Openings.toldWhere).
    function openSprouted() {
        const stood = Sprouting.settled(openings, one => sessionOf(one), Date.now(),
            openingPatience);
        if (stood.late.length + stood.up.length + stood.gone.length === 0)
            return;
        const launcher = provider.launcher ?? null;
        const late = stood.late.map(one => ({ one, text: said(one, "unopened") }));
        openings = Openings.toldOf(stood.waiting, stood.late, late.map(each => each.text));
        stood.late.forEach(one => sproutJournal(
            `new work held ${one.held} on ${one.machine} is still coming up`));
        stood.gone.forEach(one => {
            sproutJournal(`new work held ${one.held} on ${one.machine} wasn't up in time`);
            // "Lost" answers its "still coming up", as an up word does.
            takeBack(one);
            // Lost, it no longer counts as made, so the same words again start
            // it rather than say it started and Go To opens it.
            sproutsMade = StartSaid.forgot(sproutsMade, one);
        });
        const told = late.concat(stood.gone.map(one => ({ one, text: said(one, "lost") })));
        stood.up.forEach(one => {
            const text = openOne(one);
            if (text !== "")
                told.push({ one, text });
        });
        const where = Openings.toldWhere(told,
            one => Sprouting.waitsOn(launcher, one.key, one.keys, one.letters));
        if (where.foot.length > 0)
            sproutTell(where.foot);
        if (where.notice.length > 0)
            sproutNoticed(where.notice);
    }
    function sessionOf(one) {
        return Sprouting.sproutedOf(Sessions.running, one.held, one.machine);
    }
    // Says `word` of `one` on its act, unless the act was pressed again
    // since (Openings.saysOnAct), and gives the words for the foot, its
    // session named where it's up (footName).
    function said(one, word) {
        const text = Openings.filled(sproutWords(), word, { machine: one.machine,
            title: one.session ? footName(one.session) : "",
            minutes: String(openingPatience.watch / 60000) });
        // Still coming up, or opening, it's still pending; up, it's done;
        // lost, it failed.
        const mark = word === "lost" ? "failed"
            : word === "unopened" || word === "opening" ? "pending" : "done";
        if (Openings.saysOnAct(sproutPressed, one.key, Openings.askedOf(one)))
            sproutSays(one.key, text, mark);
        return text;
    }
    // Opens `one`'s session if Joel still waits on its act, giving "", or
    // gives what the foot says of it left for Go To.
    function openOne(one) {
        const launcher = provider.launcher ?? null;
        const late = one.late === true ? " late" : "";
        // Its window, or its up word, answers the foot's "still coming up"
        // of it, so neither says it beside them nor notices it at the shut.
        takeBack(one);
        // Its session's address only with reports on, as main said it.
        Report.say(`new work held ${one.held} is session ${one.session.address}`);
        const waits = Sprouting.waitsOn(launcher, one.key, one.keys, one.letters);
        if (!Openings.takesKeys(waits, sproutPressed, one)) {
            sproutJournal(`new work held ${one.held} is up${late}; left for Go To`);
            return said(one, Openings.upWord(one));
        }
        said(one, "opening");
        launcher.open = false;
        Sessions.goTo(one.session);
        sproutJournal(`opening new work held ${one.held}${late}`);
        return "";
    }
    // Takes back what the foot said of `one` late (Openings.takenBack),
    // clearing the foot where nothing is left of it.
    function takeBack(one) {
        const launcher = provider.launcher ?? null;
        const taken = Openings.takenBack(sproutFooted, one);
        sproutFooted = taken.footed;
        if (launcher !== null && taken.cleared !== null)
            footCleared(launcher, taken.cleared);
    }
    // The launcher forgets the foot `shown` (sproutTell's), where it still
    // holds it to say.
    function footCleared(launcher, shown) {
        if (launcher.toldOnOpen === shown)
            launcher.toldOnOpen = [];
        if (launcher.saying === shown)
            launcher.say([]);
    }
    // What becomes of a start, to the journal on any machine, so a slow or
    // lost one can be matched to the node's record by its hold: as one line,
    // the node's words escaped as JSON escapes them, so no hold or machine
    // it names can break the line or forge another.
    function sproutJournal(what) {
        console.info(`launcher: ${JSON.stringify(String(what)).slice(1, -1)}`);
    }
    Timer {
        interval: 5000
        repeat: true
        running: provider.openings.length > 0
        onTriggered: provider.openSprouted()
    }

    // The sessions there are to reopen (Sessions.qml's toReopen): those the
    // last boot's end took away, gone on the boot they ran in or once put
    // off, and those other machines carried here or pi's host gave back,
    // offered whatever the boot until each runs here (VKCF3Z's design,
    // [fce929da], [17db838c]; 2QZBX9, [44e65ce0]).
    // One row after the sessions running while any are left, which For You
    // lists as a thing to do (Now.qml's waitingRows). Return steps in with
    // Reopen Them All first, so Return twice brings every one back, each in
    // its own window and folder; then each one, as a closed session,
    // reopening just it, the last boot's and then those apart, newest
    // first; then Not Now, which puts off only the last boot's, so it's
    // there only while any are. Its count is at its side, never in its
    // name, which would change as he reopens them, and its note says where
    // they came from.
    // Made of what Sessions offers and how it words each, Not Now's last
    // boot read as it's stepped into.
    readonly property var toReopen: reopenRead ? ownKept("toReopen", [Words.all, provider.weight,
        Sessions.toReopen, Sessions.reopenRows, Sessions.reopenPlace], toReopenMade)
        : madeBefore("toReopen")
    function toReopenMade() {
        const lost = Sessions.toReopen;
        if (lost.length === 0)
            return null;
        const said = Words.of("agents:to-reopen").said ?? {};
        const act = (id, fields) => thing(Object.assign({
            key: `verb:agents:to-reopen:${id}`,
            verb: Words.verb(`verb:agents:to-reopen:${id}`),
            name: Words.name(`verb:agents:to-reopen:${id}`),
            glyph: Words.glyph(`verb:agents:to-reopen:${id}`),
            hue: "action"
        }, fields));
        // Each row and the place come worded by the agents daemon
        // (modules/agents's reopen.rs): its name, the folder it reopens
        // in from home, or its machine, and the way it came, the last
        // boot, the machine that carried it or pi's host that gave it
        // back. Only the acts are the shell's.
        const rowOf = one => Sessions.reopenRows[one.id] ?? {};
        const place = Sessions.reopenPlace;
        // One given back names its host in its drawer alone, last, as an
        // identifier, since the host's name is no place Joel chose
        // (VKCF3Z, [92a1d30c]).
        const hostOf = one => !rowOf(one).host ? {} : {
            drawer: () => ({
                eyebrow: String(rowOf(one).note ?? ""),
                title: String(rowOf(one).name ?? ""),
                facts: [{ label: Words.name("agents:host"), value: String(rowOf(one).host),
                    identifies: true }]
            })
        };
        // In its own folder only, never the home folder instead. One whose
        // folder is gone says so on its row, looked for as the daemon makes
        // the list, and holds no Reopen (VKCF3Z, 2026-11-07 [4a00987e]);
        // one whose folder went since it was listed is said once the
        // launcher opens next, with how the rest went, never as a notice.
        const oneSaid = Words.of("verb:agents:to-reopen:one").said ?? {};
        const goneHere = one => rowOf(one).gone === true;
        const home = Sessions.home;
        const homely = folder => folder === home || folder.startsWith(`${home}/`)
            ? `~${folder.slice(home.length)}` : folder;
        const reopen = one => Sessions.reopen(one, cwd => provider.launcher?.tellOnOpen([
            [homely(cwd), String(oneSaid.isGone ?? ""), Theme.alert]]));
        // Forgetting one whose folder has gone costs nothing, since it can't
        // be reopened anyway, so d does it at once, though :forget is costly. The daemon takes
        // it as Joel's, so only his own Return forgets one (joelsOwn).
        const forget = one => act("forget", {
            run: () => {
                if (joelsOwn("forget"))
                    Sessions.forgetReopen(one);
            },
            costly: false
        });
        const worded = (one, fields) => Object.assign({
            key: `agents:to-reopen:${one.id}`,
            name: String(rowOf(one).name ?? one.id),
            side: String(rowOf(one).side ?? ""),
            note: String(rowOf(one).note ?? ""),
            glyph: Theme.glyph.agent
        }, fields, hostOf(one));
        const unreopened = one => thingRow(worded(one, { alerts: true, hue: "alert" }),
            () => [forget(one)]);
        const closed = one => goneHere(one) ? unreopened(one) : thing(worded(one, {
            thing: true,
            verb: Words.verb("verb:agents:to-reopen:one"),
            hue: "program",
            later: true,
            run: () => reopen(one)
        }));
        const reopenable = lost.filter(one => !goneHere(one));
        // Reopen Them All reopens those that can be, its side counting them
        // when any can't: "58 of 60 \u00b7 2 folders gone". Each found gone
        // after is counted with how many reopened, by the walk Sessions
        // joins the press to (Reopening.js's tallyOf), and said once the
        // launcher opens next (ReopenTally.js).
        const tellAll = now => {
            const [gone, reopened] = ReopenTally.told(now, said);
            provider.launcher?.tellOnOpen([[...gone, Theme.alert], reopened]);
        };
        // Its count of what failed is drawn in the alert's colour, as the
        // row that failed is, the rest as a side reads (VKCF3Z, 2026-11-07).
        const themAll = () => act("all", {
            later: true,
            side: String(place.allSide ?? ""),
            sideItems: place.allItems ?? [],
            // Each back where it sat (Sessions.qml's reopenThemAll), a
            // carried one beside where Joel is, since it never sat here,
            // and then the windows that reopen themselves. Each is told to
            // carry on as it opens, so none waits to be poked; one reopened
            // alone from its own row isn't, since Joel is there. Each walk
            // counts on its own, though the row is kept, and a press
            // mid-walk joins its count.
            run: () => Sessions.reopenThemAll(reopenable, tellAll)
        });
        return thingRow({
            key: "agents:to-reopen",
            name: Words.name("agents:to-reopen"),
            side: String(place.side ?? ""),
            note: String(place.note ?? ""),
            glyph: Words.glyph("agents:to-reopen"),
            hue: "action",
            // For You's filter knows it as a session's, to be done.
            forYou: { kind: "do", source: "sessions" }
        }, () => (reopenable.length === 0 ? [] : [themAll()]).concat(lost.map(closed),
            Sessions.lastBoot.length === 0 ? [] : [act("notNow", {
            run: () => {
                if (joelsOwn("not now"))
                    Sessions.putOffLastBoot();
            }
        })]));
    }

    Connections {
        target: provider.launcher

        function onOpenChanged() {
            provider.wrapAhead();
            provider.wrapForTyping();
            // Offers are a provider or two, asked at most every ten minutes
            // (Cloud.stale), so unlike the places they're asked at an opening:
            // In the Cloud is then listed the first time Start New Work opens.
            if (provider.launcher.open) {
                provider.startPlacesAsked = false;
                provider.askOffers();
                // A share's end is the shell's alone to tell, so one refused through every
                // try, as while the user manager is slow, is told again as Joel next looks,
                // which also ends a start any program may tell, as it only hides amounts
                // (round 000036 on #6796, F121 and F133; round 000035, F20).
                provider.tellSharing();
            }
            // Shut, no drawer shows, so none is watched.
            if (!provider.launcher.open) {
                provider.drawerWanted = "";
                provider.watchDrawers();
                provider.sproutShut();
            }
        }
        // The rows beside the one drawn move with the list, as a session
        // turns or another place is shown, which watches none beside it.
        function onMatchesChanged() {
            if (provider.drawerWanted !== "")
                provider.watchDrawers();
        }
        // An opening let go before it showed shuts as well: a tell told it
        // waits for an opening that may be hours off.
        function onOpeningChanged() {
            if (!provider.launcher.opening && !provider.launcher.open)
                provider.sproutShut();
        }
    }

    // What sessions landed, as the agents daemon serves them ready
    // (modules/agents/src/landed.rs): Landed, one row always, after
    // the sessions, the landings' own place, as a to-do lives in its app
    // (VKCF3Z, 2026-11-10). A step in, each landing of the past 30 days,
    // newest first: what it is as its name and its ref at the right in the
    // mono; its drawer says who landed it and when, and what to try, a
    // sentence the row has no room for beside its ref. Return goes to the
    // change, or to its session where no link to the change is known, and
    // marks it read, as opening a notice or a mail does: a mark, never an
    // ending, since no Return ends or takes away; its drawer shown reads it
    // too (`landingDrawn`). Its own step in holds Open the Change, Go To the
    // Session and Copy What to Try. Read, it stays, in Landed and in For
    // You's History, so nothing read is lost, and the daemon keeps what he
    // read. What landed is kept 30 days past its session's end, so a
    // session closed leaves what it landed in Landed. Its row says at its
    // right how many landed today, never how many are to read, which For
    // You alone counts (`forYou`), and its drawer, as a place's, what it
    // holds (VKCF3Z, adba1297).
    // Each landing is { key, at, ref, what, person, url, quest, repo,
    // machine, session: { address, title, id }, read }, `read` when Joel
    // read it, `true` before when was kept, or null; held as the daemon
    // tells each move (landedMoves), newest first by its own time, so the
    // shell works none of it out. It had worked out which of the realm's
    // two thousand were worth reading, and whose, at every session's
    // change (PT5P13, 2026-11-10).
    property int landedMoves: 0
    readonly property var landings: {
        void landedMoves;
        const held = Object.values(rowsKind?.held ?? {})
            .filter(one => one?.kind === "landed" && one.landed);
        return Order.sorted(held, Order.smaller(one => one.order)).map(one => one.landed);
    }
    // The landings of a day before the last one, as the day's own watch
    // tells them, newest first. The daemon tells a landing over a day old
    // only to a watch naming its date (landed.rs's landing_place), and the
    // shell watches only the day Joel is in (landedDayIn), so it makes no
    // row of a day he isn't in: all 4,270 of the past 30 days' landings
    // were held and made rows, most of what the shell weighed (WCJQSG,
    // bea3796d; 19 November). Those of the last day are Landed's own,
    // whatever their date, so each landing is listed in one place, the
    // one the daemon keeps it in, and none goes missing as midnight
    // passes before the daemon moves it.
    property int landedDayMoves: 0
    readonly property string landedDayIn: {
        const step = heldKeys().find(key => key.startsWith("agents:landed-day:")) ?? "";
        return step.slice("agents:landed-day:".length);
    }
    // A day's rows are let go once Joel leaves it, as its watch is.
    onLandedDayInChanged: {
        if (landedDayIn === "")
            landedDayKeeping.rows = {};
    }
    function landingsOn(day) {
        void landedDayMoves;
        const told = Object.values(landedDayKind?.held ?? {})
            .filter(one => one?.kind === "landed" && one.key.startsWith(`landed:${day}:`));
        return Order.sorted(told, Order.smaller(one => one.order)).map(one => one.landed);
    }
    // A landing's record by its own key, in Agents' place or the day's.
    function landingRecord(key) {
        void landedMoves;
        void landedDayMoves;
        const own = (rowsKind?.held ?? {})[`landed:agents:${key}`];
        return own ?? Object.values(landedDayKind?.held ?? {})
            .find(one => one?.kind === "landed" && one.landed?.key === key) ?? null;
    }
    // The landings listed where Joel is: the day he's in, else the newest.
    readonly property var landingsNear: landedDayIn === "" ? landings : landingsOn(landedDayIn)
    // Where each landing stands among them, by its key, so its drawer drawn
    // finds those beside it.
    readonly property var landingAt: {
        const at = new Map();
        landingsNear.forEach((one, index) => at.set(one.key, index));
        return at;
    }
    // What For You holds of them, as the daemon serves it (landed.rs's
    // `try`): each waiting in To Try, `read` null, and each read since,
    // for its History, with when, its landing beside it. For You had sifted
    // all two thousand for the few at each change.
    property int triesMoves: 0
    readonly property var tries: {
        void triesMoves;
        void landedMoves;
        const held = rowsKind?.held ?? {};
        const served = Object.values(held).filter(one => one?.kind === "try" && one.try);
        return Order.sorted(served, Order.smaller(one => one.order))
            .map(one => ({ one: held[one.try.landing]?.landed ?? null, read: one.try.read }))
            .filter(tried => tried.one !== null);
    }
    // The keys of the drawers watched ahead, each landing's served apart
    // and told only to a watch naming it (landed.rs): the one drawn and
    // those beside it, so an arrow to the next finds its drawer held.
    property var landingDrawersAhead: []
    function landingDrawerOf(one) {
        const at = landingAt.get(one.key) ?? -1;
        const near = [landingsNear[at - 1], one, landingsNear[at + 1]]
            .filter(beside => typeof beside?.drawer === "string")
            .map(beside => beside.drawer);
        if (near.join("\n") !== landingDrawersAhead.join("\n"))
            Qt.callLater(() => {
                provider.landingDrawersAhead = near;
            });
        // Read by what draws it, so it's drawn again as the daemon tells it.
        void rowsKind?.revision;
        return rowsKind?.held?.[one.drawer]?.drawer ?? null;
    }
    // How many landed each day, by date, as the daemon counts them by
    // Joel's clock, which Landed's row says of today.
    readonly property var landedDays: {
        void landedMoves;
        return (rowsKind?.held ?? {})["landed-days:agents"]?.days ?? {};
    }
    // The session here that landed `one`, by its id, or by its address only
    // for a landing that names no id, or null once it has ended. A landing
    // naming an id no session holds now has no lander: another session at
    // its address never saw it, and Talk would send Joel's words there.
    function landerOf(one) {
        const here = Sessions.running.concat(Sessions.parked);
        provider.tally.sought += here.length;
        if (one.session.id !== "")
            return here.find(session => session.presence?.session === one.session.id) ?? null;
        return here.find(session => session.address === one.session.address) ?? null;
    }
    // What a landing's row in For You offers of the session that landed it,
    // Talk and Go To as its own row does (SessionRow.js's behindActs), so
    // what the mesh said reaches whoever said it; none once it has ended,
    // and no Talk while it asks a question or a yes.
    function landerActs(one) {
        const session = provider.landerOf(one);
        if (session === null)
            return [];
        return SessionRow.behindActs(stateOf(session), windowedOf(session),
            SessionRow.asks(session.presence)).map(id => sessionAct(session, id));
    }
    // Joel reads a landing, which the daemon keeps, and tells as its record
    // moves; one read already is asked nothing. Only his own reading counts:
    // a walk that opens a landing reads nothing, and says nothing of it, as
    // heard's Switch doesn't, since the walk did what it asked.
    function readLanding(key) {
        const one = landingRecord(key)?.landed;
        if (!one || one.read !== null || !Sprouting.fromJoel(launcher))
            return;
        rowsKind.askTillTaken({ ask: "read", keys: [key] });
    }
    // Joel's words on a landing go to the quest that landed it, or its
    // session, as a finding the agents daemon sends and keeps
    // (modules/agents/src/feedback.rs), so what he makes of it reaches
    // whoever can act on it (WCJQSG, 2026-11-11). The foot says where they
    // went, by the quest's title or the session's name, never an address,
    // or why they didn't, in words for Joel rather than the plumbing's
    // (VKCF3Z, 2026-11-12).
    // What a session asks Joel, as the agents daemon serves it (wants.rs):
    // what it asked, since when, and the answers it offers, or null.
    function wantOf(address) {
        return (rowsKind?.held ?? {})[`want:agents:${String(address).toUpperCase()}`] ?? null;
    }
    // Answers what a session asks, `fields` its `value` or Joel's own words
    // as `other`: the daemon finds the want by the session's address and
    // steers it there, and `then(put)` hears whether the node put it in
    // the session's inbox, which Sessions says as it does any act's. Not
    // `answer`, which the launcher asks of every source for what's typed.
    // Only answerKept sends one, having refused a walk or a second answer first.
    function answerWant(address, fields, then) {
        const said = Object.assign({ ask: "answer", address: String(address) }, fields);
        if (!rowsKind) {
            then(Sessions.steered(address, null));
            return;
        }
        // No reply where it was sent is the daemon gone while asked (Daemon.qml's gone), the
        // answer put or not, so it's said unheard rather than not delivered; none where the
        // daemon wasn't there to send to is not delivered. An unheard one isn't Sessions'
        // steered, whose refusal would put "not told" on the foot beside "may not have it".
        const sent = rowsKind.wire !== null;
        rowsKind.ask(said, reply => {
            const why = String(reply?.error?.message ?? "");
            if (!TalkAnswers.unheard(sent, reply)) {
                then(Sessions.steered(address, reply), why, false);
                return;
            }
            Report.say(`unheard answer to ${String(address).toUpperCase()}: ${why}`);
            then(false, why, true);
        });
    }

    // Joel's answer to what the session at `address` asks (`chose.answers`, asked's), shown as
    // `chose.label` and typed as `chose.typed`, sent with `fields` (answerWant's) and kept
    // among its answers (answeredTo) until the node says of it, whichever row gave it, For
    // You's choices too, so its Talk shows it and a second Return there knows it went.
    // `then(put, why, unheard, refused)` hears the node, after it's kept, or `refused` true
    // where it never went, with why.
    function answerKept(address, fields, chose, then) {
        const to = String(address).toUpperCase();
        // An answer goes as Joel's, so it goes only from his own Return, whichever row gives it
        // (Sprouting.fromJoel), and only his first to an ask, For You's choices too. Refused,
        // nothing is kept, so its Talk shows no words he didn't send. A walk is told why
        // (Launcher.qml's walkedRun) and only the walk, since `then` says it in Joel's foot,
        // where a terminal's walk would write over what he reads (round 000035 on #6796,
        // F2); his own second Return hears it refused, its row saying why.
        const said = Words.of("verb:agent:talk").said ?? {};
        const name = sessionAt(address)?.title ?? to;
        const walked = TalkAnswers.walkRefused(Sprouting.fromJoel(launcher),
            said.answerWalked ?? "", name);
        if (walked !== null) {
            launcher?.walkRefused?.(walked);
            return;
        }
        const asked = { list: answeredTo[to], want: chose.answers.want, nowMs: Date.now() };
        const refused = TalkAnswers.answerRefused(true, asked, said, name);
        if (refused !== null) {
            launcher?.walkRefused?.(refused);
            then(false, refused, false, true);
            return;
        }
        const one = TalkAnswers.said(chose.label, Date.now(), chose.answers.want, chose.typed);
        const keep = changed => {
            answeredTo = Object.assign({}, answeredTo,
                { [to]: TalkAnswers.kept(answeredTo[to], changed) });
        };
        keep(one);
        answerWant(address, fields, (put, why, unheard) => {
            keep(TalkAnswers.put(one, put, why, unheard));
            then(put, why, unheard);
        });
    }

    property string fedBack: ""
    // What Joel said of each landing here, by its key, newest first, each a talk as
    // SessionTalks.js's toSession makes one with where it went as `stands`: shown until the
    // daemon serves it on the landing, and alone when it went as a finding, which no talk
    // follows.
    property var fedBackOn: ({})
    // What Joel said of the landing at `key` as the launcher shows a talk: the landing's talks
    // as the daemon serves them (modules/agents/src/feedback.rs's told), each said as a
    // session's talk is, after those said here it hasn't served yet.
    function feedbackTalks(key) {
        void landedMoves;
        const served = landingRecord(key)?.landed?.said ?? [];
        const said = Words.of("agents:landed").said ?? {};
        const stages = Words.of("orchestrators:talk").said ?? {};
        const words = Object.assign({}, Words.of("agents:line").said ?? {}, {
            unsent: stages.failed ?? "",
            stages
        });
        const days = { yesterday: String(said.yesterday ?? ""),
            date: (date, format) => Qt.formatDate(date, format) };
        const dayOf = since => When.day(since, Date.now(), days);
        const sentTo = to => to === "" ? String(said.sentAway ?? "")
            : String(said.sent ?? "").replace("{to}", to);
        const here = (fedBackOn[key] ?? []).filter(one => !served.some(kept =>
            Said.delivers(kept, one))).map(one => Object.assign(Said.spoken(one, "", words,
            dayOf), { stands: one.stands }));
        return here.concat(served.map(one => {
            const to = String(one.to ?? "");
            return Said.spoken(one, to, Object.assign({}, words, { sent: sentTo(to) }), dayOf);
        }));
    }
    function feedback(key, words) {
        const text = words.trim();
        if (text === "" || !joelsOwn("feedback"))
            return;
        const said = Words.of("agents:landed").said ?? {};
        const at = Date.now();
        const since = new Date(at).toISOString();
        const told = stands => {
            const talk = Object.assign(Said.toSession(text, at), { stands });
            fedBackOn = Object.assign({}, fedBackOn, { [key]: [talk].concat(
                (fedBackOn[key] ?? []).filter(one => one.at !== since)).slice(0, 3) });
        };
        fedBack = String(said.sending ?? "");
        told(fedBack);
        const landing = landingRecord(key)?.key ?? `landed:agents:${key}`;
        rowsKind?.ask({ ask: "feedback", landing, words: text },
            answer => {
                const to = String(answer?.to ?? "");
                const why = { "no-one": said.toNoOne, "no-mesh": said.noMesh }[
                    String(answer?.error?.code ?? "")];
                fedBack = answer?.sent
                    ? to === "" ? String(said.sentAway ?? "")
                        : String(said.sent ?? "").replace("{to}", to)
                    : why !== undefined ? String(why)
                    : String(said.unsent ?? "")
                        .replace("{why}", String(answer?.error?.message ?? ""));
                told(fedBack);
            });
    }
    // Where a landing lives, Agents \u203a Landed \u203a it, or through its day
    // where the daemon keeps it in its date's place, which For You's rows
    // step into rather than show a copy of (VKCF3Z, 2026-11-10). What
    // waits in To Try is of the last day, Landed's own.
    function landingTrail(one) {
        const record = String(landingRecord(one.key)?.key ?? "");
        const place = record.split(":")[1] ?? "agents";
        return ["collection:agents", "agents:landed"].concat(
            place === "agents" || place === "" ? [] : [`agents:landed-day:${place}`],
            [`agents:landed:${one.key}`]);
    }
    // A landing as both its rows show it, in Landed and in For You, one
    // component the same in both: when it landed at its side (Landed.js's
    // landedSide), and its drawer, which the daemon serves with all but
    // its time said (modules/agents/src/landed.rs). Its side is kept
    // by its moment for the day Calendars' hourly clock names
    // (landedSides), and the whole moment said only as its drawer is
    // drawn, since saying both for over a thousand landings at every
    // opening was most of the date work pegging the shell after one
    // (H6EDLC's profile, 2026-11-10). Said here, since a time is the
    // shell's to word as the day turns (modules/agents/src/clock.rs).
    readonly property var landedSides: ({ day: "", sides: new Map() })
    function landingShows(one) {
        const said = Words.of("agents:landed").said ?? {};
        const clock = at => Moment.clock(new Date(at));
        const when = {
            clock: clock,
            day: at => When.day(at, Date.now(), {
                yesterday: String(said.yesterday ?? ""),
                date: (date, format) => Qt.formatDate(date, format)
            })
        };
        return {
            side: Landed.sideOf(one, landedSides, String(Calendars.today ?? ""), when),
            // None until the daemon tells it, a moment after it's first
            // drawn, as a session's drawer comes.
            drawer: () => {
                const made = provider.landingDrawerOf(one);
                return made === null ? null : SessionDrawer.shown(made, {
                    clock: clock,
                    since: word("since"),
                    dated: at => `${Moment.day(new Date(at), Date.now())}, ${clock(at)}`
                });
            }
        };
    }
    // Each landing's row kept while the landing, where its Return goes and
    // the day read the same, so Landed's rows read again make none anew:
    // while Joel is in Landed, every change to Agents reads them twice, to
    // see what it shows and to remake the step (Provider.qml's says and
    // Launcher.qml's relive), which made his 1,683 landings anew each time
    // (LR3WMN, 2026-11-12). Kept by identity, since the daemon's record of
    // an unchanged landing stays the same object.
    readonly property var landedKeeping: keeper()
    // Landed's rows kept whole while what they're made from are the same
    // objects: the landings, the day, and the sessions running and parked,
    // which Sessions makes anew, each session with them, at any change.
    // Landed's rows are read again at each step through it, and each read
    // looked at all 2,600 landings, 27 to 39 ms, four times in To Try's
    // Return (2026-11-14). A plain object, written in place, so keeping
    // them notifies nothing.
    readonly property var landedWhole: ({ rows: null, of: [] })
    // A day's rows kept as Landed's are, apart, so stepping into one day
    // lets go of none of today's.
    readonly property var landedDayKeeping: keeper()
    // The dates whose places hold landings over a day old, newest first,
    // with how many each holds, as the daemon counts them (landed.rs's
    // earlier).
    readonly property var landedEarlier: {
        void landedMoves;
        return (rowsKind?.held ?? {})["landed-days:agents"]?.earlier ?? [];
    }
    // A day's row in Landed: its name the day as a side says one
    // ("Yesterday", "Tuesday", "17 Nov"), its side how many landed, and
    // its landings made rows only as Joel steps into it, which is when
    // the daemon tells them (landedDayIn).
    function landedDayName(day) {
        const named = When.day(new Date(`${day}T00:00:00`).getTime(), Date.now(), {
            yesterday: Words.name("agents:landed:yesterday"),
            date: (date, format) => Qt.formatDate(date, format)
        });
        return named === "" ? day : named;
    }
    function landedDay(each, today) {
        const words = Words.of("agents:landed").said ?? {};
        const day = String(each.day);
        return thingRow({
            key: `agents:landed-day:${day}`,
            name: landedDayName(day),
            side: String(words.count ?? "").replace("{count}", String(each.count)),
            glyph: Words.glyph("agents:landed"),
            hue: "action",
            // Stepped in, it's empty until the day's watch answers, as
            // Applications is while it reads.
            empty: "\u2026"
        }, () => {
            const kept = provider.landedKept(today, landedDayKeeping);
            provider.keepingBegun(landedDayKeeping);
            const rows = provider.landingsOn(day).map(kept);
            provider.keptOnly(landedDayKeeping);
            provider.tally.looked += rows.length;
            return rows;
        });
    }

    // What keeps a landing's row in Landed for the day `today` names: the
    // row kept while the landing, where its Return goes and the day are the
    // same, else made anew. Landed's rows read and the warm below both keep
    // them through it, so a row warmed is the row Landed shows.
    function landedKept(today, keeping = landedKeeping) {
        const read = one => provider.readLanding(one.key);
        const act = (id, fields) => thing(Object.assign({
            key: `verb:agents:landed:${id}`,
            verb: Words.verb(`verb:agents:landed:${id}`),
            name: Words.name(`verb:agents:landed:${id}`),
            glyph: Words.glyph(`verb:agents:landed:${id}`),
            hue: "action"
        }, fields));
        const windowed = one => {
            const session = provider.landerOf(one);
            return session !== null && Sessions.running.includes(session)
                && provider.windowedOf(session);
        };
        // The sessions as they stand while these rows are kept, found by
        // id and by address as landerOf finds them, the first of each
        // winning: each landing's way looked for its session among all of
        // Joel's, twice, then among those running, 2,600 times a build, so
        // each To Try Return, which reads a landing, spent 40 ms of its
        // 87 remaking Landed's ways (2026-11-15).
        const here = Sessions.running.concat(Sessions.parked);
        provider.tally.sought += here.length;
        const byId = new Map();
        const byAddress = new Map();
        here.forEach(session => {
            const id = session.presence?.session;
            if (!byId.has(id))
                byId.set(id, session);
            if (!byAddress.has(session.address))
                byAddress.set(session.address, session);
        });
        const running = new Set(Sessions.running);
        const windowedNow = one => {
            // By its address only for a landing that names no id, as
            // landerOf, so Go To shows only where its Return goes.
            const session = (one.session.id !== "" ? byId.get(one.session.id)
                : byAddress.get(one.session.address)) ?? null;
            return session !== null && running.has(session) && provider.windowedOf(session);
        };
        const toChange = one => () => Niri.spawn(["xdg-open", one.url]);
        const toSession = one => () => Sessions.goTo(provider.landerOf(one));
        // Where Return takes it: the change, else its session, else nowhere
        // but read, its session found as it stands now unless told how.
        const goes = (one, windowedBy = windowed) => one.url !== "" ? "change"
            : windowedBy(one) ? "session" : "";
        const parts = one => [
            one.url !== "" ? act("change", { later: true, run: toChange(one) }) : null,
            windowed(one) ? act("session", { later: true, run: toSession(one) }) : null,
            act("feedback", {
                ask: Words.of("verb:agents:landed:feedback").ask ?? "",
                chip: one.name ?? one.what,
                state: () => provider.fedBack,
                talks: () => provider.feedbackTalks(one.key),
                given: words => provider.feedback(one.key, words)
            }),
            one.person !== "" ? copyRow({
                key: "verb:agents:landed:try",
                name: Words.name("verb:agents:landed:try"),
                copy: one.person,
                figure: false,
                glyph: Words.glyph("verb:agents:landed:try")
            }) : null
        ].filter(row => row !== null);
        // Where it goes, `way`, is worked out once as it's kept, and again
        // only as its Return runs, when its session may have gone: each
        // working out looks for its session among all of Joel's, and a row
        // had worked it out five times.
        const landing = (one, way) => thing(Object.assign({
            thing: true,
            key: `agents:landed:${one.key}`,
            // What it asks Joel to do where it asks anything, else what
            // landed, as the daemon names it (landed.rs), the same here
            // and in For You.
            name: one.name ?? one.what,
            verb: Words.verb(`verb:agents:landed:${way || "read"}`),
            glyph: Words.glyph("agents:landed"),
            hue: "action",
            // Its drawer shown is it read (`landingDrawn`).
            readsLanding: one.key,
            later: way !== "",
            // Read where it goes nowhere, which keeps the launcher, as a
            // setting's change does, so the next landing is under the cursor.
            stays: way === "",
            // Landings going the same way say the same keys where they
            // aren't selected, so the foot weighs one of a run for all
            // (Pacing.toWeigh): weighing each of 2,600 took 270 to 355 ms
            // of every step into Landed, most of To Try's Return.
            keysAlike: `landing:${way}`,
            run: () => {
                if (goes(one) === "change")
                    toChange(one)();
                else if (goes(one) === "session")
                    toSession(one)();
                read(one);
            },
            verbs: () => parts(one)
        }, provider.landingShows(one)));
        return one => {
            const way = goes(one, windowedNow);
            return provider.keptAlike(keeping, one.key, [one, way, today],
                () => landing(one, way));
        };
    }

    // Made again as the day Joel is in is told, so its step is too, and as
    // the day turns, which renames the days before.
    readonly property var landed: readNow ? ownKept("landed", [Words.all, provider.weight,
        landedDayMoves, landedDays, Calendars.today, landings, landedEarlier], landedMade)
        : madeBefore("landed")
    function landedMade() {
        const words = Words.of("agents:landed");
        return thingRow({
            key: "agents:landed",
            name: Words.name("agents:landed"),
            side: Landed.stands(Number(landedDays[String(Calendars.today ?? "")] ?? 0),
                words.said ?? {}),
            // A place's drawer is drawn once it names what it holds, as a
            // Recent's is (Launcher.qml's drawable); with nothing landed,
            // it has nothing to say past its row.
            heldNames: landings.map(one => one.name ?? one.what)
                .concat(landedEarlier.map(each => landedDayName(String(each.day)))),
            drawer: landings.length + landedEarlier.length === 0 ? undefined : () => ({
                eyebrow: (Words.field.drawer ?? {}).place ?? "",
                title: Words.name("agents:landed"),
                body: (words.tells ?? {}).holding ?? ""
            }),
            glyph: Words.glyph("agents:landed"),
            hue: "action"
        }, () => {
            const today = String(Calendars.today ?? "");
            const of = [landings, today, Sessions.running, Sessions.parked, landedEarlier];
            if (landedWhole.rows !== null
                && of.every((part, at) => part === landedWhole.of[at]))
                return landedWhole.rows;
            const kept = provider.landedKept(today);
            provider.keepingBegun(landedKeeping);
            const rows = landings.map(kept)
                .concat(landedEarlier.map(each => provider.landedDay(each, today)));
            provider.keptOnly(landedKeeping);
            // Each landing looked at is counted, as Windows counts each window it
            // hears, for the check that Landed read again looks at none.
            provider.tally.looked += landings.length;
            landedWhole.rows = rows;
            landedWhole.of = of;
            return rows;
        });
    }

    // A landing whose drawer was out is read, as Return on it makes it
    // (VKCF3Z, 2026-11-10: it clears itself once he opens its drawer or
    // presses Return on it), wherever its row is, in Landed or For You.
    // It's read once the drawer leaves it, for another row's or as the
    // launcher closes, so the row he's on stays there for his Return,
    // which a row gone dimmed would not take.
    readonly property string landingDrawn: provider.launcher?.open === true
        && provider.launcher.drawerOut === true
        ? String(provider.launcher.drawerRow?.readsLanding ?? "") : ""
    property string landingShown: ""
    onLandingDrawnChanged: {
        if (landingShown !== "" && landingShown !== landingDrawn)
            provider.readLanding(landingShown);
        landingShown = landingDrawn;
    }

    // No carry stands in For You, though an orchestrator made it in Joel's
    // stead: it asks nothing of him, and five there read "A Session on Its
    // Way" hours after they went, naming nothing he could act on (Joel,
    // 2026-11-23 21:52: "how does it provide me with anything useful?").
    // Agents › Carried lists every carry of the day, each by its quest,
    // where it went and when, so he still sees each one (ISXEG8's fourth
    // condition). A refused or stuck one says so there in alert, and rings
    // nowhere else yet.
    // What Landed gives For You (ForYou.js's given): each landing that says
    // what to try and Joel hasn't read, waiting in To Try with its mark,
    // weighed as the rest are, and each read since, in its History with
    // when he read it. Return on either, as its drawer shown, steps into the
    // landing in Agents \u203a Landed, never a copy, and reads it, so one
    // landing is counted once, in For You, and goes from it once read
    // (VKCF3Z and Y2YBTS, 2026-11-10).
    // Each row kept while its landing, how it's read and the day read the
    // same, so a landing coming makes its own row alone (Provider.qml's
    // keptOr). The day is Calendars' hourly clock's, which turns at
    // midnight, so yesterday's landing stops reading as today's; a day
    // read here from Date would never make forYou again by itself
    // (Y2YBTS, 2026-11-10).
    readonly property var forYouKeeping: keeper()
    readonly property var forYou: {
        const said = Words.of("agents:landed").said ?? {};
        const today = String(Calendars.today ?? "");
        const opens = one => {
            provider.launcher?.goTo({ trail: provider.landingTrail(one), spot: "" });
            provider.readLanding(one.key);
        };
        // Its own Open leads its step, as a look's does, so the ↵ its drawer
        // marks is what Return does and the one Go To is the session's. It
        // keeps the launcher as the row does, since the row always stays
        // (Catalogue.js's closingFirst sets how a staying row's own act runs).
        const own = one => thing({
            key: `verb:agents:landed:open:${one.key}`,
            verb: Words.verb("verb:on:look:open"),
            name: Words.name("verb:on:look:open"),
            glyph: Words.glyph("verb:on:look:open"),
            hue: "action",
            run: () => opens(one)
        });
        const row = (one, fields) => thing(Object.assign({
            thing: true,
            name: one.name ?? one.what,
            // Its own Open's word, so that act leads its step (Catalogue.js's
            // leading) rather than the Go To of the session that landed it.
            verb: Words.verb("verb:on:look:open"),
            glyph: Theme.glyph.toTry,
            readsLanding: one.key,
            trail: provider.landingTrail(one),
            spot: "",
            stays: true,
            changes: false,
            // Its step's Open walks to Landed in the launcher too, so Return
            // there keeps it rather than closing it on nothing.
            alwaysStays: true,
            // Every one says open, so the foot weighs one of a run.
            keysAlike: "landing:for-you",
            // Asked as it's stepped into, since its session may have gone;
            // a read one's in History too, since who landed it is still
            // whom Joel would talk to of it.
            // Each act names the landing as the row does, so a drawer out on
            // the row that follows into its step doesn't read the landing and
            // take its row from To Try while Joel stands in it.
            verbs: () => [own(one)].concat(provider.landerActs(one))
                .map(act => Object.assign(act, { readsLanding: one.key })),
            run: () => opens(one)
        }, provider.landingShows(one), fields));
        const kept = (one, fields) => provider.keptOr(forYouKeeping, fields.key,
            JSON.stringify([one, fields.hue, today]), () => row(one, fields));
        provider.keepingBegun(forYouKeeping);
        const given = tries.filter(tried => tried.read === null).map(tried => tried.one).map(one => ({
            row: kept(one, {
                key: `on:waiting:try:${one.key}`,
                hue: "action",
                forYou: { kind: "try", source: "sessions" }
            }),
            thing: { kind: "try", ready: true, since: one.at }
        })).concat(tries.filter(tried => typeof tried.read === "number").map(tried => ({
            row: kept(tried.one, { key: `on:waiting:tried:${tried.one.key}`, hue: "muted" }),
            done: { how: String(said.read ?? ""), at: tried.read }
        })));
        provider.keptOnly(forYouKeeping);
        return given;
    }

    // Agents' Recent: the sessions that closed this boot, newest first, as
    // the agents daemon serves them ready (modules/agents/src/recent.rs),
    // never one Agents already shows, a closed orchestrator at the tree's
    // end, one of Sessions to Reopen or one running again. The last boot's
    // are Sessions to Reopen's alone. It always stands, after Landed and
    // before Start, its names in its drawer and how many at its right, and
    // Return on one reopens it where it left off, by its id (VKCF3Z,
    // 2026-11-08). Made again only as the daemon says its records moved,
    // or the day turns, which rewords when each closed: Sessions had made
    // every closed session anew as any one changed, and compared nine of
    // Joel's, about 30,000 characters, each time (2026-11-10).
    // What it holds changes in place, which recentMoves counts; rowsKind's
    // held is a part too since a reload of rowsLink gives another.
    readonly property var recent: readNow ? ownKept("recent", [Words.all, provider.weight,
        recentMoves, Calendars.today, rowsKind?.held], recentMade) : madeBefore("recent")
    function recentMade() {
        const held = Object.values(rowsKind?.held ?? {})
            .filter(one => one?.kind === "recent" && one.recent);
        const listed = Order.sorted(held, Order.smaller(one => one.order))
            .map(one => ({ row: one.recent }));
        const words = Words.of("agents:recent");
        // What its place says of them comes worded by the agents daemon
        // (recent.rs's place): how many, and their names once each.
        const place = rowsKind?.held?.["recent:agents"]?.place ?? {};
        const said = Words.of("agents").said ?? {};
        const now = new Date();
        const home = Sessions.home;
        const homely = folder => folder === home || folder.startsWith(`${home}/`)
            ? `~${folder.slice(home.length)}` : folder;
        // Here, never where a boot kept it (reopenThemAll), so it opens
        // alone in the folder its record ran in, as one of Sessions to
        // Reopen does.
        const reopen = one => Sessions.reopen(Object.assign({}, one,
            { machine: undefined, cwd: String(one.cwd ?? "") }),
            cwd => provider.launcher?.tellOnOpen([[homely(cwd), String(
                (Words.of("verb:agents:to-reopen:one").said ?? {}).isGone ?? ""), Theme.alert]]));
        const row = one => thing({
            thing: true,
            key: `agents:recent:${one.id}`,
            name: String(one.name ?? ""),
            also: Fuzzy.prepare(`${one.address ?? ""} ${one.folder ?? ""} ${one.quest ?? ""}`),
            side: SessionRow.closedAt(one.ended, now, said,
                date => Qt.formatDate(date, "ddd")),
            note: String(one.note ?? ""),
            verb: Words.verb("verb:agents:to-reopen:one"),
            glyph: Theme.glyph.agent,
            hue: SessionRow.hue("off"),
            later: true,
            run: () => reopen(one)
        });
        return Object.assign(thingRow({
            key: "agents:recent",
            name: Words.name("agents:recent"),
            glyph: Words.glyph("agents:recent"),
            hue: "program",
            side: String(place.side ?? ""),
            heldNames: place.held ?? [],
            ask: "which?",
            empty: words.empty
        }, () => listed.map(one => row(one.row))), {
            stays: true
        });
    }

    // What each budget the sessions draw on stands at, the last of its
    // parts, beside AI Spend, wherever the tree is and however many
    // sessions it holds, since a place's rows keep their places (VKCF3Z,
    // 2026-11-08): a thing of Agents the vocabulary names as its part
    // after AI spend (words.json), so the launcher lists it there. It is
    // AI spend's own Budgets row (Spend.qml), so its order, its words and
    // its drawers are the ones AI spend shows, never worked out twice. A
    // step in, All AI Spend in billed dollars first, then each budget
    // nearest its limit first, as spend-watch orders them, a budget with
    // windows stepping in to a row for each, and every one opening the
    // drawer AI spend's opens (SpendDrawer.js). Its glyph is AI spend's
    // row's, wearing the nearest budget's warning, so it reads as the
    // same thing rather than as a session. None where AI spend
    // lists no budgets: with one budget, which its periods already say,
    // with no spend-watch, or while the screen is shared, when AI spend
    // lists only what holds no amount (KP0L6A, 2026-11-08 [f566a31a]).
    readonly property var budgets: readNow ? budgetsKept() : madeBefore("budgets")
    // Kept while AI spend's row is the one it was made of: a row it
    // serves is made again only once its record changes (ReadyRows.qml's
    // rowAt and took), so the same row says the same, and a - out of Now
    // made it again for nothing (core.nix#6037's follow-up).
    function budgetsKept() {
        // AI spend's row as it reads now, since AI spend makes its rows
        // only while it's wanted, which it isn't while Agents is held;
        // found among them where it can't say (1KZFHC, 2026-11-10). Its
        // words and weight are the vocabulary's and the settings', as the
        // other rows kept here are.
        const spend = source("spend");
        const now = spend?.rowNow("spend:budgets");
        const spent = now !== undefined ? now : (spend?.everything() ?? [])
            .find(row => row.key === "spend:budgets") ?? null;
        return ownKept("budgets", [Words.all, provider.weight, spent], () => budgetsOf(spent));
    }
    function budgetsOf(spent) {
        if (spent === null)
            return null;
        return Object.assign(thingRow({
            key: "agents:budgets",
            name: Words.name("agents:budgets"),
            side: spent.side,
            glyph: spent.glyph,
            hue: spent.hue
        }, spent.verbs), {
            ask: "which?"
        });
    }

    // Spec after Budgets, since it's a reading of the agents as Budgets is
    // (VKCF3Z, 2026-11-14 [efa2d961]): how they deliver against the bars
    // Joel set, its side the head cadence-spec serves, "14 readings out of spec",
    // wearing the daemon's glyph, and a step in its axes, each stepping in
    // to its readings (modules/cadence/Spec.qml). Agents' own row says
    // nothing of it, so it never competes with "needs you". None where
    // cadence-spec hasn't answered or isn't on this machine. Listed as its
    // part, as Budgets is, and never among Agents' own rows as well, where
    // its key twice over moved the cursor between them (2026-11-18).
    readonly property var spec: readNow ? specKept() : madeBefore("spec")
    // Kept while the head says the same. Spec's head() is a new object each
    // time it's asked, so what it says is compared rather than the object.
    function specKept() {
        const place = source("spec");
        const head = place?.head() ?? null;
        const says = head === null ? [null] : [String(head.said ?? ""), head.glyph, head.hue];
        return ownKept("spec", [Words.all, provider.weight, place].concat(says),
            () => specOf(place, head));
    }
    function specOf(place, head) {
        if (head === null)
            return null;
        return thingRow({
            key: "agents:spec",
            name: Words.name("agents:spec"),
            side: String(head.said ?? ""),
            glyph: head.glyph || Words.glyph("agents:spec"),
            hue: head.hue || "agent"
        }, () => place.axes());
    }

    things: [toReopen, landed, budgets, spec, recent, start, startNewWork]
        .filter(row => row !== null)

    // Asked while they aren't wanted, as a step into Agents asks before it's
    // held, its sessions' rows are laid out as asked, from the daemon's rows
    // held; its own after them only while they're read (readNow), and
    // otherwise as last made, so a step from a place that doesn't read
    // them, as a category Agents is in, shows them made again once held.
    readsAfreshUnwanted: readNow
    function everything() {
        // Its search looks through the tree's rows, so their texts are
        // made ready from those (Provider.qml's looksThrough).
        const tree = provider.wanted || !opened() ? treeRows() : laidOut();
        const sessions = topServed && opened() && !provider.lookingThrough.now ? top.rows() : tree;
        return sessions.concat([toReopen, landed, recent, start, startNewWork]
            .filter(row => row !== null));
    }

    // How they stand together, "1 needs you \u00b7 3 working \u00b7 $214
    // today", as the daemon says it, the day's dollars none while the
    // screen is shared; held while the launcher is shut, as Home's row is
    // read.
    function summary() {
        void rowsKind?.revision;
        return String(rowsKind?.held?.[summaryKey]?.summary?.said ?? "");
    }

    // How many lines it has read of the agents daemon, for the check that
    // the shut shell hears its rows only once a pace. Read only when
    // asked, so nothing wakes for it.
    function linesHeard(): int {
        return rowsKind?.linesHeard ?? 0;
    }

    // How its sessions stand together, at its row's right wherever it
    // stands, home included, while its sessions are named in its drawer
    // as every place's are (VKCF3Z, 2a429f67, dae7fb67). Found by its name,
    // "agents", its row is the answer, as
    // Processor's is for "cpu", and Return still opens it, since that's
    // what Joel types its name for.
    function counted() {
        return summary();
    }
    // The found row and Home's are said again as it moves, since the day's
    // dollars and sharing can change it while no session's row changes,
    // and the launcher joins what a place says only when it says it
    // changed (Y2YBTS, 2026-11-07).
    readonly property string standing: summary()
    onStandingChanged: changed()
    // With no session to say, its right says so, "none running", rather
    // than standing bare, since Agents holds sessions (VKCF3Z, 00ca9231).
    function homeSays() {
        const said = summary();
        return said !== "" ? said : collection.none ?? "";
    }
    saysAtHome: true
}
