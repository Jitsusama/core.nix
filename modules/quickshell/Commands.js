// The launcher's command line, the : line, as Vim's command line after a
// colon, apart from drawing it (CommandLine.qml), so tests/command-line.qml
// holds it in Qt's own engine: what its menu lists, how typing narrows it,
// what Tab and the arrows choose and write in, and what Return runs
// (VKCF3Z's spec, The Command Line). The menu is his completion menu over
// the line, as his cmp shows one: the cursor row's acts, Return's first,
// then the surface's commands, then what every surface has.
.pragma library

// Each command's fixed short form and what it is, by its full name, as the
// key language will say them (modules/keys/language.toml's [acts.<name>]
// command, short, changes, undoable and range, reaching the shell as
// Options.commands): until it does, they're said here, in the same shape.
// A short form is fixed, never worked out from what else is listed, so
// :disc never changes meaning as a command is added. Vim's own keep Vim's
// (:help ex-cmd-index); a name Neovim gives another meaning isn't ours to
// take, so forgetting is :forg (Vim's :fo is :fold).
const standIn = {
    quit: { short: "q", changes: false, undoable: true, range: false },
    help: { short: "h", changes: false, undoable: true, range: false },
    set: { short: "se", changes: true, undoable: true, range: false },
    sort: { short: "sor", changes: false, undoable: true, range: true },
    global: { short: "g", changes: true, undoable: true, range: true },
    vglobal: { short: "v", changes: true, undoable: true, range: true },
    delete: { short: "d", changes: true, undoable: true, range: true },
    yank: { short: "y", changes: false, undoable: true, range: true },
    undo: { short: "u", changes: true, undoable: true, range: false },
    redo: { short: "red", changes: true, undoable: true, range: false },
    nohlsearch: { short: "noh", changes: false, undoable: true, range: false },
    edit: { short: "e", changes: false, undoable: true, range: false },
    buffer: { short: "b", changes: false, undoable: true, range: false },
    split: { short: "sp", changes: false, undoable: true, range: false },
    move: { short: "m", changes: true, undoable: true, range: true },
    history: { short: "his", changes: false, undoable: true, range: false },
    close: { short: "clo", changes: true, undoable: false, range: true },
    disconnect: { short: "disc", changes: true, undoable: true, range: true },
    forget: { short: "forg", changes: true, undoable: false, range: true },
    suspend: { short: "sus", changes: true, undoable: false, range: false }
};

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
    const flags = command => table[command] ?? {};
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
        const undoable = flags(command).undoable !== false;
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
// what follows, { line, name, bang, arg, spaced }, spaced saying whether a
// space came after the name.
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
        return { line, name: "", bang: false, arg: "", spaced: false };
    }
    return { line, name: found[1], bang: found[2] === "!", arg: found[4].trim(),
        spaced: found[3] !== "" };
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
// ignorecase has it. A number shows :{n}'s line, and a name with a space or
// a ! after it shows its own line alone.
function narrowed(menu, typed) {
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
        return menu.filter(entry => names(read.name, entry));
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

// :set's words, split at spaces as Vim splits them, a backslash keeping
// the letter after it, so :set wifi=Home\ 5G names one network.
function setWords(arg) {
    const said = [];
    let word = "";
    let escaped = false;
    for (const letter of arg) {
        if (escaped) {
            word += letter;
            escaped = false;
        } else if (letter === "\\") {
            escaped = true;
        } else if (space.test(letter)) {
            if (word !== "") {
                said.push(word);
            }
            word = "";
        } else {
            word += letter;
        }
    }
    return word === "" ? said : said.concat([word]);
}

const setWord = /^(no|inv)?([a-z]+)(!|\?|\+=|-=|=)?([\s\S]*)$/;

// One of :set's words split into what it says: no or inv, the option's
// name, how it's set and the value, or null where it names no option. An
// option whose own name starts with no or inv is that option.
function setRead(word, options) {
    const found = word.match(setWord);
    if (!found) {
        return null;
    }
    const [, said = "", called, how = "", value = ""] = found;
    const whole = said !== "" && options[said + called] !== undefined;
    const name = whole ? said + called : called;
    const option = options[name];
    const given = how === "=" || how === "+=" || how === "-=";
    if (!option || (!given && value !== "")) {
        return null;
    }
    return { prefix: whole ? "" : said, name, how, value, option };
}

// A value given an option with =, += or -=: a change, or Vim's error.
function setValue(word, read) {
    const { name, how, value, option } = read;
    const invalid = { error: `E474: Invalid argument: ${word}` };
    const numbered = option.kind === "number";
    if ((how !== "=" && !numbered) || (option.kind === "switch" && !option.takes)) {
        return invalid;
    }
    if (!numbered) {
        const choices = option.values ?? null;
        return choices !== null && !choices.includes(value) ? invalid
            : { option: name, how: "set", value };
    }
    if (!number.test(value)) {
        return { error: `E521: Number required after =: ${word}` };
    }
    const by = Number(value);
    if (how === "=" && (by < (option.least ?? 0) || by > (option.most ?? by))) {
        return invalid;
    }
    const moved = { "=": "set", "+=": "add", "-=": "subtract" };
    return { option: name, how: moved[how], value: by };
}

// One of :set's words, against the option it names: a change, or Vim's
// error for it.
function setOne(word, options) {
    const read = setRead(word, options);
    if (read === null) {
        return { error: `E518: Unknown option: ${word}` };
    }
    const { prefix, name, how } = read;
    const switches = read.option.kind === "switch";
    const flipped = prefix !== "" || how === "!";
    if (flipped && (!switches || (how !== "" && how !== "!"))) {
        return { error: `E474: Invalid argument: ${word}` };
    }
    if (flipped) {
        const off = prefix === "no" && how === "";
        return { option: name, how: off ? "off" : "toggle", value: "" };
    }
    if (how === "?" || (how === "" && !switches)) {
        return { option: name, how: "show", value: "" };
    }
    return how === "" ? { option: name, how: "on", value: "" } : setValue(word, read);
}

// What :set's words ask (:help :set), read against `options`, each an
// option's kind: a switch (on, off, or with `takes` a value, as
// :set wifi=Home-5G joins a network), a number between its least and
// most, or a choice among its values. Each word is on, off, toggle, set,
// add, subtract or show; all, or nothing after :set, shows every one.
// { changes: [{ option, how, value }] }, or { error } in Vim's words for
// the first word it can't take, so none of them changes.
function setting(arg, options) {
    const words = setWords(arg);
    if (words.length === 0) {
        return { changes: [{ option: "all", how: "show", value: "" }] };
    }
    const changes = [];
    for (const word of words) {
        const change = word === "all" ? { option: "all", how: "show", value: "" }
            : setOne(word, options);
        if (change.error !== undefined) {
            return change;
        }
        changes.push(change);
    }
    return { changes };
}

// What a change sets the option to, from where it stands now: { to }, or
// { error } for one that can't change while the shell runs (`fixed`), as
// the theme's background, which a switch of the system sets. A switch
// stands as true, false, or with `takes` the value it holds, as the
// network joined; a number is held within its least and most.
function toward(change, now, option) {
    if (option.fixed) {
        const said = change.how === "set" ? `${change.option}=${change.value}` : change.option;
        return { error: `E519: Option not supported: ${said}` };
    }
    const within = to => Math.max(option.least ?? to, Math.min(option.most ?? to, to));
    switch (change.how) {
    case "on":
        return { to: true };
    case "off":
        return { to: false };
    case "toggle":
        return { to: now === false };
    case "add":
        return { to: within(Number(now) + change.value) };
    case "subtract":
        return { to: within(Number(now) - change.value) };
    default:
        return { to: change.value };
    }
}

// :set's arguments, as a space after it lists them: the options, with
// where each stands, or after one's = the values it takes.
function setArguments(arg, options) {
    return [];
}

// A surface command's own arguments, a list or what it gives for what's
// typed after it.
function ownArguments(entry, arg) {
    return [];
}

// One option as Vim's :set volume? says it: a switch its name or its no,
// or its value where it holds one, as the network it's joined.
function shown(name, now, option) {
    if (option.kind !== "switch") {
        return `${name}=${now}`;
    }
    if (now === false) {
        return `no${name}`;
    }
    return now === true ? name : `${name}=${now}`;
}
