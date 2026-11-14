// The launcher's command line, the : line, as Vim's command line after a
// colon, apart from drawing it (CommandLine.qml), so tests/command-line.qml
// holds it in Qt's own engine: what its menu lists, how typing narrows it,
// what Tab and the arrows choose and write in, and what Return runs
// (VKCF3Z's spec, The Command Line). The menu is his completion menu over
// the line, as his cmp shows one: the cursor row's acts, Return's first,
// then the surface's commands, then what every surface has.
.pragma library
.import "Ex.js" as Ex

// A line's command as the menu writes it: its notation, its own bang, then
// the argument that tells an act apart from another of its name, as :new .
// from :new and :skip! from :skip.
function called(entry) {
    const name = `${entry.every ? "%" : ""}${notation(entry.command, entry.short)}`
        + `${entry.banged ? "!" : ""}`;
    return (entry.argument ?? "") === "" ? name : `${name} ${entry.argument}`;
}

// An act's words with what's in braces filled, as {name} in :set wifi={name}:
// from the act's own fill first, then the row it's on, a space escaped as
// Vim writes one in an argument. Words it can't fill aren't the act's, so
// it's named by its verb instead.
const braces = /\{(\w+)\}/g;
const aSpace = / /g;

function filled(words, act, thing) {
    let missing = false;
    const argument = String(words.argument ?? "").replace(braces, (whole, field) => {
        const value = act.fill?.[field] ?? thing?.[field];
        missing = missing || value === undefined || value === null;
        return missing ? whole : String(value).replace(aSpace, "\\ ");
    });
    return missing ? {} : Object.assign({}, words, { argument });
}

// Vim's help notation: the shortest form a command takes outside the
// brackets, the rest of its name inside, as :help writes disc[onnect].
function notation(command, short) {
    const shortest = short ?? "";
    if (shortest === "" || shortest === command || !command.startsWith(shortest)) {
        return command;
    }
    return `${shortest}[${command.slice(shortest.length)}]`;
}

// Whether a command never asks, whatever it changes: :set and :setl, a
// row's own included, as Vim's :set and :setlocal never ask (2V74KO,
// 2026-11-05).
function neverAsks(command) {
    return command === "set" || command === "setlocal";
}

// What an act is named on the line: its verb in lowercase, the word on its
// row, or else its name's first word. A verb of more words is named by its
// first, since a command is one word.
const letters = /^[a-z]+/;

function commandOf(act) {
    const verb = String(act.verb ?? "").trim().toLowerCase();
    const said = verb === "" ? String(act.name ?? "").trim().toLowerCase() : verb;
    const found = said.match(letters);
    return found ? found[0] : "";
}

// The key an act has besides the line, as the menu says it, so it teaches
// the key too: Return for the row's own act, d for its ending act and y
// for its copy (modules/keys/language.toml's do, ending and copy).
function keyOf(act, first) {
    if (first) {
        return "\u21b5";
    }
    if (act.act === "ending") {
        return "d";
    }
    return commandOf(act) === "copy" ? "y" : "";
}

