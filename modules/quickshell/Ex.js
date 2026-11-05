// Vim's command line, read as Vim reads it (docs/design.md, Vi's Keys): a
// range, a command's name in full or by its fixed short form, a bang, and
// what follows, or Vim's own error for a line it can't take. Each surface
// says which commands it has; this only reads them, so every surface's :
// line reads alike. Apart from the shell, so tests/ex.qml can hold it in
// Qt's own engine.
//
// A range stays unresolved until `resolve` is given the rows as they stand,
// so the caller maps the rows it names to things at once, and a row
// arriving after can't shift what the line acts on.
.pragma library

// Vim's :global and :vglobal, which every list has: a pattern, and a
// command run on each row it finds, or on each it doesn't.
const globals = [
    { command: "global", short: "g", invert: false },
    { command: "vglobal", short: "v", invert: true }
];

const letters = /^[A-Za-z]+/;
const number = /^[0-9]+/;
const offsetAt = /^([+-])([0-9]*)/;
const opening = /^[\s:]+/;
const spaces = /^\s+/;
const blank = /^\s*$/;
const space = /\s/;
// What Vim refuses to mark a pattern's ends with.
const unmarkable = /[A-Za-z0-9\\"|]/;

// Whether a name typed is the command, in full or cut no shorter than its
// short form, as Vim's forg[et] takes forg, forge and forget.
function names(typed, entry) {
    return typed.length >= entry.short.length && entry.command.startsWith(typed);
}

// Where an address starts, as Vim's . $ 12 or 'a: what it is and how much
// of the line it took, or null for a line that starts with none.
function start(line) {
    const [first] = line;
    if (first === "." || first === "$") {
        return { at: first, taken: 1 };
    }
    const digits = line.match(number);
    if (digits) {
        return { at: Number(digits[0]), taken: digits[0].length };
    }
    if (first === "'" && line.length > 1) {
        return { at: line.slice(0, 2), taken: 2 };
    }
    return null;
}

// Reads one address and the offsets after it (+2, -, ++) from the start of
// `line`: the address, and how much of the line it took, or null where
// none starts there. A bare offset counts from the cursor row, as Vim's +2
// does.
function address(line) {
    const begun = start(line);
    let taken = begun ? begun.taken : 0;
    let offset = 0;
    let found = line.slice(taken).match(offsetAt);
    while (found) {
        const by = found[2] === "" ? 1 : Number(found[2]);
        offset += found[1] === "+" ? by : -by;
        taken += found[0].length;
        found = line.slice(taken).match(offsetAt);
    }
    if (begun === null && taken === 0) {
        return null;
    }
    return { address: { at: begun ? begun.at : ".", offset }, taken };
}

function separates(letter) {
    return letter === "," || letter === ";";
}

// Reads the range at the start of `line`: null for none, "%" for every
// row, or { from, to }, each an address; and how much of the line it took.
// Vim's ,5 starts and its 3, ends at the cursor row.
function rangeOf(line) {
    if (line.startsWith("%")) {
        return { range: "%", taken: 1 };
    }
    const here = { at: ".", offset: 0 };
    const first = address(line);
    if (first === null && !separates(line[0])) {
        return { range: null, taken: 0 };
    }
    const from = first ? first.address : here;
    const taken = first ? first.taken : 0;
    if (!separates(line[taken])) {
        return { range: { from, to: from }, taken };
    }
    const second = address(line.slice(taken + 1));
    const to = second ? second.address : here;
    return { range: { from, to }, taken: taken + 1 + (second ? second.taken : 0) };
}

// Reads a command line, without its colon, against the commands a surface
// has, each { command, short, ranged }: what it asks as { range, name,
// bang, args }, where name is the command in full, or "" for a line that's
// a range alone, as :12 goes to a row; { range, global: { pattern, invert,
// command } } for :g and :v, the command read the same way; or { error }
// with Vim's own words for a line it can't take.
function parse(line, commands) {
    const said = line.replace(opening, "");
    const read = rangeOf(said);
    const after = said.slice(read.taken).replace(spaces, "");
    const [typed] = after.match(letters) ?? [""];
    const rest = after.slice(typed.length);
    if (typed === "") {
        // Nothing but a range goes to its row; nothing at all does nothing.
        return blank.test(rest) ? { range: read.range, name: "", bang: false, args: "" }
            : { error: `E492: Not an editor command: ${said}` };
    }
    const everywhere = globals.find(one => names(typed, one));
    if (everywhere) {
        return globally(read.range, everywhere.invert, rest, commands);
    }
    const entry = commands.find(one => names(typed, one));
    if (!entry) {
        return { error: `E492: Not an editor command: ${said}` };
    }
    if (read.range !== null && !entry.ranged) {
        return { error: "E481: No range allowed" };
    }
    const bang = rest.startsWith("!");
    return { range: read.range, name: entry.command, bang, args: rest.slice(bang ? 1 : 0).trim() };
}

// The pattern of :g between the mark that opens it and the next of the
// same, a backslash keeping one within it: the pattern, and the rest of
// the line after it.
function marked(line) {
    const [mark] = line;
    let pattern = "";
    let at = 1;
    while (at < line.length && line[at] !== mark) {
        if (line[at] === "\\" && line[at + 1] === mark) {
            at += 1;
        }
        pattern += line[at];
        at += 1;
    }
    return { pattern, rest: line.slice(at + 1) };
}

// :g/pat/cmd and :v/pat/cmd, or :g!, which is :v: the pattern, and the
// command after read as any other. With no range they look at every row,
// as Vim's do.
function globally(range, inverts, rest, commands) {
    const bang = rest.startsWith("!");
    const line = rest.slice(bang ? 1 : 0);
    const [mark] = line;
    if (mark === undefined || space.test(mark)) {
        return { error: "E35: No previous regular expression" };
    }
    if (unmarkable.test(mark)) {
        return { error: "E146: Regular expressions can't be delimited by letters" };
    }
    const { pattern, rest: after } = marked(line);
    if (pattern === "") {
        return { error: "E35: No previous regular expression" };
    }
    const command = parse(after, commands);
    if (command.error) {
        return command;
    }
    if (command.global) {
        return { error: "E147: Cannot do :global recursive" };
    }
    return { range: range ?? "%", global: { pattern, invert: inverts !== bang, command } };
}

// The range Vim writes for a count typed before the colon, as 3: writes
// .,.+2, and for a selection, as : in visual writes '<,'>.
function counted(count) {
    return count > 1 ? `.,.+${count - 1}` : ".";
}

const selected = "'<,'>";

// The rows a range names, as indexes from 0 in the order listed, given
// where things stand: the cursor row's index, how many rows there are, and
// the marks set, by name ('a, '<, '>), as indexes. No range is the cursor
// row, as Vim's is; a range given backwards is turned round, as Vim offers
// to. { rows } or { error } in Vim's words.
function resolve(range, at) {
    if (range === null) {
        return at.count === 0 ? { error: "E16: Invalid range" } : { rows: [at.cursor] };
    }
    if (range === "%") {
        return { rows: Array.from({ length: at.count }, (_, row) => row) };
    }
    const ends = [range.from, range.to].map(one => where(one, at));
    const missing = ends.find(end => end.error);
    if (missing) {
        return missing;
    }
    const from = Math.min(ends[0].row, ends[1].row);
    const to = Math.max(ends[0].row, ends[1].row);
    if (from < 0 || to >= at.count) {
        return { error: "E16: Invalid range" };
    }
    return { rows: Array.from({ length: to - from + 1 }, (_, step) => from + step) };
}

// The index one address names, or Vim's error for a mark not set. Rows are
// numbered from 1 as the gutter numbers them, and Vim's 0 is the first.
function where(one, at) {
    if (one.at === ".") {
        return { row: at.cursor + one.offset };
    }
    if (one.at === "$") {
        return { row: at.count - 1 + one.offset };
    }
    if (typeof one.at === "number") {
        return { row: Math.max(one.at, 1) - 1 + one.offset };
    }
    const mark = at.marks?.[one.at.slice(1)];
    return mark === undefined ? { error: "E20: Mark not set" } : { row: mark + one.offset };
}
