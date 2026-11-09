// What a program's drawer says (Applications.qml): where its windows are and
// what it is above its name; what moves first, the share of the processors
// its processes use, the memory they hold and how many there are, and what
// it plays; then what other providers say of it; then what describes it,
// the kinds of file it opens, the links it handles and its categories; and
// what identifies it last, its command and its desktop entry's id (VKCF3Z,
// 2026-11-07). Its desktop entry's comment is its words. Plain JavaScript,
// so tests/program-drawer.qml holds it in Qt's own engine.
.pragma library
.import "../FileFacts.js" as FileFacts
.import "Numbers.js" as Numbers

// Parts of a value, and a list alone in one, run on a dot.
const DOT = " \u00b7 ";
// A file's kind by the name its files end in, a glob as `*.pdf`.
const ENDING = /^\*\.([a-z0-9]+)$/i;
// A link's scheme a program handles, as its desktop entry lists it.
const SCHEME = "x-scheme-handler/";
// A desktop entry's group's heading, and the field listing its kinds.
const GROUP = /^\[(.*)\]$/;
const MIME_TYPE = /^MimeType\s*=\s*(.*)$/;
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
// alone rather than twice.
function usage(trees) {
    const known = trees.filter(tree => tree !== null && tree !== undefined);
    const within = (tree, other) => other !== tree && other.pid !== tree.pid
        && Array.from(other.pids ?? []).includes(tree.pid);
    const said = known.filter(tree => !known.some(other => within(tree, other)));
    if (said.length === 0) {
        return null;
    }
    const sum = field => said.reduce((all, tree) => all + (Number(tree[field]) || 0), 0);
    return {
        processor: Math.max(0, Math.min(1, sum("share_ratio"))),
        memory: sum("resident_b"),
        count: sum("processes")
    };
}

// The kinds of file a desktop entry says it opens, from its file's text:
// its own group's MimeType, in its order.
function mimeTypes(text) {
    let group = "";
    for (const line of String(text ?? "").split("\n")) {
        const heading = GROUP.exec(line.trim());
        const field = MIME_TYPE.exec(line.trim());
        if (heading !== null) {
            group = heading[1];
        } else if (field !== null && group === "Desktop Entry") {
            return field[1].split(";").map(type => type.trim()).filter(type => type !== "");
        }
    }
    return [];
}

// Each kind's ending, by its MIME type, from shared-mime-info's globs2,
// its lines `50:application/pdf:*.pdf`: the weightiest plain ending each
// has, the first of equal weight.
function endings(text) {
    const best = {};
    for (const line of String(text ?? "").split("\n")) {
        const [weight, type, glob] = line.split(":");
        const ending = ENDING.exec(glob ?? "");
        if (ending !== null && (best[type] === undefined || Number(weight) > best[type].weight)) {
            best[type] = { weight: Number(weight), name: ending[1].toUpperCase() };
        }
    }
    const names = {};
    for (const type of Object.keys(best)) {
        names[type] = best[type].name;
    }
    return names;
}

// A kind by the ending its files have, PDF, or where none is known by its
// own name, without the x- an unregistered one carries.
function kindName(type, named) {
    if (named[type] !== undefined) {
        return named[type];
    }
    const own = String(type.split("/")[1] ?? "").replace(UNREGISTERED, "").split("+")[0];
    return own.length <= 4 ? own.toUpperCase() : own;
}

// The kinds of file a program opens, ["PDF", "HTML", "PNG"], each once, and
// the links it handles, ["http", "https"].
function opens(types, named) {
    return {
        kinds: Array.from(new Set(types.filter(type => !type.startsWith(SCHEME))
            .map(type => kindName(type, named)).filter(name => name !== ""))),
        schemes: types.filter(type => type.startsWith(SCHEME))
            .map(type => type.slice(SCHEME.length))
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

// Its command, in the Name face as an identifier, each argument an item
// the drawer wraps between, never inside, one naming a path giving way
// from its middle where alone it's too wide (VKCF3Z, 2026-11-08: nothing
// draws past its frame).
function commandFact(command, named) {
    const words = Array.from(command);
    return {
        label: named("command"),
        value: words.join(" "),
        valueItems: words.map(word => (word.includes("/") ? { said: word, path: word }
            : { said: word })),
        between: " ",
        identifies: true
    };
}

// A program's facts, as { label, value }, in the drawer's order, what it
// doesn't say left out. Its usage, { processor, memory, count }, is what
// its windows' processes and all they started use, which the machine
// daemon says. While it's `asking`, those three hold their places with no
// value yet, so they don't push in above the rest once it answers; with
// no window to ask of, they're left out.
function facts(known, named, said) {
    const used = known.usage ?? null;
    const opened = opens(known.types ?? [], known.endings ?? {});
    const own = id => value => ({ label: named(id), value: String(value ?? "") });
    const figure = (id, say) => (used ? own(id)(say(used))
        : { label: named(id), value: known.asking ? null : "" });
    return [
        figure("processor", one => percent(one.processor, known.marks)),
        figure("memory", one => FileFacts.weighed(one.memory)),
        figure("processes", one => String(one.count)),
        own("playing")(known.playing ?? "")
    ].concat((known.given ?? []).map(fact => ({ label: fact.label,
        value: String(fact.value ?? "") })), [
        opensFact(opened.kinds, named, said),
        own("handles")(opened.schemes.join(DOT)),
        own("categories")(categories(known.categories)),
        commandFact(known.command ?? [], named),
        Object.assign(own("desktop-id")(known.id ?? ""), { identifies: true })
    ]).filter(fact => fact.value !== "");
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
