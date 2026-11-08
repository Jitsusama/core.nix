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

// How many kinds of file the drawer names beside their count.
const NAMED = 3;
// A share under this is said with a place after its point.
const SMALL = 0.1;
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

// Several said as one: "1", "1 and 4", "1, 2 and 4".
function listed(each, and) {
    if (each.length < 2) {
        return each.join("");
    }
    return `${each.slice(0, -1).join(", ")} ${and} ${each[each.length - 1]}`;
}

// Where its windows are and what it is, "on 1 and 4 · web browser", from
// the numbers of the workspaces they're on; what it is alone with none.
function eyebrow(places, kind, said) {
    const numbers = Array.from(new Set(places.filter(place => place !== null
        && place !== undefined))).sort((a, b) => a - b);
    if (numbers.length === 0) {
        return kind;
    }
    const on = filled(said.on, { places: listed(numbers, said.and) });
    return kind === "" ? on : `${on} \u00b7 ${kind}`;
}

// A share of the whole machine, from 0 to 1, as a percentage, with a place
// after its point under a tenth: 4.5%, 37%.
function percent(share) {
    const places = share < SMALL ? 1 : 0;
    return `${Number((share * 100).toFixed(places))}%`;
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

// What a program opens, "42 kinds · PDF, HTML, PNG", naming the first few
// whole, and the links it handles, "http, https"; each empty for none.
function opens(types, named, said) {
    const kinds = Array.from(new Set(types.filter(type => !type.startsWith(SCHEME))
        .map(type => kindName(type, named)).filter(name => name !== "")));
    const schemes = types.filter(type => type.startsWith(SCHEME))
        .map(type => type.slice(SCHEME.length));
    const count = kinds.length === 1 ? said.kind : filled(said.kinds, { count: kinds.length });
    return {
        kinds: kinds.length === 0 ? "" : `${count} \u00b7 ${kinds.slice(0, NAMED).join(", ")}`,
        schemes: schemes.join(", ")
    };
}

// A desktop entry's categories as words, "Network, Web Browser", leaving
// out a vendor's own (X-).
function categories(each) {
    return Array.from(new Set(Array.from(each ?? []).filter(one => !one.startsWith("X-"))
        .map(one => one.replace(WORDS, "$1 $2")))).join(", ");
}

// A program's facts, as { label, value }, in the drawer's order, what it
// doesn't say left out. Its usage, { processor, memory, count }, is what
// its windows' processes and all they started use, which the machine
// daemon says; without it, those three are left out.
function facts(known, named, said) {
    const used = known.usage ?? null;
    const opened = opens(known.types ?? [], known.endings ?? {}, said);
    const own = id => value => ({ label: named(id), value: String(value ?? "") });
    return [
        own("processor")(used ? percent(used.processor) : ""),
        own("memory")(used ? FileFacts.weighed(used.memory) : ""),
        own("processes")(used ? String(used.count) : ""),
        own("playing")(known.playing ?? "")
    ].concat((known.given ?? []).map(fact => ({ label: fact.label,
        value: String(fact.value ?? "") })), [
        own("opens")(opened.kinds),
        own("handles")(opened.schemes),
        own("categories")(categories(known.categories)),
        own("command")(Array.from(known.command ?? []).join(" ")),
        own("desktop-id")(known.id ?? "")
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
