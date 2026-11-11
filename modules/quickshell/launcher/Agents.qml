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
// (SessionRow.js): those needing Joel, then those working, in line,
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
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "../PiSource.js" as Pi
import "../Places.js" as Places
import "../When.js" as When
import "Landed.js" as Landed
import "Moment.js" as Moment
import "Numbers.js" as Numbers
import "Order.js" as Order
import "Recent.js" as Recent
import "SessionDrawer.js" as SessionDrawer
import "SessionRow.js" as SessionRow
import "SpendDrawer.js" as SpendDrawer

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
    readonly property var all: opened() && provider.wanted ? laidOut() : []

    function opened() {
        const launcher = provider.launcher;
        return !!launcher && (launcher.open || launcher.cataloguing);
    }

    // Agents' own rows after its sessions' (toReopen, landed, recent, start
    // and budgets), as each was last made while anything could read it:
    // the launcher open, or its catalogue walked. Shut, nothing does, so
    // each gives what it last made and reads nothing else, and a change
    // nobody sees makes none of them; each is made again as the launcher
    // opens, three or four rows (keeps-pace). Joel's shut launcher made 312
    // of Agents' rows in 240 s (YQZGPC, 2026-11-10). For You's (forYou)
    // are made shut too, since the tray counts what waits there, but only
    // as a landing comes, goes or changes. A plain object, so keeping one
    // notifies nothing.
    readonly property var lastMade: ({})
    function madeBefore(name, none) {
        return name in lastMade ? lastMade[name] : none;
    }
    onToReopenChanged: lastMade.toReopen = toReopen
    onLandedChanged: lastMade.landed = landed
    onRecentChanged: lastMade.recent = recent
    onStartChanged: lastMade.start = start
    onBudgetsChanged: lastMade.budgets = budgets

    // The daemon's rows of Agents, each record held by its key, its fields
    // under `row` (Y2YBTS's ready-rows.md), watched whether or not the
    // launcher's open, so it opens on rows already held.
    readonly property var rowsKind: rowsLink.item
    LazyLoader {
        id: rowsLink
        active: !!Quickshell.env("XDG_RUNTIME_DIR")

        Daemon {
            socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/agents.sock`
            kind: ["row", "summary"]
            watching: ({ place: "agents" })
            ahead: provider.drawerWanted !== "" ? [provider.drawerWanted] : []
            answersWhole: true
            onHeard: (changed, gone) => provider.placed(changed, gone)
            // Told as it answers, and again as sharing changes, whether the
            // screen is shared, which only the shell knows from niri's
            // casts, so the daemon says no amount while it is.
            onStateChanged: provider.tellSharing()
        }
    }
    readonly property string summaryKey: "summary:agents"

    function tellSharing() {
        if (rowsKind?.state === "answering")
            rowsKind.ask({ ask: "sharing", sharing: Quiet.sharing });
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
    property int placings: 0

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
        if (!moved.some(rowKey))
            return;
        for (const key of moved) {
            delete wrapped[key];
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
    }

    // Every row at the tree's top, in the daemon's order, a fold holding
    // its members, then the closed orchestrators.
    function laidOut() {
        void placings;
        const held = rowsKind?.held ?? {};
        const rows = placing.keys.filter(key => !held[key]?.row?.held?.fold)
            .map(key => rowOf(key, held)).filter(row => row !== null);
        const orchestrators = source("orchestrators");
        return rows.concat((orchestrators?.closed ?? [])
            .map(one => closedRow(one, orchestrators)));
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
            const row = thingRow({
                key: served.row,
                name: served.name,
                glyph: Theme.glyph[served.glyph] ?? Theme.glyph.agent,
                hue: served.hue,
                copiesNothing: true,
                depth: served.depth
            }, () => members);
            row.held = members;
            return row;
        }
        if (!(key in wrapped)) {
            const session = sessionAt(String(served.row).replace(/^agent:/, ""));
            if (session === null)
                return null;
            wrapped[key] = sessionRow(session, served);
        }
        return wrapped[key];
    }

    // The names its row says after its own, its rows' names in their
    // order, read from the daemon's rows, never from what's made of them,
    // so Home names them without laying any out.
    function memberNames() {
        void placings;
        const held = rowsKind?.held ?? {};
        const closed = source("orchestrators")?.closed ?? [];
        return placing.keys.filter(key => !held[key]?.row?.held?.fold)
            .map(key => held[key].row.name)
            .concat(closed.map(one => closedName(one)),
                [toReopen, landed, recent].filter(row => row !== null).map(row => row.name));
    }

    // A closed orchestrator's name: what the harness calls it, short first.
    function closedName(one) {
        return one.short || one.title || one.address;
    }

    // A closed orchestrator's row: its glyph muted, as one paused is, its
    // name in text, "closed" at its right with what its whole orchestration
    // spent today when that's anything, its quest's title as its note, and
    // Return reopening it where it left off. One whose folder is gone says
    // so in alert, and starts afresh in its quest's start folder
    // (Orchestrators.qml's goneFrom), or, with none, steps in to its talks.
    function closedRow(one, orchestrators) {
        const family = one.quest && !Quiet.sharing ? SpendWatch.families[one.quest] ?? null
            : null;
        const items = SessionRow.side(word("closed"),
            family ? word("allOf", { spent: family.all }) : "");
        const gone = orchestrators.goneFrom(one);
        const row = member({
            thing: true,
            key: `orchestrator:${one.address}`,
            kind: SessionRow.KIND,
            name: closedName(one),
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
    // nothing. Wanted again, it says whatever moved meanwhile.
    readonly property var everyRow: provider.wanted ? everything() : null
    onEveryRowChanged: {
        if (everyRow !== null)
            changedTo(everyRow);
    }

    // Every session's row, those folded too, as typing searches them.
    function sessionRows(rows) {
        return rows.reduce((every, row) => every.concat(row.held ? sessionRows(row.held)
            : [row]), []);
    }

    // How a session stands (SessionRow.js), from its presence, whether its
    // want is truly Joel's and whether it waits its turn in line.
    function stateOf(session) {
        return SessionRow.state(session.presence, Sessions.asks(session),
            inLine(session.presence) !== "");
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

    // A session elsewhere that couldn't be joined, a hosted one with no
    // window to go to, or any the node couldn't tell, is said in the foot,
    // by its address in the alert colour, where the act was pressed: at
    // once while the launcher's open, or else once it opens next, never as
    // a notice (VKCF3Z, 2026-11-07 [4a00987e]).
    Connections {
        target: Sessions

        function onUnjoined(session) {
            provider.launcher?.tellOnOpen([[session.address,
                provider.elsewhere(session.reachable ? "unheld" : "unreached", session),
                Theme.alert]]);
        }

        function onUnhosted(session) {
            provider.launcher?.tellOnOpen([[session.address,
                provider.elsewhere("hosted", session), Theme.alert]]);
        }

        function onUnsteered(session, why) {
            provider.launcher?.tellOnOpen([[session.address,
                provider.elsewhere(why !== "" ? "untold" : "unanswered", session, why),
                Theme.alert]]);
        }

        function onToReopenChanged() {
            provider.lookForFolders();
        }
    }

    // Where a session's first job waiting for its turn stands in line.
    function inLine(presence) {
        const waiting = presence && presence.session
            ? Jobs.of(presence.session).find(job => job.state === "waiting") : undefined;
        return waiting ? word("inLine", { place: Numbers.ordinal(waiting.place) }) : "";
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
            hue: served.hue,
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
            drawer: () => provider.drawerOf(session.address)
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

    // A session's row at no depth, as Now and Ask show one, or none while
    // the daemon hasn't said it.
    function made(session) {
        void placings;
        const row = rowOf(`row:agents:${session.address}`, rowsKind?.held ?? {});
        return row === null ? null : Object.assign({}, row, { depth: 0 });
    }

    // A session's drawer as the agents daemon made it ready
    // (modules/agents/src/drawer.rs), what matters first: watched by its
    // key only while it's shown, since its figures move every turn, so the
    // drawer names its key just after it first asks, and fills as the
    // daemon answers. Only its times are worded here, by the clock
    // (SessionDrawer.js), so one left open never waits on the daemon to
    // turn a minute.
    property string drawerWanted: ""

    function drawerOf(address) {
        const key = `drawer:agents:${address}`;
        if (key !== drawerWanted)
            Qt.callLater(() => {
                provider.drawerWanted = key;
            });
        void rowsKind?.revision;
        const made = rowsKind?.held?.[key]?.drawer ?? null;
        return made === null ? null : SessionDrawer.shown(made, {
            clock: at => started(at),
            since: word("since")
        });
    }

    // What the foot says once words have gone to a session.
    property string told: ""

    // A session's own acts, in the one order every session keeps
    // (SessionRow.js), each steering it through its inbox but Go To; an
    // orchestrator's row is its session's, with its talks after them.
    function sessionActs(session) {
        return SessionRow.acts(stateOf(session), windowedOf(session), session.here)
            .map(id => sessionAct(session, id));
    }

    // What Close leaves of a session: its worktree, where it works in a
    // folder under a trees folder, as the harness's and World's trees both
    // are cut, else its folder, and its quest's to-dos, which stay
    // until each is done.
    function leaves(session) {
        const presence = session.presence ?? {};
        const cwd = String(presence.cwd ?? "");
        const inTree = cwd.includes("/trees/");
        const quest = presence.quest ?? presence.questInfo?.id ?? "";
        const needs = quest === "" ? 0
            : WaitingOnYou.items.filter(item => item.quest === quest).length;
        return SessionRow.leaves(inTree, needs, Words.of("agents").said ?? {});
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
            hue: SessionRow.hue("listening", full !== "")
        }, session.title, "wake", {
            done: () => !Sessions.parked.some(one => String(one.presence?.session ?? "") === id),
            within: Sessions.startsWithin
        });
        Sessions.wake(session);
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
        }, one.short || one.title || one.address, "reopen", {
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
        const steered = what => () => Sessions.steer(session.address, { act: what });
        const made = {
            switch: () => act("switch", { later: true, run: () => Sessions.goTo(current()) }),
            talk: () => act("talk", {
                ask: Words.of("verb:agent:talk").ask ?? "",
                chip: session.title,
                state: () => provider.told,
                given: words => {
                    if (Sessions.talk(session.address, words))
                        provider.told = (Words.of("orchestrators").said ?? {}).sent
                            ?.replace("{name}", session.title) ?? "";
                }
            }),
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
                leaves: leaves(session), askedAs: session.address },
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
        for (const row of sessionRows(all)) {
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

    readonly property var start: {
        if (!opened())
            return madeBefore("start", null);
        const quests = () => (source("quests")?.everything() ?? [])
            .filter(row => row.unfinished && row.verb !== "go to");
        return Object.assign(thingRow({
            key: "agents:start",
            name: Words.name("agents:start"),
            glyph: Theme.glyph.agent,
            hue: "action"
        }, quests), {
            folded: true
        });
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
    readonly property var toReopen: {
        if (!opened())
            return madeBefore("toReopen", null);
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
        // Where each came from, and where they all did: the last boot, the
        // machine that carried it or pi's host that gave it back.
        const fromOf = one => Places.wayOf(said, one);
        const note = Places.reopenNote(said, Sessions.lastBoot.length, Sessions.apart);
        // One given back names its host in its drawer alone, last, as an
        // identifier, since the host's name is no place Joel chose
        // (VKCF3Z, [92a1d30c]).
        const hostOf = one => Places.apartOf(one)?.kind !== "released" ? {} : {
            drawer: () => ({
                eyebrow: fromOf(one),
                title: String(one.title || one.address || one.id),
                facts: [{ label: Words.name("agents:host"), value: Places.apartOf(one).from,
                    identifies: true }]
            })
        };
        const home = Sessions.home;
        const homely = folder => folder === home || folder.startsWith(`${home}/`)
            ? `~${folder.slice(home.length)}` : folder;
        // In its own folder only, never the home folder instead. One whose
        // folder is gone says so on its row, looked for as the launcher
        // opens (lookForFolders), and holds no Reopen (VKCF3Z, 2026-11-07
        // [4a00987e]); one whose folder went since it was listed is said
        // once the launcher opens next, with how the rest went, never as a
        // notice.
        const oneSaid = Words.of("verb:agents:to-reopen:one").said ?? {};
        const goneHere = one => one.machine === undefined && Sessions.folderGone(one.cwd);
        const reopen = one => Sessions.reopen(one, cwd => provider.launcher?.tellOnOpen([
            [homely(cwd), String(oneSaid.isGone ?? ""), Theme.alert]]));
        const goneCount = count => count === 1 ? String(said.goneOne ?? "")
            : String(said.goneMany ?? "").replace("{count}", count);
        const forget = one => act("forget", { run: () => Sessions.forgetReopen(one) });
        const unreopened = one => thingRow(Object.assign({
            key: `agents:to-reopen:${one.id}`,
            name: String(one.title || one.address || one.id),
            side: String(oneSaid.gone ?? "").replace("{cwd}", homely(one.cwd)),
            alerts: true,
            note: fromOf(one),
            glyph: Theme.glyph.agent,
            hue: "alert"
        }, hostOf(one)), () => [forget(one)]);
        const closed = one => goneHere(one) ? unreopened(one) : thing(Object.assign({
            thing: true,
            key: `agents:to-reopen:${one.id}`,
            name: String(one.title || one.address || one.id),
            // One held elsewhere says where, as a session there does.
            side: one.machine !== undefined ? provider.elsewhere("on", one) : homely(one.cwd),
            note: fromOf(one),
            verb: Words.verb("verb:agents:to-reopen:one"),
            glyph: Theme.glyph.agent,
            hue: "program",
            later: true,
            run: () => reopen(one)
        }, hostOf(one)));
        const reopenable = lost.filter(one => !goneHere(one));
        const goneNow = lost.length - reopenable.length;
        // Reopen Them All reopens those that can be, its side counting them
        // when any can't: "58 of 60 \u00b7 2 folders gone". Each found gone
        // after is counted with how many reopened, and said once the
        // launcher opens next.
        const outcome = { asked: reopenable.length, gone: 0 };
        const tellAll = () => provider.launcher?.tellOnOpen([
            [String(outcome.gone), String(said[outcome.gone === 1 ? "folderGone"
                : "foldersGone"] ?? ""), Theme.alert],
            [String(outcome.asked - outcome.gone), String(said.reopened ?? "")]]);
        // Its count of what failed is drawn in the alert's colour, as the
        // row that failed is, the rest as a side reads (VKCF3Z, 2026-11-07).
        const there = String(said.there ?? "").replace("{there}", reopenable.length)
            .replace("{count}", lost.length);
        const themAll = () => act("all", {
            later: true,
            side: goneNow === 0 ? "" : `${there} \u00b7 ${goneCount(goneNow)}`,
            sideItems: goneNow === 0 ? [] : [{ said: there, keep: 2 },
                { said: goneCount(goneNow), keep: 3, hue: "alert" }],
            // Each back where it sat (Sessions.qml's reopenThemAll), a
            // carried one beside where Joel is, since it never sat here,
            // and then the windows that reopen themselves. Each is told to
            // carry on as it opens, so none waits to be poked; one reopened
            // alone from its own row isn't, since Joel is there.
            run: () => Sessions.reopenThemAll(reopenable, () => {
                outcome.gone += 1;
                tellAll();
            })
        });
        return thingRow({
            key: "agents:to-reopen",
            name: Words.name("agents:to-reopen"),
            side: String(said[lost.length === 1 ? "one" : "many"] ?? "")
                .replace("{count}", lost.length),
            note: note,
            glyph: Words.glyph("agents:to-reopen"),
            hue: "action",
            // For You's filter knows it as a session's, to be done.
            forYou: { kind: "do", source: "sessions" }
        }, () => (reopenable.length === 0 ? [] : [themAll()]).concat(lost.map(closed),
            Sessions.lastBoot.length === 0 ? [] : [act("notNow", {
            run: () => Sessions.putOffLastBoot()
        })]));
    }

    // Whether each folder there is to reopen in is there, looked for as the
    // launcher opens and as the sessions to reopen change while it's open,
    // so each row says it before Return rather than trusting what the
    // shell found at its start.
    function lookForFolders() {
        if (provider.launcher?.open)
            Sessions.lookForFolders(Sessions.toReopen.filter(one => one.machine === undefined)
                .map(one => one.cwd));
    }
    Connections {
        target: provider.launcher

        function onOpenChanged() {
            provider.lookForFolders();
            // Shut, no drawer shows, so none is watched.
            if (!provider.launcher.open)
                provider.drawerWanted = "";
        }
    }

    // What sessions landed (Landed.js's all): Landed, one row always, after
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
    // You's History, so nothing read is lost. What landed is kept 30 days
    // past its session's end (the
    // harness's landings.json, served as the node's `landing` kind), so a
    // session closed leaves what it landed in Landed. Its row says at its
    // right how many landed today, never how many are to read, which For
    // You alone counts (`forYou`), and its drawer, as a place's, what it
    // holds (VKCF3Z, adba1297).
    readonly property var keptLandings: {
        void landingKind?.revision;
        return Object.values(landingKind?.held ?? {});
    }
    readonly property var landingKind: landingLink.item
    LazyLoader {
        id: landingLink
        active: Sessions.nodePath !== ""

        Daemon {
            socket: Sessions.nodePath
            kind: "landing"
            changesDiffer: true
            answersWhole: true
        }
    }
    // What Landed.all reads of each session, its id, address, title and
    // own landings, said as text, so what landed is worked out again, and
    // what's made of it made again, only as that text or a kept landing
    // changes, never as the rest of a session's record moves. Worked out
    // from the records themselves, Landed's row was made again at every
    // session's change (the net's vm-desktop-launcher-keeps-up), and For
    // You's rows for every landing, hundreds at a time with the launcher
    // shut (PT5P13, 2026-11-10). The whole of what landed said as text was
    // over six times this, and took twice Landed.all's time at each change
    // (PT5P13, 2026-11-10).
    readonly property string landersSaid: JSON.stringify(Sessions.running
        .concat(Sessions.parked)
        .map(session => ({
            address: String(session.address ?? ""),
            title: String(session.title ?? ""),
            presence: {
                session: session.presence?.session,
                landings: session.presence?.landings ?? []
            }
        })))
    // A landing's session is said by its address, title and id, and one
    // still here is found again as it's acted on (landerOf).
    readonly property var landings: Landed.all(keptLandings, JSON.parse(landersSaid))
        .map(one => Object.assign({}, one, {
            session: {
                address: one.session.address,
                title: one.session.title,
                id: String(one.session.presence?.session ?? "")
            }
        }))
    // The session here that landed `one`, by its id, else by its address,
    // or null once it has ended.
    function landerOf(one) {
        const here = Sessions.running.concat(Sessions.parked);
        return here.find(session => one.session.id !== ""
            && session.presence?.session === one.session.id)
            ?? here.find(session => session.address === one.session.address) ?? null;
    }
    function readLanding(key) {
        if (Sessions.landingsRead.includes(key))
            return;
        Sessions.readLandings([key], Landed.stillRead(keptLandings,
            Sessions.running.concat(Sessions.parked), Sessions.landingsRead));
    }
    // Where a landing lives, Agents \u203a Landed \u203a it, which For You's rows
    // step into rather than show a copy of (VKCF3Z, 2026-11-10).
    function landingTrail(one) {
        return ["collection:agents", "agents:landed", `agents:landed:${one.key}`];
    }
    // A landing as both its rows show it, in Landed and in For You, one
    // component the same in both: when it landed at its side, and its
    // drawer (Landed.js's landedWhen and landingDrawer). Its side is kept
    // by its moment for the day Calendars' hourly clock names
    // (landedSides), and the whole moment said only as its drawer is
    // drawn, since saying both for over a thousand landings at every
    // opening was most of the date work pegging the shell after one
    // (H6EDLC's profile, 2026-11-10). A stopgap until the agents daemon
    // serves Landed's rows ready, KP0L6A's, on 2026-11-12.
    readonly property var landedSides: ({ day: "", sides: new Map() })
    function landingShows(one) {
        const said = Words.of("agents:landed").said ?? {};
        const when = {
            clock: at => Moment.clock(new Date(at)),
            day: at => When.day(at, Date.now(), {
                yesterday: String(said.yesterday ?? ""),
                date: (date, format) => Qt.formatDate(date, format)
            }),
            date: at => Moment.day(new Date(at), Date.now())
        };
        return {
            side: Landed.landedSide(landedSides, one.at, String(Calendars.today ?? ""), when),
            drawer: () => Landed.landingDrawer(one, {
                kind: String(said.kind ?? ""),
                when: Landed.landedWhen(one.at, when).fact,
                session: one.session.title ?? "",
                named: id => Words.name(`agents:${id}`)
            })
        };
    }
    readonly property var landed: {
        if (!opened())
            return madeBefore("landed", null);
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
        const toChange = one => () => Niri.spawn(["xdg-open", one.url]);
        const toSession = one => () => Sessions.goTo(provider.landerOf(one));
        // Where Return takes it: the change, else its session, else nowhere
        // but read.
        const goes = one => one.url !== "" ? "change" : windowed(one) ? "session" : "";
        const parts = one => [
            one.url !== "" ? act("change", { later: true, run: toChange(one) }) : null,
            windowed(one) ? act("session", { later: true, run: toSession(one) }) : null,
            one.person !== "" ? copyRow({
                key: "verb:agents:landed:try",
                name: Words.name("verb:agents:landed:try"),
                copy: one.person,
                figure: false,
                glyph: Words.glyph("verb:agents:landed:try")
            }) : null
        ].filter(row => row !== null);
        const landing = one => thing(Object.assign({
            thing: true,
            key: `agents:landed:${one.key}`,
            name: one.what,
            verb: Words.verb(`verb:agents:landed:${goes(one) || "read"}`),
            glyph: Words.glyph("agents:landed"),
            hue: "action",
            // Its drawer shown is it read (`landingDrawn`).
            readsLanding: one.key,
            later: goes(one) !== "",
            // Read where it goes nowhere, which keeps the launcher, as a
            // setting's change does, so the next landing is under the cursor.
            stays: goes(one) === "",
            run: () => {
                if (goes(one) === "change")
                    toChange(one)();
                else if (goes(one) === "session")
                    toSession(one)();
                read(one);
            },
            verbs: () => parts(one)
        }, provider.landingShows(one)));
        const words = Words.of("agents:landed");
        return thingRow({
            key: "agents:landed",
            name: Words.name("agents:landed"),
            side: Landed.stands(Landed.landedToday(keptLandings, Date.now()), words.said ?? {}),
            // A place's drawer is drawn once it names what it holds, as a
            // Recent's is (Launcher.qml's drawable); with nothing landed,
            // it has nothing to say past its row.
            heldNames: landings.map(one => one.what),
            drawer: landings.length === 0 ? undefined : () => ({
                eyebrow: (Words.field.drawer ?? {}).place ?? "",
                title: Words.name("agents:landed"),
                body: (words.tells ?? {}).holding ?? ""
            }),
            glyph: Words.glyph("agents:landed"),
            hue: "action"
        }, () => landings.map(landing));
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
        const kept = (one, fields) => provider.keptOr(forYouKeeping, fields.key,
            JSON.stringify([one, fields.hue, today]), () => row(one, fields));
        const row = (one, fields) => thing(Object.assign({
            thing: true,
            name: one.what,
            verb: "go to",
            glyph: Theme.glyph.toTry,
            readsLanding: one.key,
            trail: provider.landingTrail(one),
            spot: "",
            stays: true,
            changes: false,
            run: () => {
                provider.launcher?.goTo({ trail: provider.landingTrail(one), spot: "" });
                provider.readLanding(one.key);
            }
        }, provider.landingShows(one), fields));
        provider.keepingBegun(forYouKeeping);
        const given = Landed.toTry(landings, Sessions.landingsRead).map(one => ({
            row: kept(one, {
                key: `on:waiting:try:${one.key}`,
                hue: "action",
                forYou: { kind: "try", source: "sessions" }
            }),
            thing: { kind: "try", ready: true, since: one.at }
        })).concat(Landed.triedRead(landings, Sessions.landingsReadAt).map(tried => ({
            row: kept(tried.one, { key: `on:waiting:tried:${tried.one.key}`, hue: "muted" }),
            done: { how: String(said.read ?? ""), at: tried.at }
        })));
        provider.keptOnly(forYouKeeping);
        return given;
    }

    // Agents' Recent: the sessions that closed this boot, as every Recent
    // is laid out (Recent.js), newest first, each once by its id, and
    // never one Agents already shows, a closed orchestrator at the tree's
    // end, one of Sessions to Reopen or one running again
    // (SessionRow.js's shownElsewhere). The last boot's are Sessions to
    // Reopen's alone. It always stands, after Landed and before Start, its
    // names in its drawer and how many at its right, and Return on one
    // reopens it where it left off, by its id (VKCF3Z, 2026-11-08).
    // Made again only when what it lists reads otherwise, counted by a
    // number, which tells its readers only when it changes: Sessions makes
    // every session anew as any one changes, and a new Recent each time was
    // a change to Agents' things, which has the launcher walk again where
    // every row lives (Launcher.qml's stale). What it lists is kept beside
    // it in recentListed, a plain object that's never reassigned, so
    // writing its fields notifies nothing and can't loop; recent reads them
    // only once this number has said they changed. The rows are compared
    // in place rather than as text, since nine of Joel's closed sessions'
    // text, about 30,000 characters, started a garbage collection every
    // second change with the launcher open (2026-11-10).
    readonly property int recentFrom: {
        if (!opened())
            return recentListed.made;
        const closed = Sessions.closed;
        const live = Sessions.running.concat(Sessions.parked);
        const addresses = (source("orchestrators")?.closed ?? []).map(one => one.address)
            .concat(live.map(one => one.address));
        const quests = live.map(one => one.presence?.questInfo?.id ?? one.presence?.quest ?? "");
        const listed = Recent.listed([SessionRow.closedRecent(closed)],
            SessionRow.shownElsewhere(closed, Sessions.toReopen, addresses, quests));
        const rows = listed.map(one => one.row);
        const day = new Date().toDateString();
        recentListed.rows = listed;
        if (recentListed.made > 0 && recentListed.day === day
            && Pi.alike(recentListed.said, rows))
            return recentListed.made;
        recentListed.said = rows;
        recentListed.day = day;
        recentListed.made += 1;
        return recentListed.made;
    }
    readonly property var recentListed: ({ rows: [], said: [], day: "", made: 0 })
    readonly property var recent: {
        if (!opened())
            return madeBefore("recent", null);
        const listed = recentFrom === 0 ? [] : recentListed.rows;
        const words = Words.of("agents:recent");
        const stands = Recent.said(listed.map(one => String(one.row.title || one.row.address
            || one.row.id)), false, words.said ?? {});
        const said = Words.of("agents").said ?? {};
        const now = new Date();
        const home = Sessions.home;
        const homely = folder => folder === home || folder.startsWith(`${home}/`)
            ? `~${folder.slice(home.length)}` : folder;
        // Here, never where a boot kept it (reopenThemAll), so it opens
        // alone in the folder its record ran in, as one of Sessions to
        // Reopen does.
        const reopen = one => Sessions.reopen(Object.assign({}, one,
            { machine: undefined, cwd: String(one.presence?.cwd ?? "") }),
            cwd => provider.launcher?.tellOnOpen([[homely(cwd), String(
                (Words.of("verb:agents:to-reopen:one").said ?? {}).isGone ?? ""), Theme.alert]]));
        const row = one => thing({
            thing: true,
            key: `agents:recent:${one.id}`,
            name: String(one.title || one.address || one.id),
            also: Fuzzy.prepare(`${one.address ?? ""} ${one.folder ?? ""} ${one.quest ?? ""}`),
            side: SessionRow.closedAt(one.ended, now, said,
                date => Qt.formatDate(date, "ddd")),
            note: SessionRow.closedNote(one),
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
            side: stands.side,
            heldNames: stands.held,
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
    readonly property var budgets: {
        if (!opened())
            return madeBefore("budgets", null);
        // AI spend's row as it reads now, since AI spend makes its rows
        // only while it's wanted, which it isn't while Agents is held;
        // found among them where it can't say (1KZFHC, 2026-11-10).
        const spend = source("spend");
        const now = spend?.rowNow("spend:budgets");
        const spent = now !== undefined ? now : (spend?.everything() ?? [])
            .find(row => row.key === "spend:budgets") ?? null;
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

    things: [toReopen, landed, budgets, recent, start].filter(row => row !== null)

    // Asked while they aren't wanted, as a step into Agents asks before it's
    // held, its rows are laid out as asked, from the daemon's rows held.
    function everything() {
        const sessions = provider.wanted || !opened() ? all : laidOut();
        return sessions.concat([toReopen, landed, recent, start].filter(row => row !== null));
    }

    // How they stand together, "1 needs you \u00b7 3 working \u00b7 $214
    // today", as the daemon says it, the day's dollars none while the
    // screen is shared; held while the launcher is shut, as Home's row is
    // read.
    function summary() {
        void rowsKind?.revision;
        return String(rowsKind?.held?.[summaryKey]?.summary?.said ?? "");
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
