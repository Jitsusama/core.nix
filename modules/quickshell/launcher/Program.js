// What a program's drawer says (Applications.qml): where its windows are and
// what it is above its name; what moves first, the share of the processors
// its processes use, the memory they hold and how many there are, and what
// it plays; then what other providers say of it; then what describes it,
// the kinds of file it opens, the links it handles, its categories and
// keywords, where it starts and whether in a terminal, and its version and
// package; and what identifies it last, its command, window class, desktop
// entry's id and file, and store path (VKCF3Z, 2026-11-07 and 08). Its desktop entry's comment is
// its words. What its entry says is the programs daemon's record of it. A
// slow making says what each part of it spent. Plain JavaScript, so
// tests/program-drawer.qml holds it in Qt's own engine.
.pragma library
.import "../FileFacts.js" as FileFacts
.import "Numbers.js" as Numbers

// Parts of a value, and a list alone in one, run on a dot.
const DOT = " \u00b7 ";
// A file's kind by the name its files end in, a glob as `*.pdf`.
const ENDING = /^\*\.([a-z0-9]+)$/i;
// A category's words, as its name runs them together: WebBrowser.
const WORDS = /([a-z])([A-Z])/g;
// A template's field, {count}.
const FIELD = /\{(\w+)\}/g;
// What an unregistered kind's name starts with, x-pdf.
const UNREGISTERED = /^x-/;

// A template's {fields} filled.
function filled(template, fields) {
    return String(template ?? "").replace(FIELD, (whole, name) =>
        fields[name] === undefined ? whole : String(fields[name]));
}

// Where its windows are and what it is, one where: "on 4 · web browser"
// while one workspace holds them, "on 3 workspaces · web browser" for
// more, never a list beside the dot (VKCF3Z, 2026-11-08); what it is alone
// with none.
function eyebrow(places, kind, said) {
    const numbers = Array.from(new Set(places.filter(place => place !== null
        && place !== undefined)));
    if (numbers.length === 0) {
        return kind;
    }
    const on = numbers.length === 1 ? filled(said.on, { places: numbers[0] })
        : filled(said.workspaces, { count: numbers.length });
    return kind === "" ? on : `${on}${DOT}${kind}`;
}

// A share of the whole machine, from 0 to 1, as a percentage by the
// launcher's one rule (Numbers.significant), three significant digits with
// their zeros kept, pointed as `marks` say: 4.50%, 37.0%; none at all
// reads "0%", as the machine's Using It says it, since nothing has no
// places to read it to.
function percent(share, marks) {
    return share === 0 ? "0%"
        : `${Numbers.significant(share * 100, marks?.group, marks?.point)}%`;
}

// What a program's processes use, from what the machine daemon says of
// each of its windows' processes and every one it started (its `tree`
// kind, each { pid, share_ratio, resident_b, processes, pids }): their
// share of the processors, the memory they hold and how many there are,
// summed; null while it says nothing. A window's process started from
// another's is in that one's tree, its `pids`, so it's counted there
// alone rather than twice. Each process another holds is found once, in
// one pass, rather than every tree looking through every other's: kitty's
// 41 processes on Joel's laptop made that some 84,000 looks a drawer.
// Given the processes holding agents' windows alone (`agents`, from
// split), it says their memory too, which is kitty's share for agents.
function usage(trees, agents) {
    const known = trees.filter(tree => tree !== null && tree !== undefined);
    const held = new Set();
    for (const tree of known) {
        for (const pid of Array.from(tree.pids ?? [])) {
            if (pid !== tree.pid) {
                held.add(pid);
            }
        }
    }
    const said = known.filter(tree => !held.has(tree.pid));
    if (said.length === 0) {
        return null;
    }
    const sum = (field, trees) => (trees ?? said)
        .reduce((all, tree) => all + (Number(tree[field]) || 0), 0);
    const summed = {
        processor: Math.max(0, Math.min(1, sum("share_ratio"))),
        memory: sum("resident_b"),
        count: sum("processes")
    };
    if (agents !== undefined) {
        const theirs = new Set(agents);
        summed.agents = sum("resident_b", said.filter(tree => theirs.has(tree.pid)));
    }
    return summed;
}

