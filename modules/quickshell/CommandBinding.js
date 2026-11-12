// What a command line binds to, apart from the shell so tests/command-line.qml
// holds it: the rows a range or :g names, worked out from the rows as they
// stood when : was pressed, and each bound thing's own act (VKCF3Z's spec,
// The Command Line: a range binds to the things in it, not their places).
.pragma library
.import "Commands.js" as Commands
.import "Ex.js" as Ex
.import "Operators.js" as Operators

// The rows a line Return read names (Commands.ran), as indexes from 0, given
// where things stood as : was pressed, `at` = { names, cursor, marks }: the
// cursor row with no range, a range's rows (Ex.resolve), and of those, for
// :g, the rows whose names hold its pattern, or for :v the rest. A pattern
// with no capital ignores case, as Vim's smartcase has it. { rows } or
// { error } in Vim's words.
const capital = /[A-Z]/;

function bound(read, at) {
    const range = read.range ?? null;
    const rows = Ex.resolve(range, { cursor: at.cursor, count: at.names.length,
        marks: at.marks });
    if (rows.error !== undefined || !read.global) {
        return rows;
    }
    const { pattern, invert } = read.global;
    const cased = capital.test(pattern);
    const wanted = cased ? pattern : pattern.toLowerCase();
    const holds = row => {
        const name = String(at.names[row] ?? "");
        return (cased ? name : name.toLowerCase()).includes(wanted);
    };
    return { rows: rows.rows.filter(row => holds(row) !== invert) };
}

// What a line asks before it runs, once, by the rule d asks by
// (Operators.asks), so d, the line and the menu are one, given the acts
// found among the things it's bound to, how the line stands, { all, bang },
// `all` how many things and `bang` whether a ! was typed, and the
// language's commands (Options.commands): "some" where only some have
// it, ! or not, so a range never acts on fewer than it named unasked; else,
// where the language says it changes, never :set (Commands.neverAsks), and
// d would ask, as it does unless each act says how it's put back (its
// undo) or it's guarded and alone, "one" or "many", unless the ! says
// don't; else nothing. None having it says so.
function asking(entry, acts, line, commands) {
    const { all, bang } = line;
    const found = acts.length;
    if (found === 0) {
        return { ask: "", none: true };
    }
    if (found < all) {
        return { ask: "some" };
    }
    const changes = commands[entry.command]?.changes === true
        && !Commands.neverAsks(entry.command);
    const asks = changes && Operators.asks(acts.map(act => ({ command: entry.command,
        undo: act.undo, leaves: act.leaves })), commands);
    if (!asks || bang) {
        return { ask: "" };
    }
    return { ask: found > 1 ? "many" : "one" };
}

// The act a line names among a thing's acts, so a line bound to several runs
// each one's own: by its words' id (words.json) where the line's act has
// words, else by its command and title; undefined where it has none.
function actFor(entry, acts, wordsOf) {
    if (entry.named) {
        return acts.find(act => (wordsOf(act)?.id ?? "") === entry.named);
    }
    return acts.find(act => Commands.commandOf(act) === entry.command
        && String(act.name ?? "") === entry.title);
}

// The options :setl sets on each thing it's bound to, as Vim's :setlocal
// sets the current buffer's rather than the global one: a row's own
// `options`, each as a provider's setOptions are. The one thing under the
// cursor, an act's row with none, or no row at all, sets the nearest of
// `around`, the things it's inside, outermost first, that has some, as
// Muted inside a program sets the program's; over a range each thing sets
// its own. A thing with none has none, so :setl there says E518, never
// setting the machine's.
function localOptions(things, around) {
    if (things.length > 1) {
        return things.map(thing => thing?.options ?? {});
    }
    const nearest = things.concat([...around].reverse())
        .find(thing => thing?.options !== undefined);
    return [nearest?.options ?? {}];
}

// An act's words for the line (words.json's command, argument, bang and
// range), by its key and the state its words are in (its wordState), `of`
// reading words.json's entry for an id in a state, as Words.of does: a
// verb's id, or one made from it for each of several, as verb:sound-port-2 or
// verb:on:waiting:go:<id>, which reads the verb's own; or a row that's an
// act itself, as session:Lock names :lockscreen, the step it asks first,
// as session:Power Off:sure, reading its row's. A state's words name the
// command for that state, as Wi-Fi On's :set wifi, since the line says
// what runs, never a toggle. A row keyed so it never walks back to its
// words, as a display's Warmth is displays:eDP-1:warmth, names them by their
// id, its wordsId. Where no words name one, the act is named by its verb
// (Commands.commandOf).
function wordsFor(act, of) {
    let id = String(act?.wordsId ?? act?.key ?? "");
    while (id !== "") {
        const words = of(id, act.wordState ?? "");
        if (words.command !== undefined) {
            return words;
        }
        id = id.slice(0, Math.max(id.lastIndexOf(":"), id.lastIndexOf("-"), 0));
    }
    return {};
}

// Whether a line typed where words are asked for, such as a session's talk
// line, is a command to leave it rather than words to give: a colon, then
// what the command line reads as :quit (`table` the key language's
// commands), as :q leaves a page. Joel's vi keys make :q typed alone mean
// leave (2026-11-11, when it went to a session as his words), and words
// that only hold it are given as typed.
function leaves(words, table) {
    const typed = String(words ?? "").trim();
    return typed.startsWith(":")
        && Commands.ran(typed.slice(1), Commands.listed({ table })).entry?.kind === "quit";
}