// The menu: the cursor row's or the selection's acts, Return's first, the
// surface's commands, then what every surface has, :q, :help and :{n}, each
// line { command, short, title, key, kind } with the act or the surface's
// command it runs. `from` gives them: { acts, surface, table, said, row,
// called, thing }, `said` the words for the line's own commands, by
// command, `row` the cursor row's number, counted from 1, which :{n}'s line
// writes, `called` an act's words (words.json's command, argument, bang and
// range) and `thing` the row the acts are on. A name listed twice is listed
// once, the first kept, so a name runs one thing.
function listed(from) {
    const table = from.table ?? {};
    const said = from.said ?? {};
    const short = command => table[command]?.short ?? command;
    const seen = new Set();
    const menu = [];
    // Each line's title is lowered once, as it's listed, a space before it,
    // so narrowing as a key is typed only looks for a word's start in it.
    // A name and its argument are listed once.
    const add = entry => {
        entry.argument = entry.argument ?? "";
        const naming = `${called(entry)} ${entry.argument}`;
        if (entry.command === "" || seen.has(naming)) {
            return;
        }
        seen.add(naming);
        entry.lowered = ` ${entry.title.toLowerCase()}`;
        menu.push(entry);
    };
    // Earlier lines first, newest first, where :his opens them; each runs
    // as typed, never named.
    for (const line of [...(from.earlier ?? [])].reverse()) {
        add({ command: line, short: line, title: "", key: "", kind: "earlier", line,
            bang: false, takes: false, undoable: true });
    }
    for (const [at, act] of (from.acts ?? []).entries()) {
        add(actLine(act, at === 0, from));
    }
    for (const own of from.surface ?? []) {
        add(surfaceLine(own, short));
    }
    add({ command: "quit", short: short("quit"), title: said.quit ?? "", key: "ZZ",
        kind: "quit", bang: true, takes: false, undoable: true });
    add({ command: "help", short: short("help"), title: said.help ?? "", key: "F1",
        kind: "help", bang: true, takes: true, undoable: true });
    add({ command: "set", short: short("set"), title: said.set ?? "", key: "",
        kind: "set", bang: false, takes: true, undoable: true });
    // :setl, the option of what it's on, as Vim's :setlocal is the
    // buffer's, so it takes a range of things.
    add({ command: "setlocal", short: short("setlocal"), title: said.setlocal ?? "", key: "",
        kind: "setlocal", bang: false, takes: true, undoable: true,
        ranged: table.setlocal?.range === true });
    add({ command: "history", short: short("history"), title: said.history ?? "", key: "",
        kind: "history", bang: false, takes: false, undoable: true });
    add({ command: "{n}", short: "{n}", title: said.row ?? "", key: "", kind: "row",
        bang: false, takes: false, undoable: true, row: from.row ?? 1 });
    return menu;
}

// An act's line. It's named by the command its words give it, with the
// argument telling it from another of that name, or else by its verb. An
// act takes a ! where the language says it changes and the act has no
// undo, since there it could ask (Operators.asks); its ! says don't, and
// on one that costs little only runs it. A command the language doesn't
// name takes none. A bare close, its words naming no command, only
// closes the launcher, as a row with nothing to copy does: it's :quit, the
// launcher's close, and stands for the :q every surface has.
// One its words give a bang of its own is banged: the bang is its line's,
// as :skip! goes back. One its words give every row is the line's with %,
// as :%forg, which forgets them all. An act takes a range where the
// language says it takes rows.
function actLine(act, first, from) {
    const words = filled(from.called?.(act) ?? {}, act, from.thing);
    const command = words.command ?? commandOf(act);
    if (words.command === undefined && command === "close") {
        return { command: "quit", short: from.table?.quit?.short ?? "quit",
            title: String(act.name ?? ""), key: keyOf(act, first), kind: "quit",
            bang: true, takes: false, undoable: true };
    }
    const flags = from.table?.[command] ?? {};
    const undoable = flags.changes !== true || typeof act.undo === "function";
    const banged = words.bang === true;
    return { command, argument: String(words.argument ?? ""), short: flags.short ?? command,
        title: String(act.name ?? ""), key: keyOf(act, first), kind: "act", act,
        bang: banged || (!undoable && !neverAsks(command)), banged, takes: false, undoable,
        guarded: flags.guarded === true, ranged: flags.range === true,
        every: words.range === "%", named: String(words.id ?? "") };
}

// A surface's own command's line, as a viewer's :rotate: it takes what
// follows it, and a ! and a range where it says so.
function surfaceLine(own, short) {
    return { command: own.command, short: own.short ?? short(own.command),
        title: own.title ?? "", key: own.key ?? "", kind: "surface", own,
        bang: own.bang !== false, takes: true, undoable: own.undoable !== false,
        ranged: own.ranged === true };
}

