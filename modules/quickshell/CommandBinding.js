// What a command line binds to, apart from the shell so tests/command-line.qml
// holds it: the rows a range or :g names, worked out from the rows as they
// stood when : was pressed, and each bound thing's own act (VKCF3Z's spec,
// The Command Line: a range binds to the things in it, not their places).
.pragma library
.import "Commands.js" as Commands
.import "Ex.js" as Ex

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
