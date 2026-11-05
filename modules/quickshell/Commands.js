// The launcher's command line, the : line, as Vim's command line after a
// colon, apart from drawing it (CommandLine.qml), so tests/command-line.qml
// holds it in Qt's own engine: what its menu lists, how typing narrows it,
// what Tab and the arrows choose and write in, and what Return runs
// (VKCF3Z's spec, The Command Line). The menu is his completion menu over
// the line, as his cmp shows one: the cursor row's acts, Return's first,
// then the surface's commands, then what every surface has.
.pragma library

// A line's command as the menu writes it: its notation, then the argument
// that tells an act apart from another of its name, as :new . from :new.
function called(entry) {
    const name = notation(entry.command, entry.short);
    return (entry.argument ?? "") === "" ? name : `${name} ${entry.argument}`;
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
// command it runs. `from` gives them: { acts, surface, table, said, row },
// `said` the words for the line's own commands, by command, and `row` the
// cursor row's number, counted from 1, which :{n}'s line writes. A name
// listed twice is listed once, the first kept, so a name runs one thing.
function listed(from) {
    const table = from.table ?? {};
    const said = from.said ?? {};
    const short = command => table[command]?.short ?? command;
    // What asks first is what changes and can't be undone, the language
    // leaving undoable out wherever nothing changes; a command it doesn't
    // name asks nothing.
    const asks = command => table[command]?.changes === true
        && table[command]?.undoable !== true;
    const seen = new Set();
    const menu = [];
    // Each line's title is lowered once, as it's listed, a space before it,
    // so narrowing as a key is typed only looks for a word's start in it.
    const add = entry => {
        if (entry.command === "" || seen.has(entry.command)) {
            return;
        }
        seen.add(entry.command);
        entry.lowered = ` ${entry.title.toLowerCase()}`;
        menu.push(entry);
    };
    // An act takes a ! only where it would ask first, its ! saying don't.
    (from.acts ?? []).forEach((act, at) => {
        const command = commandOf(act);
        const undoable = !asks(command);
        add({ command, short: short(command), title: String(act.name ?? ""),
            key: keyOf(act, at === 0), kind: "act", act, bang: !undoable, takes: false,
            undoable });
    });
    for (const own of from.surface ?? []) {
        add({ command: own.command, short: own.short ?? short(own.command),
            title: own.title ?? "", key: own.key ?? "", kind: "surface", own,
            bang: own.bang !== false, takes: true, undoable: own.undoable !== false });
    }
    add({ command: "quit", short: short("quit"), title: said.quit ?? "", key: "ZZ",
        kind: "quit", bang: true, takes: false, undoable: true });
    add({ command: "help", short: short("help"), title: said.help ?? "", key: "F1",
        kind: "help", bang: true, takes: true, undoable: true });
    add({ command: "set", short: short("set"), title: said.set ?? "", key: "",
        kind: "set", bang: false, takes: true, undoable: true });
    add({ command: "{n}", short: "{n}", title: said.row ?? "", key: "", kind: "row",
        bang: false, takes: false, undoable: true, row: from.row ?? 1 });
    return menu;
}

// What's typed, read as Vim's command line reads it: the colons and spaces
// before it passed over, then a number, or a name and a ! after it, then
// what follows, { line, name, bang, arg, rest, spaced }, arg what follows
// trimmed and rest as typed, spaced saying whether a space came after the
// name.
const leading = /^[\s:]*/;
const counting = /^[0-9$]/;
const number = /^[0-9]+$/;
// A time, as a player goes to one: :12:30, or :1:02:03.
const time = /^[0-9]+(:[0-9]+){1,2}$/;
const named = /^([A-Za-z]+|\$|[0-9]+)(!?)(\s*)([\s\S]*)$/;

function parts(typed) {
    const line = typed.replace(leading, "");
    const found = line.match(named);
    if (!found) {
        return { line, name: "", bang: false, arg: "", rest: "", spaced: false };
    }
    return { line, name: found[1], bang: found[2] === "!", arg: found[4].trim(),
        rest: found[4], spaced: found[3] !== "" };
}

// Whether a word names a command, as Vim's do: its full name, or its short
// form or any longer start of its name. Case is the command's own.
function names(word, entry) {
    if (entry.kind === "row") {
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
function narrowed(menu, typed, argued) {
    const read = parts(typed);
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
    const args = read.spaced && own.length > 0 && own[0].takes && argued
        ? argued(own[0], read.rest) : null;
    if (args === null) {
        return own;
    }
    const before = typed.slice(0, typed.length - lastWord(typed).length);
    return args.map(one => ({ command: one.value, short: one.value, title: one.note ?? "",
        key: "", kind: "argument", line: before + one.value }));
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
    if (entry.kind === "argument") {
        return entry.line;
    }
    const read = parts(typed);
    const bang = read.bang && entry.bang ? "!" : "";
    return `${entry.command}${bang}${read.arg === "" ? "" : ` ${read.arg}`}`;
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
    if (read.line.trim() === "") {
        return { error: "" };
    }
    if (number.test(read.name) && !read.bang && read.arg === "") {
        return { go: Number(read.name) };
    }
    if (read.name === "$" && !read.bang && read.arg === "") {
        return { go: "$" };
    }
    if (time.test(read.line.trim())) {
        return { go: read.line.trim() };
    }
    const entry = read.name === "" ? undefined : menu.find(one => names(read.name, one));
    if (!entry) {
        return { error: `E492: Not an editor command: ${read.line.trim()}` };
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