// What's typed, read as Vim's command line reads it: the colons and spaces
// before it passed over, a range (Ex.js), then a number, or a name and a !
// after it, then what follows, { said, range, ranging, line, name, bang,
// arg, rest, spaced }: said all of it, range the range read or null and
// ranging as typed, line what follows it, arg what follows the name
// trimmed and rest as typed, spaced saying whether a space came after the
// name. A number or $ alone is :{n}'s, not a range.
const leading = /^[\s:]*/;
const counting = /^[0-9$]/;
const number = /^[0-9]+$/;
// A time, as a player goes to one: :12:30, or :1:02:03.
const time = /^[0-9]+(:[0-9]+){1,2}$/;
const named = /^([A-Za-z]+|\$|[0-9]+)(!?)(\s*)([\s\S]*)$/;
const rowAlone = /^([0-9]+|\$)\s*$/;
const spacesAhead = /^\s*/;

function parts(typed) {
    const said = typed.replace(leading, "");
    const read = rowAlone.test(said) || time.test(said.trim()) ? { range: null, taken: 0 }
        : Ex.rangeOf(said);
    const line = said.slice(read.taken).replace(spacesAhead, "");
    const ranging = said.slice(0, said.length - line.length);
    const found = line.match(named);
    if (!found) {
        return { said, range: read.range, ranging, line, name: "", bang: false, arg: "",
            rest: "", spaced: false };
    }
    return { said, range: read.range, ranging, line, name: found[1], bang: found[2] === "!",
        arg: found[4].trim(), rest: found[4], spaced: found[3] !== "" };
}

// Whether a line can take the range typed: where every row is, one of its
// own made for every row first, as :%forg; else one that takes rows.
function rangedFor(entry, range) {
    return range === "%" && entry.every ? true : entry.ranged === true && !entry.every;
}

// Whether a word names a command, as Vim's do: its full name, or its short
// form or any longer start of its name. Case is the command's own.
function names(word, entry) {
    if (entry.kind === "row" || entry.kind === "earlier") {
        return false;
    }
    const short = entry.short ?? entry.command;
    return word === entry.command
        || (word.length >= short.length && entry.command.startsWith(word)
            && word.startsWith(short));
}

// The menu narrowed to what's typed: by the command's start, then by a word
// of the act's that starts as typed, each line once, case ignored as his
// ignorecase has it. A number shows :{n}'s line, and a name with a ! after
// it shows its own line alone, as does a space where `argued` (entry, arg)
// says it has no arguments (null); where it has, they're its lines, each
// written in place of the word being typed.
function narrowed(whole, typed, argued) {
    const read = parts(typed);
    // After a range, only what takes it.
    const menu = read.range === null ? whole : whole.filter(one => rangedFor(one, read.range));
    if (read.line === "") {
        return menu;
    }
    if (counting.test(read.name)) {
        return menu.filter(entry => entry.kind === "row");
    }
    if (read.name === "") {
        return [];
    }
    if (read.spaced || read.bang) {
        return namedLines(menu.filter(entry => names(read.name, entry)), read, typed, argued);
    }
    const word = read.name.toLowerCase();
    const starting = [];
    const worded = [];
    for (const entry of menu.filter(one => one.kind !== "row")) {
        if (entry.command.startsWith(word)) {
            starting.push(entry);
        } else if (entry.lowered.includes(` ${word}`) || entry.lowered.includes(`-${word}`)) {
            worded.push(entry);
        }
    }
    return starting.concat(worded);
}

// The lines for a command named: its own line, or after a space the
// arguments `argued` gives it, each written in place of the word typed.
function namedLines(own, read, typed, argued) {
    const taker = own.find(one => one.takes);
    const args = read.spaced && taker && argued ? argued(taker, read.rest) : null;
    if (args === null) {
        return own.filter(one => one.takes || one.argument.startsWith(read.arg)
            || (read.arg === "" && one.argument === ""));
    }
    const before = typed.slice(0, typed.length - lastWord(typed).length);
    // An argument read as a row, as :filter's are, gives its count, its mark and its hue.
    return args.map(one => ({ command: one.value, short: one.value, title: one.note ?? "",
        key: one.side ?? "", glyph: one.glyph ?? "", hue: one.hue ?? "", kind: "argument",
        line: before + one.value }));
}

// How choosing stands: what was typed, the line chosen (-1 for none) and
// what the line holds, which is what was typed until a line is chosen.
function typing(typed) {
    return { typed, at: -1, line: typed };
}

