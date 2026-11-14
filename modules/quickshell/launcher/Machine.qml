// How this machine is doing, answered in the field, each thing with its
// parts a step in:
//
// Each thing's parts lead with what moves most and end with what
// describes it, its Model last (VKCF3Z, 2026-11-07); words.json lists
// them in that order, and `ordered` keeps it.
//
// - "cpu", "processor", "cores" or "load": how busy the processors are,
//   where their Time was Spent, the Pressure on them (how long something
//   waited for one), how often they Switch tasks, take Interrupts and
//   Start processes, their Cores (one reading each, oldest core first,
//   with its kind, clock and temperature), which are Maxed, the Clock and
//   its range, base, driver, governor, preference and turbo, what Held it
//   Back over the last second and each core held, and what Kept it From
//   Resting over the last ten seconds and each part that did (as its own
//   telemetry says, which the daemon hears), the Temperature, the
//   programs Using it, the Load, how many tasks are Running, the NPU's
//   Load, the Kinds of core and the Model with its family, microcode and
//   caches. Both telemetry rows stand whenever the telemetry answers, at
//   0% when nothing holds it back or keeps it from resting, so the rows
//   under them don't move.
// - "memory" or "ram": what's Used, the Pressure, how often pages Fault,
//   the Swapping and Paging, the programs Using it, the Swap, what's
//   Cached, what it's Held By, what's Committed, the programs killed for
//   it (Out of Memory), Every Count the kernel keeps, and its Type, each
//   module as the machine read it at boot.
// - "disk", "storage" or "free space": the Space each disk has, as the disk
//   guard says it (pieces/Disk.qml, so that fact has one home), or the file
//   system where no guard watches, then how fast the disk is Reading,
//   Writing, Discarding and Flushing, the Pressure on it, how Busy it is,
//   each kind of request's Latency, its Queue, its Temperature, Health,
//   File Systems and Model, with its firmware, kind, link, size, block
//   size and scheduler.
// - "uptime": how long the machine has been up, and of that how long it
//   was Asleep and its processors Idle, when it Started and its Kernel.
//
// A part has its own words too, "disk speed" or "ram type", which answer
// it alone. The words that ask each, and every part's name, are the
// vocabulary's (launcher/words.json, Words.qml).
//
// Every figure is the machine daemon's (modules/machine), held over
// connections to its socket kept open (Daemon.qml), so nothing is read or
// spawned here: the daemon reads the kernel and asks UDisks, and says each
// record as it changes. The processor, the memory, the uptime and the
// disk come from it as rows ready to draw, worded and in their words'
// order, held and made as every place's are (ReadyRows.qml), so nothing
// here words, places or sorts them; only what a disk guard says, the
// Space it watches and What's Filling It, is still made here, until the
// guards' status is served as rows too. It reads only while watched, so
// those are watched while one of Machine's answers is shown, the disk's
// own place while a Disk answer or row is, and the programs using the
// machine, which cost it the most to read, only while a Using row can be
// seen: the processor's or the memory's parts stepped into, or Using
// asked for alone. Its
// records come at most once a second, and once more a moment after the
// first, whose rates are since the machine started, so a rate stands for
// now as soon as it can; each answer here is asked again as they come.
// With the daemon down, each thing stays, dimmed, saying it isn't
// answering.
import qs
import Quickshell
import QtQuick
import "Interfaces.js" as Interfaces
// Bytes as decimal to three digits, "12.3 GB", as what's told to a session
// says them; it's the launcher's one such reading, kept there for networks.
import "HomeNetwork.js" as HomeNetwork
import "Order.js" as Order
import "Holders.js" as Holders
import "SessionRow.js" as SessionRow