// The memory its processes hold, whole, as Machine says it, with the
// agents' windows' share beside it where it holds theirs, as kitty does
// (VKCF3Z [b31d7a90]): "1.80 GB · agents' windows 1.60 GB".
function memory(used, said) {
    const whole = FileFacts.weighed(used.memory);
    if (!(used.agents > 0)) {
        return whole;
    }
    return whole + DOT + filled(said.agents, { memory: FileFacts.weighed(used.agents) });
}

// A program's windows (as the windows daemon says them, `session` on an
// agent's) split by whose they are: each process behind them once
// (`pids`), as two windows are often one process's; those holding only
// agents' windows (`agents`), as most of kitty's do, while one holding
// any of Joel's own is his; and his own windows (`own`), said as where
// it is, since agents' windows are reached through Agents (VKCF3Z
// [b31d7a90]).
function split(windows) {
    const pids = new Set();
    const his = new Set();
    const own = [];
    for (const window of Array.from(windows ?? [])) {
        const pid = window.pid;
        const known = Number.isInteger(pid) && pid > 0;
        if (known) {
            pids.add(pid);
        }
        if (window.session !== true) {
            own.push(window);
            if (known) {
                his.add(pid);
            }
        }
    }
    const sorted = Array.from(pids).sort((a, b) => a - b);
    return { pids: sorted, agents: sorted.filter(pid => !his.has(pid)), own };
}

// A kind by the ending its files have, PDF, from its globs heaviest first
// as the programs daemon gives them, the first plain ending; or where it
// has none by its own name, without the x- an unregistered one carries.
function kindName(type, globs) {
    for (const glob of Array.from(globs ?? [])) {
        const ending = ENDING.exec(glob);
        if (ending !== null) {
            return ending[1].toUpperCase();
        }
    }
    const own = String(type.split("/")[1] ?? "").replace(UNREGISTERED, "").split("+")[0];
    return own.length <= 4 ? own.toUpperCase() : own;
}

// The kinds of file a program opens, ["PDF", "HTML", "PNG"], each once, and
// the links it handles, ["http", "https"], from the programs daemon's
// record of its entry (modules/programs): `opens`, each { type, globs },
// and `handles`, in its entry's order. A type listed twice is named by its
// first, so text/html is HTML once rather than HTML and HTM.
function opens(record) {
    const types = Array.from(record?.opens ?? [])
        .filter((kind, at, all) => all.findIndex(one => one.type === kind.type) === at);
    return {
        kinds: Array.from(new Set(types.map(kind => kindName(kind.type, kind.globs))
            .filter(name => name !== ""))),
        schemes: Array.from(record?.handles ?? [])
    };
}

// A desktop entry's categories as words, "Network \u00b7 Web Browser",
// leaving out a vendor's own (X-).
function categories(each) {
    return Array.from(new Set(Array.from(each ?? []).filter(one => !one.startsWith("X-"))
        .map(one => one.replace(WORDS, "$1 $2")))).join(DOT);
}

// The kinds it opens on one line, "PDF \u00b7 HTML \u00b7 PNG", whole kinds
// giving way from the end to a tail of "2 more" where they don't all fit
// (VKCF3Z, 2026-11-08: a list alone in a value is a dotted run).
function opensFact(kinds, named, said) {
    return {
        label: named("opens"),
        value: kinds.join(DOT),
        valueItems: kinds.map(kind => ({ said: kind })),
        tail: said.more ?? ""
    };
}

// Its command, in the Name face as an identifier, shown whole, never
// elided: each argument an item the drawer wraps between, one too long for
// a line alone broken at its own marks, and a short last one kept beside
// the one before (VKCF3Z, 2026-11-08: "mpv" / "--player-operation-mode="
// / "pseudo-gui --" where it's narrow).
function commandFact(command, named) {
    const words = Array.from(command);
    return {
        label: named("command"),
        value: words.join(" "),
        valueItems: words.map(word => ({ said: word, whole: true })),
        between: " ",
        identifies: true
    };
}

// A list alone in a value, as its keywords: a dotted run, each whole.
function runFact(label, items) {
    const each = Array.from(items ?? []).filter(item => item !== "");
    return { label, value: each.join(DOT), valueItems: each.map(item => ({ said: item })) };
}