// What choosing a line writes in: its command, keeping a ! and what was
// typed after the name; :{n}'s line writes the cursor row's number.
function written(entry, typed) {
    if (entry.kind === "row") {
        return String(entry.row);
    }
    if (entry.kind === "argument" || entry.kind === "earlier") {
        return entry.line;
    }
    const read = parts(typed);
    const bang = entry.banged || (read.bang && entry.bang) ? "!" : "";
    const arg = entry.argument === "" ? read.arg : entry.argument;
    const range = entry.every && read.ranging === "" ? "%" : read.ranging;
    return `${range}${entry.command}${bang}${arg === "" ? "" : ` ${arg}`}`;
}

// Tab, Shift+Tab, the arrows, Ctrl+N and Ctrl+P: the next line or the one
// before, writing it in, coming round through what was typed, as his cmp
// and Vim's wildmenu do.
function chosen(choosing, step, menu) {
    if (menu.length === 0) {
        return typing(choosing.typed);
    }
    const places = menu.length + 1;
    const at = ((choosing.at + 1 + step) % places + places) % places - 1;
    return { typed: choosing.typed, at, line: at < 0 ? choosing.typed
        : written(menu[at], choosing.typed) };
}

// Ctrl+E: the menu put away and what was typed back, as Vim's popup menu's
// Ctrl+E goes back to what was there before choosing.
function dropped(choosing) {
    return typing(choosing.typed);
}

// Ctrl+Y: what's chosen taken as if typed.
function taken(choosing) {
    return typing(choosing.line);
}

// Where the word before the caret starts, as Ctrl+W deletes back to on
// Vim's command line: past the spaces before it, then a run of a word's
// letters (his iskeyword's), or else a run of anything else but spaces.
const wordLetter = /[0-9A-Za-z_\u00c0-\u024f]/;
const space = /\s/;

function wordStart(text, at) {
    let start = at;
    while (start > 0 && space.test(text[start - 1])) {
        start -= 1;
    }
    if (start === 0) {
        return 0;
    }
    const kind = wordLetter.test(text[start - 1]);
    while (start > 0 && !space.test(text[start - 1])
        && wordLetter.test(text[start - 1]) === kind) {
        start -= 1;
    }
    return start;
}

// What Return runs: exactly what's typed, never what the menu shows. A name
// that's no command says E492 with the whole line, as Vim does; a ! where
// it isn't taken, E477; words after a command that takes none, E488. A
// number goes to that row and $ to the last, as they go to a line, and a
// time, 12:30, goes there in a player, its go given it as it's typed.
// { entry, bang, arg }, { go }, or { error }, which is "" for nothing typed.
function ran(typed, menu) {
    const read = parts(typed);
    if (read.said.trim() === "") {
        return { error: "" };
    }
    if (number.test(read.name) && !read.bang && read.arg === "") {
        return { go: Number(read.name) };
    }
    if (read.name === "$" && !read.bang && read.arg === "") {
        return { go: "$" };
    }
    // A time is read whole, before any range, as 12:30 is no range.
    if (time.test(read.said.trim())) {
        return { go: read.said.trim() };
    }
    if (read.name === "" && read.range !== null && read.line.trim() === "") {
        return { error: "" };
    }
    const everywhere = read.name === "" ? undefined
        : Ex.globals.find(one => Ex.names(read.name, one));
    if (everywhere) {
        return ranGlobally(read, everywhere, menu);
    }
    const matching = read.name === "" ? [] : menu.filter(one => names(read.name, one));
    if (matching.length === 0) {
        return { error: `E492: Not an editor command: ${read.said.trim()}` };
    }
    return read.range === null ? ranNamed(matching.filter(one => !one.every), read)
        : ranRanged(matching, read);
}

// A command given a range: one of its lines made for every row runs for
// :%, its own act acting on them all; else one that takes rows runs on
// those the range names, bound to things by the launcher; else Vim's E481.
function ranRanged(matching, read) {
    const every = read.range === "%" ? matching.filter(one => one.every) : [];
    if (every.length > 0) {
        return Object.assign(ranNamed(every, read), { range: null });
    }
    const taking = matching.filter(one => rangedFor(one, read.range));
    if (taking.length === 0) {
        return { error: "E481: No range allowed" };
    }
    const running = ranNamed(taking, read);
    return running.error === undefined ? Object.assign(running, { range: read.range }) : running;
}