Provider {
    id: provider

    examples: ["cpu", "memory", "uptime", "disk speed"]

    // What talking to a holder's session said, for the foot.
    property string talked: ""

    // Each answer by the thing the vocabulary names it as (Words.qml),
    // whose words ask it, best first, whole and by their start
    // (Provider.qml).
    readonly property var kinds: ({
        memory: "machine:memory",
        cpu: "machine:cpu",
        uptime: "machine:uptime",
        disks: "machine:disk"
    })
    // A part asked for alone, by its own words, as the thing and the
    // part's name.
    readonly property var parts: {
        const out = {};
        for (const kind of Object.keys(kinds))
            for (const part of Words.of(kinds[kind]).parts ?? [])
                for (const question of Words.answers(part))
                    out[question] = [kind, Words.name(part)];
        return out;
    }
    questions: Object.keys(kinds).reduce((all, kind) => {
        all.push(...Words.started(kinds[kind]));
        return all;
    }, [])
        .concat(Object.keys(parts))

    function answer(typed) {
        const question = asks(typed);
        if (question === "")
            return null;
        const part = parts[question];
        if (part)
            return alone(part[0], part[1]);
        return told(Object.keys(kinds).find(kind => Words.answers(kinds[kind]).includes(question)));
    }

    // A part's name, as the vocabulary says it: "Cores" for
    // machine:cpu:cores.
    function named(id, fill) {
        return Words.name(`machine:${id}`, "", fill);
    }

    // What a part's words say beside it, by its id and the phrase's name.
    function says(id, phrase, fill) {
        let said = (Words.of(`machine:${id}`).said ?? {})[phrase] ?? "";
        for (const key of Object.keys(fill ?? {}))
            said = said.replace(`{${key}}`, fill[key]);
        return said;
    }

    function told(kind) {
        switch (kind) {
        case "cpu":
            return cpu();
        case "memory":
            return memory();
        case "uptime":
            return uptime();
        case "disks":
            return disks();
        default:
            return null;
        }
    }

    // One part as an answer: its figure as the headline, its own parts a
    // step in, and Return copying what it shows, as every value does.
    function alone(kind, name) {
        const whole = told(kind);
        const part = whole && !whole.asking && !whole.gone
            ? whole.rows.find(row => row.name === name) : null;
        const resting = kind === "cpu" && name === named("cpu:kept-from-resting");
        if (!part)
            return resting && whole ? awaited(whole, name) : whole && (whole.asking || whole.gone)
                ? Object.assign({}, whole, { title: name }) : null;
        // A part's own parts are the rows served inside it, where it was
        // served; one the shell makes holds its verbs.
        const within = part.within ?? part.verbs;
        return {
            // Kept From Resting stands in a place whether or not anything
            // keeps it from resting, so it's kept there with no parts too;
            // and asked alone, it's what makes the telemetry be read.
            whole: resting,
            about: resting ? kinds.cpu : kind === "disks" ? kinds.disks : undefined,
            headline: part.side,
            copy: part.copies ? part.copies() : part.side ?? "",
            graph: part.graph,
            glyph: whole.glyph,
            hue: "machine",
            title: part.name,
            rows: within ? within() : [],
            note: part.note !== "" ? part.note : whole.note,
            live: provider
        };
    }

    // Kept From Resting before the telemetry has said it: the reading is
    // asked for only once something shows it, so it holds its place with
    // the fixed placeholder beside it until the first comes, a second or
    // less, and nothing moves when it does. If the reader refuses, it
    // stays there dimmed, saying so. On a machine with no reader it's left
    // out.
    function awaited(whole, name) {
        const state = telemetryWatch.held["telemetry:processor"]?.state ?? "asking";
        if (state === "missing")
            return null;
        const refused = state === "refused";
        return {
            headline: refused ? "" : "\u2026",
            copy: "",
            glyph: whole.glyph,
            hue: "machine",
            title: name,
            rows: [],
            note: refused ? says("cpu:kept-from-resting", "refused") : "",
            asking: !refused,
            gone: refused,
            whole: true,
            about: kinds.cpu,
            live: provider
        };
    }

    // A thing whose record the daemon hasn't said: while it's starting or
    // the launcher has only just opened, its place held with the fixed
    // placeholder, so nothing moves when the record comes a moment later;
    // and while the daemon isn't answering, dimmed, saying so (VKCF3Z's
    // rules for a daemon that's down). It says it has parts, as it will
    // once its record comes, so a place lists it from the start and says
    // it again in place as the record comes (Launcher.qml's relive): a
    // place is made once, and an answer that came later would never join
    // it.
    function unheld(kind) {
        const down = served.state === "not answering";
        return {
            headline: down ? "" : "\u2026",
            copy: "",
            glyph: Words.glyph(kinds[kind]),
            hue: "machine",
            title: Words.name(kinds[kind]),
            about: kinds[kind],
            rows: [],
            note: down ? says("cpu", "down") : "",
            asking: !down,
            gone: down,
            parted: true,
            live: provider
        };
    }

    // An answer that's asked again as its records change, so its figures
    // move, and its parts stepped into are never more than a second old.
    // Its parts are made by `parts` only when they're first looked at, and
    // it says it has some (`parted`), so the headline alone makes none of
    // them.
    function fresh(fields, parts) {
        const said = Object.assign({ live: provider, parted: true, hue: "machine" }, fields);
        let made = null;
        Object.defineProperty(said, "rows", {
            enumerable: true,
            get: () => made ?? (made = parts())
        });
        return said;
    }

    // How rows are drawn. A figure says something and Return closes; a part
    // holding more (Provider's holderRow) copies its value, the right arrow
    // stepping into what it holds, and wears its thing's glyph.
    function figure(fields) {
        return copyRow(Object.assign({ copy: "", hue: "machine", glyph: "", note: "" }, fields));
    }

    // A graph a row draws (Choice.qml): a bar has one value, 0 to 1.
    function bar(value, warn) {
        return { shape: "bar", values: [Math.max(0, Math.min(1, value))], warn: warn === true };
    }

    // A thing's parts in the order its words list them (VKCF3Z, 2026-11-07:
    // what moves most first, then what describes, the model last), each
    // part made where its figure is, then put in its place.
    function ordered(thing, rows) {
        const order = (Words.of(thing).parts ?? []).map(part => Words.name(part));
        const place = row => {
            const at = order.indexOf(row.name);
            return at < 0 ? order.length : at;
        };
        return rows.map((row, at) => ({ row: row, at: at }))
            .sort((a, b) => place(a.row) - place(b.row) || a.at - b.at)
            .map(each => each.row);
    }

    // The processors.

    // The processor, as the daemon serves it ready, what held it back and
    // what kept it from resting among its parts while its telemetry is
    // watched.
    function cpu() {
        return servedAnswer("cpu", kinds.cpu);
    }

    // The memory.

    // The memory, as the daemon serves it ready: its row's side is the
    // headline, and its parts, worded and in their words' order, a step
    // in.
    function memory() {
        return servedAnswer("memory", kinds.memory);
    }

    // A row the daemon serves, made as every place's are (ReadyRows.qml's
    // thing), its Copy and its step inside its verbs: in the machine's hue
    // where it names none, and closing on Return where it does nothing,
    // as a figure copying nothing does. `within` is the rows served
    // inside it, a part's own parts where it's asked alone.
    function made(rows, row, key) {
        const idle = (row.acts ?? []).length > 0 ? {} : {
            verb: "close",
            figure: true,
            plain: true,
            now: true,
            copies: () => "",
            run: () => {}
        };
        return rows.thing(row, key, Object.assign({
            hue: row.hue ?? "machine",
            within: () => rows.inside(key)
        }, idle));
    }

    // A top row the daemon serves, as the answer the launcher asks for by
    // its words: its side the headline, its parts a step in; held in its
    // place while it hasn't been served.
    function servedAnswer(id, kind) {
        return servedFresh(served, `row:machine:${id}`, kind, id);
    }

    // The top row of `key` that `rows` holds, as an answer, the rows
    // inside it a step in, or its place held as `unheld` holds `id`'s.
    function servedFresh(rows, key, kind, id) {
        const top = rows.served(key);
        if (!top)
            return unheld(id);
        return fresh({
            headline: rows.sideOf(top),
            graph: top.graph,
            glyph: Theme.glyph[top.glyph ?? ""] ?? Words.glyph(kind),
            title: top.name ?? "",
            about: kind,
            note: rows.noteOf(top)
        }, () => rows.inside(key));
    }

    // The disk.

    // The disk guards, as the disk piece reads their status: what they
    // say of the room and of what fills it is still this file's, until
    // the guards' status is served as the Disk's rows too.
    readonly property var guard: {
        const fact = Facts.all.find(fact => fact.entry.name === "disk");
        return fact && fact.piece && fact.piece.present ? fact.piece : null;
    }

    // A guard's new word, its stopping as its heartbeat's deadline passes
    // and what an act on a holder told are said at once while shown, as
    // nothing the daemon serves moves with them.
    Connections {
        target: provider.guard
        ignoreUnknownSignals: true

        function onSaidChanged() {
            if (provider.shown)
                provider.answered();
        }

        function onSilentChanged() {
            if (provider.shown)
                provider.answered();
        }

        function onToldChanged() {
            if (provider.shown)
                provider.answered();
        }
    }

    Connections {
        target: users

        function onHeard(keys) {
            if (provider.shown)
                provider.answered();
        }
    }

    // Quests, which titles a quest filling the disk by its id (filling), each
    // named as the guard's holders arrive, open or shut, so no opening shows one.
    readonly property var quests: (launcher?.sources ?? [])
        .find(one => one.settings.name === "quests") ?? null
    readonly property var heldQuests: Holders.quests(guard?.holders)
    onHeldQuestsChanged: titleHolders(guard)
    onQuestsChanged: titleHolders(guard)
    Connections {
        target: provider.quests

        function onTitlesHeard() {
            if (provider.shown)
                provider.answered();
        }
    }

    function titleHolders(guard) {
        for (const id of Holders.quests(guard?.holders))
            quests?.titleOf(id);
    }

    // The Disk, as the daemon serves it ready, its room on / beside it;
    // where a guard watches the disks, the room it says beside it and its
    // Space and What's Filling It among the served parts, in their words'
    // order, in place of the served Space.
    function disks() {
        const parts = () => diskRows.inside("row:disk:disk");
        const guard = provider.guard;
        if (guard === null)
            return servedFresh(diskRows, "row:disk:disk", kinds.disks, "disks");
        const worst = guard.worst;
        const watched = guard.disks.length > 0;
        return fresh({
            headline: worst === null ? says("disk", "unwatched")
                : says("disk", "head", { free: guard.shown(worst.free), mount: worst.path }),
            graph: worst ? bar(1 - worst.free / worst.total, worst.band !== "healthy") : undefined,
            glyph: Words.glyph(kinds.disks),
            title: Words.name(kinds.disks),
            about: kinds.disks,
            // The guard's own words are each disk's, in its drawer (disked).
            note: guard.standing
        }, () => ordered(kinds.disks, guardParts(guard).concat(parts()
            .filter(row => !watched || row.key !== "machine:disk:space"))));
    }

    // What's filling the disks, as the guards measured it (Joel, 12:27:
    // "why isn't the disk guard helping?"): each holder biggest first, its
    // share of the disk it's on, and whether the guard may clear any of it
    // or why not, with the guard's own sentence below. Beside the part, the
    // biggest, so what matters most is seen without a step in.
    function filling(guard, chip) {
        const said = Words.of("machine:disk:filling").said ?? {};
        const called = holder => Holders.called(holder, said[holder.owner?.kind],
            id => provider.quests?.titleOf(id), Sessions.home);
        const each = holder => {
            const disk = guard.holding(holder.path) ?? guard.worst;
            return thingRow({
                thing: true,
                key: `machine:disk:holder:${holder.path}`,
                name: called(holder),
                side: guard.shown(holder.bytes),
                graph: disk ? bar(holder.bytes / disk.total) : undefined,
                note: holder.clearing ? said.clearing ?? ""
                    : holder.clearable && !holder.kept ? said.clears ?? "" : holder.why ?? "",
                reads: holder.says ?? ""
            }, () => holderParts(guard, holder, called(holder)));
        };
        const minutes = Math.max(0, Math.round((Date.now() / 1000 - guard.holdersAt) / 60));
        const biggest = guard.holders[0];
        return holderRow({
            name: named("disk:filling"),
            side: `${called(biggest)} \u00b7 ${guard.shown(biggest.bytes)}`,
            glyph: chip,
            note: minutes < 1 ? said.now ?? "" : (said.ago ?? "").replace("{minutes}", minutes)
        }, () => guard.holders.map(each));
    }

    // What can be done to a holder, then what it reads, then Copy the Path
    // (VKCF3Z's design, the disk guard's acts): open its folder; keep it
    // from the guard's clearing, a switch that asks Joel why as it's
    // turned on, where the account's guard measured it, since only that
    // guard keeps; clear what the guard would take of it now, asking once,
    // since it can't be taken back; its size and what a clear would take.
    function holderParts(guard, holder, name) {
        const act = (id, fields) => (fields.copy ? copyRow : thing)(Object.assign({
            key: `verb:disk:${id}:${holder.path}`,
            name: Words.name(`verb:disk:${id}`),
            verb: Words.verb(`verb:disk:${id}`),
            glyph: Words.glyph(`verb:disk:${id}`),
            hue: "action"
        }, fields));
        const rows = [act("open", { run: () => Niri.spawn(["xdg-open", holder.path]) })];
        // Each session it belongs to, at work on its quest or in its
        // folder, the one that worked last first, told what Joel types and
        // which folder and how much it's about, so it can clear its own
        // (Joel: "talk to its session"); at its right its title and how it
        // stands, as Agents says them, and absent where none is, never
        // dimmed (VKCF3Z, 2026-11-08).
        const talk = Words.of("verb:disk:talk");
        const standing = Words.of("agents").said ?? {};
        // The folder as the guard names it, "~/src/reef/target", as Joel reads it too.
        const about = (talk.said?.about ?? "").replace("{path}", holder.name || holder.path)
            .replace("{size}", HomeNetwork.bytes(holder.bytes, settings));
        for (const session of Holders.owners(Sessions.running.concat(Sessions.parked),
            holder)) {
            const state = SessionRow.state(session.presence, Sessions.asks(session), false);
            const sided = Holders.talkSide(session.title, String(standing[state] ?? ""));
            // Its talk shows under the field as Agents' Talk shows one.
            const talking = (provider.launcher?.sources ?? [])
                .find(one => one.settings.name === "agents")
                ?.talking(session.address, session.title) ?? null;
            rows.push(act("talk", {
                key: `verb:disk:talk:${holder.path}:${session.address}`,
                side: sided.map(item => item.said).join(" \u00b7 "),
                sideItems: sided,
                // The ask names who it's told to (VKCF3Z, 2026-11-08).
                ask: (talk.ask ?? "").replace("{name}", session.title),
                chip: session.title,
                state: () => provider.talked,
                talks: talking?.talks,
                heard: talking?.heard,
                given: words => {
                    const said = `${words.trim()} ${about}`;
                    if (words.trim() !== "" && Sessions.talk(session.address, said)) {
                        talking?.said(said);
                        provider.talked = (talk.said?.sent ?? "")
                            .replace("{name}", session.title);
                    }
                }
            }));
        }
        if (holder.ours) {
            const kept = holder.kept ?? null;
            const keeping = {
                side: kept ? `yes, ${kept.reason}` : "no",
                stays: true,
                state: () => guard.told
            };
            rows.push(act("keep", kept
                ? Object.assign(keeping, {
                    verb: Words.verb("verb:disk:keep", "kept"),
                    run: () => guard.release(holder.path)
                })
                : Object.assign(keeping, {
                    ask: Words.of("verb:disk:keep").ask ?? "",
                    chip: name,
                    given: reason => guard.keep(holder.path, reason)
                })));
        }
        // Only what a clear takes now, never what waits to sit idle, and
        // never what Joel keeps; gone while the guard clears it, the
        // holder's note saying so.
        if (holder.ours && (holder.clears ?? 0) > 0 && !holder.kept && !holder.clearing) {
            const amount = guard.shown(holder.clears);
            const sure = () => [thing({
                key: `verb:disk:clear:sure:${holder.path}`,
                // The step that asks Joel to be sure, so its Return does it (Launcher.lands).
                sure: true,
                name: (Words.of("verb:disk:clear").sure ?? "").replace("{amount}", amount)
                    .replace("{name}", name),
                verb: Words.verb("verb:disk:clear"),
                glyph: Words.glyph("verb:disk:clear"),
                hue: "alert",
                stays: true,
                state: () => guard.told,
                run: () => guard.clear(holder.path)
            })];
            rows.push(act("clear", { side: amount, verbs: sure, targets: sure }));
        }
        rows.push(figure({
            key: `machine:disk:holder:size:${holder.path}`,
            name: named("disk:holder:size"),
            side: guard.shown(holder.bytes)
        }));
        // How much it grew over the last day, as the guard measured it, once
        // it has measured for an hour; what Joel looks at when something
        // fills the disk faster than it should.
        const grew = holder.grew ?? null;
        if (grew && grew.bytes > 0)
            rows.push(figure({
                key: `machine:disk:holder:grew:${holder.path}`,
                name: named("disk:holder:grew"),
                side: `${guard.shown(grew.bytes)} in ${Interfaces.lasted(grew.over)}`
            }));
        if ((holder.clears ?? 0) > 0)
            rows.push(figure({
                key: `machine:disk:holder:clears:${holder.path}`,
                name: named("disk:holder:clears"),
                side: guard.shown(holder.clears)
            }));
        // An act like any other, though copying is its Return, so it wears
        // its glyph as acts do rather than standing as a figure.
        rows.push(act("copy-path", { copy: holder.path, figure: false, plain: false }));
        return rows;
    }

    // A disk is a device, so a thing (VKCF3Z, 2026-10-29): beside it its
    // room and, while it's short, what the guard is doing about it, so
    // that's seen without selecting it; in the drawer, while it's short,
    // its figures and the guard's own sentence, and its name alone while
    // it's healthy, as the sentence then only says the room again. Its
    // room is beside it, so the drawer doesn't say Free again, as Now's
    // disk drawer doesn't (VKCF3Z, 2026-11-08); its Size and Full In stay.
    function disked(guard, disk, fields) {
        const free = `${guard.shown(disk.free)} free`;
        if (disk.band === "healthy")
            return figure(Object.assign({ thing: true, key: `machine:disk:${disk.path}`,
                side: free }, fields));
        const said = Words.of("machine:disk:space").said ?? {};
        const doing = disk.exhausted ? said.cleared
            : disk.band === "filling" ? said.clearing : said.clearingAll;
        const full = typeof disk.full_at === "number" ? disk.full_at - Date.now() / 1000 : 0;
        const facts = [
            { label: named("disk:size"), value: guard.shown(disk.total) },
            full > 0 ? { label: named("disk:full-in"), value: guard.duration(full) } : null
        ];
        return figure(Object.assign({
            thing: true,
            key: `machine:disk:${disk.path}`,
            side: `${free} \u00b7 ${doing ?? guard.banded(disk.band)}`,
            drawer: { title: fields.name, facts: facts.filter(fact => fact !== null),
                body: disk.says ?? "" }
        }, fields));
    }

    // What the guards say among the Disk's parts: the room on each disk
    // they watch, the least first, and what's filling them. The served
    // parts say the rest. A stopgap until the guards' status is served as
    // the Disk's rows, with keep, unkeep and clear as its acts.
    function guardParts(guard) {
        if (guard.disks.length === 0)
            return [];
        const chip = Words.glyph(kinds.disks);
        // Disks are amounts: the worst band first, as the disk piece
        // judges it, so the first is the headline's; within a band the
        // system disk, as the piece leads with it (#3768), so a small
        // build disk with room never reads as a nearly full machine; then
        // the least room, then A to Z.
        const system = one => one.path === "/" ? 0 : 1;
        const watched = Order.sorted(guard.disks, Order.amounts({
            worse: (a, b) => guard.worse(a, b) ? -1 : guard.worse(b, a) ? 1
                : system(a) - system(b) || a.free - b.free,
            name: one => one.path
        }));
        const each = one => disked(guard, one, {
            name: one.path,
            graph: bar(1 - one.free / one.total, one.band !== "healthy")
        });
        const worst = watched[0];
        const least = watched.every(one => worst.free <= one.free);
        const space = {
            name: named("disk:space"),
            graph: bar(1 - worst.free / worst.total, worst.band !== "healthy"),
            note: watched.length === 1 ? `on ${worst.path}`
                : least ? `on ${worst.path}, the least of ${watched.length}`
                : `on ${worst.path}, one of ${watched.length}`
        };
        const rows = [watched.length > 1
            ? holderRow(Object.assign(space, {
                glyph: chip,
                side: `${guard.shown(worst.free)} free`
            }), () => watched.map(each))
            : disked(guard, worst, space)];
        if (guard.holders.length > 0)
            rows.push(filling(guard, chip));
        return rows;
    }

    // The boot.

    // How long it's been up, served as the time it started, so it counts
    // up here as the clock does (the Timer below) while the daemon's row
    // changes only when the machine wakes; its parts are Idle, Asleep,
    // when it started and its kernel.
    function uptime() {
        return servedAnswer("uptime", kinds.uptime);
    }

    // While one of its answers is shown, as the answer to what's typed, a
    // row in a category's list or a step held, it's asked again as the
    // daemon's records change. A place lists an answer only once it has
    // parts, which come from the records, so a place holding one of
    // Machine's whole answers (the launcher's `placed`, from its words
    // alone) watches too, or Machine's own place would wait for a read
    // nothing asks. Kept From Resting, which Power holds on battery, is
    // listed whole while it waits (`awaited`), so it's asked once listed.
    // A terminal's `launcher show` sees them too, with the launcher closed,
    // while the rows it walked through or listed are Machine's
    // (Launcher.qml's terminalSees), so a place asked cold is read.
    readonly property bool shown: launcher !== null && (launcher.open
        && (launcher.answer?.live === provider
            || launcher.held.some(step => step.row.live === provider)
            || launcher.matches.some(row => row.live === provider)
            || launcher.placed.some(id => /^machine:[a-z]+$/.test(id)))
        || terminalWalked.concat(terminalListed).some(row => row?.live === provider))
    readonly property var terminalWalked: launcher?.terminalSees.walked ?? []
    readonly property var terminalListed: launcher?.terminalSees.listed ?? []

    // Whether a daemon it watches hasn't yet answered this watch whole, so
    // what it shows is the placeholder, or what an earlier watch left,
    // still being read (Provider.stillReading).
    stillReading: [served.link, diskRows.link, users].some(watch => watch?.live
        && !watch.answeredThisWatch && watch.state !== "not answering")

    // What the open launcher holds, answers and lists is left as it was
    // once it closes, and set afresh only as it opens, so with it closed
    // only a terminal's rows count (Y2YBTS's review of #2433: Processor
    // left held would read every process each second for a terminal).
    readonly property bool launcherOpen: launcher?.open === true

    // Whether a Using row can be seen, so the daemon is asked for the
    // programs: the processor's or the memory's parts stepped into, or
    // Using asked for alone. Building parts to find one, or to search
    // them, asks for nothing, since every process's file is read for it
    // each second (Y2YBTS's review of #2276).
    readonly property var usingNames: [named("cpu:using"), named("memory:using")]
    readonly property bool usingWanted: shown && (launcherOpen && (launcher.held.some(step =>
        step.row?.live === provider
            && (step.row.about === kinds.cpu || step.row.about === kinds.memory))
        || launcher.answer?.live === provider && usingNames.includes(launcher.answer.title))
        || terminalWalked.some(row => row?.live === provider
            && (row.about === kinds.cpu || row.about === kinds.memory)))

    // Whether a Disk answer or row is shown, listed or stepped into, so the
    // daemon reads the disks: each read asks an NVMe disk its temperature,
    // a command to the drive, which nothing else Machine shows needs.
    readonly property bool disksWanted: shown && (launcherOpen && ([launcher.answer,
        ...launcher.held.map(step => step.row), ...launcher.matches]
        .some(row => row?.live === provider && row.about === kinds.disks)
        || launcher.placed.includes("machine:disk"))
        || terminalWalked.concat(terminalListed)
            .some(row => row?.live === provider && row.about === kinds.disks))

    // The processor's telemetry is watched while a Processor answer is
    // listed, shown or stepped into, so what held it back is there once its
    // parts are opened, and it's read for nothing else Machine answers.
    // Kept From Resting is asked alone too, and a place may hold it, as
    // Power does on battery: what the place holds now is the launcher's
    // `placed`, read from its words and the battery alone, so reading it
    // here can't loop back through an answer.
    readonly property bool watched: shown && launcherOpen && ([launcher.answer,
        ...launcher.held.map(step => step.row), ...launcher.matches]
        .some(row => row?.live === provider && row.about === kinds.cpu)
        || launcher.placed.includes("machine:cpu:kept-from-resting"))

    // The uptime moves with the clock, a minute at a time, while shown.
    Timer {
        interval: 60000
        repeat: true
        running: provider.shown
        onTriggered: provider.answered()
    }

    // The machine daemon: Machine's own rows, served ready while one of
    // its answers is shown (the processor, the memory and the uptime), the
    // disks while a Disk answer or row is, and the programs while a Using
    // row can be, in the open launcher or a terminal's show. Nothing is
    // read while neither sees them; a first look holds the placeholder for
    // the few milliseconds the daemon's first read takes. The place is
    // remade as the programs are read, so Using It names them only while a
    // Using row can be seen.
    ReadyRows {
        id: served

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
        place: "machine"
        maker: provider
        make: (row, key) => provider.made(served, row, key)
        drawn: provider.launcherOpen
        live: provider.shown
        paced: provider.launcher
        keptThroughGap: false
        onHeard: (changed, gone) => provider.heard()
    }

    // What's shown is asked again as its rows change.
    function heard() {
        if (shown)
            answered();
    }

    // The processor's telemetry, asked of the daemon only while it's
    // watched: its one record says whether it's asking, heard, refused or
    // missing, so Kept From Resting holds its place until it's served,
    // and each second heard comes as Machine's rows, remade.
    Daemon {
        id: telemetryWatch

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
        kind: "telemetry"
        live: provider.watched
        paced: provider.launcher
        keptThroughGap: false
        answersWhole: true
    }

    // The Disk's rows, a place of their own, served ready while a Disk
    // answer or row is seen, since each read asks an NVMe disk its
    // temperature; their things are still Machine's.
    ReadyRows {
        id: diskRows

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
        place: "disk"
        maker: provider
        make: (row, key) => provider.made(diskRows, row, key)
        drawn: provider.launcherOpen
        live: provider.disksWanted
        paced: provider.launcher
        keptThroughGap: false
        onHeard: (changed, gone) => provider.heard()
    }

    Daemon {
        id: users

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
        kind: "using"
        live: provider.shown && provider.usingWanted
        paced: provider.launcher
        keptThroughGap: false
        answersWhole: true
    }
}