// A program's facts, as { label, value }, in the drawer's order, what it
// doesn't say left out: what moves most first and identifiers last
// (VKCF3Z, 2026-11-08), the entry's own from the programs daemon's
// record of it (`record`), its package from the store path its command
// runs from, its version with every digit. Its usage, { processor, memory, count }, is what
// its windows' processes and all they started use, which the machine
// daemon says. While it's `asking`, those three hold their places with no
// value yet, so they don't push in above the rest once it answers; with
// no window to ask of, they're left out.
function facts(known, named, said) {
    const used = known.usage ?? null;
    const record = known.record ?? null;
    const opened = opens(record);
    const pack = record?.package ?? null;
    const startsIn = String(record?.starts_in ?? "");
    const entryFile = String(record?.entry_path ?? "");
    const own = id => value => ({ label: named(id), value: String(value ?? "") });
    const figure = (id, say) => (used ? own(id)(say(used))
        : { label: named(id), value: known.asking ? null : "" });
    return [
        figure("processor", one => percent(one.processor, known.marks)),
        figure("memory", one => memory(one, said)),
        figure("processes", one => String(one.count)),
        own("playing")(known.playing ?? "")
    ].concat((known.given ?? []).map(fact => ({ label: fact.label,
        value: String(fact.value ?? "") })), [
        opensFact(opened.kinds, named, said),
        own("handles")(opened.schemes.join(DOT)),
        own("categories")(categories(known.categories)),
        runFact(named("keywords"), record?.keywords),
        Object.assign(own("starts-in")(startsIn),
            startsIn === "" ? {} : { valueItems: [{ said: startsIn, whole: true }] }),
        // Said only where its entry asks for a terminal, as most don't.
        own("runs-in")(record?.terminal === true ? said.terminal ?? "" : ""),
        own("version")(pack?.version ?? ""),
        own("package")(pack?.name ?? ""),
        commandFact(known.command ?? [], named),
        // Its window class, where it isn't the entry's id, as it most
        // often is: the same word twice says nothing.
        Object.assign(own("window-class")(record?.window_class === known.id ? ""
            : record?.window_class ?? ""), { identifies: true }),
        Object.assign(own("desktop-id")(known.id ?? ""), { identifies: true }),
        // The file its entry was read from, which says whose data folder
        // won where several hold one of that id.
        Object.assign(own("entry-file")(entryFile), { identifies: true,
            valueItems: [{ said: entryFile, whole: true }] }),
        Object.assign(own("store-path")(pack?.store_path ?? ""), { identifies: true,
            valueItems: [{ said: pack?.store_path ?? "", whole: true }] })
    ]).filter(fact => fact.value !== "");
}

// How long a program's drawer or acts may take making before what each
// part spent is said in the journal. Most take 4 to 17 ms, and the bar from
// a key to the screen is 50, which five of 26 drawers' making alone missed
// in Joel's journal, with nothing saying which part was dear (2026-11-13).
const SAID_FROM = 25;

// The answer of `work`, its time on `clock` (ms) kept in `parts` by its
// name, { name, ms }, summed with the part of that name already kept: a
// provider asked twice in one making is one part. With no parts, it's
// only run, since nothing is timing a making.
function timed(parts, name, clock, work) {
    if (!parts) {
        return work();
    }
    const began = clock();
    const answer = work();
    const ms = clock() - began;
    const kept = parts.find(part => part.name === name);
    if (kept) {
        kept.ms += ms;
    } else {
        parts.push({ name, ms });
    }
    return answer;
}

// What a making that took `took` ms spent, by its parts, the dearest first
// and what no part holds as "the rest", each to the millisecond and those
// under one unsaid: "slack's drawer made in 90 ms: kitty 72, the rest 14".
// Empty for one under SAID_FROM, which Joel never feels.
function makingSaid(what, took, parts) {
    if (took < SAID_FROM) {
        return "";
    }
    const rest = took - parts.reduce((sum, part) => sum + part.ms, 0);
    const each = parts.concat([{ name: "the rest", ms: rest }])
        .map((part, at) => ({ name: part.name, ms: Math.round(part.ms), at }))
        .filter(part => part.ms >= 1)
        .sort((one, other) => other.ms - one.ms || one.at - other.at);
    return `${what} made in ${Math.round(took)} ms: `
        + each.map(part => `${part.name} ${part.ms}`).join(", ");
}

// A program's drawer: { eyebrow, title, facts, body }. What it is is said
// in our word, "program", its desktop entry's generic name staying in its
// row's note (VKCF3Z, 2026-11-08).
function drawer(known, named, said) {
    return {
        eyebrow: eyebrow(known.places ?? [], said.program ?? "", said),
        title: known.name,
        facts: facts(known, named, said),
        body: String(known.comment ?? "").trim()
    };
}