// The earlier lines with one more remembered, as Vim's history keeps them
// for :his: once each, the newest last, fifty at most.
const kept = 50;

function remembered(earlier, line) {
    const said = line.trim();
    if (said === "") {
        return earlier;
    }
    return earlier.filter(one => one !== said).concat([said]).slice(-kept);
}

// :g/pat/cmd and :v/pat/cmd, read as Ex.js reads them, then the command
// after as any other, so :g/Slack/d runs d's act on each row that finds
// Slack: { global: { pattern, invert }, range, entry, bang, arg }, every
// row where no range is typed.
function ranGlobally(read, everywhere, menu) {
    const commands = menu.map(one => ({ command: one.command, short: one.short, ranged: true }));
    const parsed = Ex.globally(read.range, everywhere.invert,
        `${read.bang ? "!" : ""}${read.rest}`, commands);
    if (parsed.error !== undefined) {
        return parsed;
    }
    const inner = parsed.global.command;
    const line = `${inner.name}${inner.bang ? "!" : ""}${inner.args === "" ? "" : ` ${inner.args}`}`;
    const each = ran(line, menu);
    if (each.error !== undefined) {
        return each;
    }
    return Object.assign(each, { range: parsed.range,
        global: { pattern: parsed.global.pattern, invert: parsed.global.invert } });
}

// Of the lines a name names, one whose bang is its own runs only where a
// bang is typed, as :skip! goes back; then the act holding the argument
// typed; then the one holding none.
function ranNamed(matching, read) {
    const banged = read.bang
        ? matching.find(one => one.banged && one.argument === read.arg) : undefined;
    if (banged) {
        return { entry: banged, bang: true, arg: "" };
    }
    const plain = matching.filter(one => !one.banged);
    if (plain.length === 0) {
        return { error: `E492: Not an editor command: ${read.said.trim()}` };
    }
    const holding = plain.find(one => one.argument !== "" && one.argument === read.arg);
    return holding ? ranHolding(holding, read) : ranBare(plain, read);
}

// The act holding the argument typed, which runs ahead of the rest of its
// name, as a row's :set bluetooth+= ahead of :set.
function ranHolding(entry, read) {
    return read.bang && !entry.bang ? { error: "E477: No ! allowed" }
        : { entry, bang: read.bang, arg: "" };
}

// Else the one of its name holding no argument, which an act needing its
// own goes without.
function ranBare(matching, read) {
    const entry = matching.find(one => one.argument === "");
    if (!entry) {
        return { error: read.arg === "" ? "E471: Argument required"
            : `E488: Trailing characters: ${read.arg}` };
    }
    if (read.bang && !entry.bang) {
        return { error: "E477: No ! allowed" };
    }
    if (read.arg !== "" && !entry.takes) {
        return { error: `E488: Trailing characters: ${read.arg}` };
    }
    return { entry, bang: read.bang, arg: read.arg };
}

// The word being typed at the end of what follows a name: from the last
// space Vim doesn't escape, "" just after one.
function lastWord(arg) {
    let start = 0;
    let at = 0;
    while (at < arg.length) {
        if (arg[at] === "\\") {
            at += 1;
        } else if (space.test(arg[at])) {
            start = at + 1;
        }
        at += 1;
    }
    return arg.slice(start);
}

// A surface command's own arguments, a list or what it gives for what's
// typed after it, each { value, note } or a plain word, narrowed to those
// starting as the word typed does; null for one that has none.
function ownArguments(entry, arg) {
    const args = entry.own?.args;
    if (args === undefined) {
        return null;
    }
    const given = typeof args === "function" ? args(arg) : args;
    const word = lastWord(arg).toLowerCase();
    return given.map(one => typeof one === "string" ? { value: one, note: "" } : one)
        .filter(one => one.value.toLowerCase().startsWith(word));
}
