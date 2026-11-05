// The keys of the held notices, Vim's quickfix list (docs/design.md, Vi's
// Keys; VKCF3Z's vi-paradigm spec, Notices), as one table of key and act in
// each mode, read apart from the stack (Notifications.qml) so
// tests/notice-keys.qml can hold it in Qt's own engine. The foot's hints
// and Space ?'s list come from the same table, so neither can name a key
// that does nothing. Beside it, how the find, the command line and a
// reply's prompt are typed. Letters go by what they type, wherever they sit
// on the layout.
.pragma library
.import "LineEditing.js" as LineEditing

// The modes, each with one purpose: normal stands on a card and acts on it;
// visual chooses several; pending is an operator waiting for what it acts
// on; find is the / line narrowing the cards; command is the : line;
// insert writes a reply's words, and prompt is normal over them.
const moving = ["normal", "visual", "pending"];

// Each act, its keys, the modes it's done in (normal unless it says), and
// its kind: a motion moves the cursor, or says what an operator acts on; an
// operator waits for one; an object is what an operator or visual takes as
// a whole. A key is named as the stack names it (Notifications.qml,
// `named`): a modifier, then the letter it typed, or else its name, with
// Shift unnamed beside a letter, which says it by its case. A list of keys
// is a sequence typed one after another, as Vim's dd. `hint` is the word
// the foot gives it, and `does` what Space ? says it does, in a few words.
const table = [
    // Moving through the cards, the newest on line 1, as Vim moves through
    // lines; no arrow does anything in normal, as Joel asked (2026-11-05:
    // "remove them and rely only on vi right feeling things").
    { act: "down", keys: ["j"], modes: moving, kind: "motion", does: "down a card" },
    { act: "up", keys: ["k"], modes: moving, kind: "motion", does: "up a card" },
    { act: "first", keys: [["g", "g"]], modes: moving, kind: "motion", does: "newest, or {n}th" },
    { act: "last", keys: ["G"], modes: moving, kind: "motion", does: "oldest, or {n}th" },
    { act: "viewTop", keys: ["H"], modes: moving, kind: "motion", does: "the view's top" },
    { act: "viewMiddle", keys: ["M"], modes: moving, kind: "motion", does: "the view's middle" },
    { act: "viewBottom", keys: ["L"], modes: moving, kind: "motion", does: "the view's bottom" },
    // A list's paragraph is a run of cards from one sender.
    { act: "runOn", keys: ["}"], modes: moving, kind: "motion", does: "the next sender" },
    { act: "runBack", keys: ["{"], modes: moving, kind: "motion", does: "the sender before" },
    { act: "halfDown", keys: ["Ctrl+D"], modes: moving, kind: "motion", does: "half a view on" },
    { act: "halfUp", keys: ["Ctrl+U"], modes: moving, kind: "motion", does: "half a view back" },
    { act: "viewDown", keys: ["Ctrl+F"], modes: moving, kind: "motion", does: "a view on" },
    { act: "viewUp", keys: ["Ctrl+B"], modes: moving, kind: "motion", does: "a view back" },

    // Into the cursor card's acts and out, as his oil's l and h; Backspace
    // and - are h, as in Vim.
    { act: "into", keys: ["l"], does: "into its acts" },
    { act: "out", keys: ["h", "Backspace", "-"], does: "out of its acts" },
    // Return goes to whoever sent it, or does the act it stands on.
    { act: "open", keys: ["Return", "Enter"], hint: "go to", does: "go to its sender" },

    // Operators: d is done, marked read where the sender offers Read and
    // then forgotten; y copies what it said. c changes nothing here, so
    // it's no key, and says so.
    { act: "done", keys: ["d"], kind: "operator", hint: "done", does: "done with it" },
    { act: "copy", keys: ["y"], kind: "operator", does: "copy what it said" },
    { act: "done", keys: ["d", "x"], modes: ["visual"], kind: "operator", hint: "done",
        does: "done with them" },
    { act: "copy", keys: ["y"], modes: ["visual"], kind: "operator", hint: "copy",
        does: "copy their words" },
    // A run from one sender, as Vim's paragraph; on a list there's no gap
    // to take with it, so ap is ip.
    { act: "inRun", keys: [["i", "p"], ["a", "p"]], modes: ["pending", "visual"],
        kind: "object", does: "its sender's run" },

    { act: "repeat", keys: ["."], does: "the change again" },
    { act: "undo", keys: ["u"], does: "bring them back" },
    { act: "redo", keys: ["Ctrl+R"], does: "take them again" },

    // Choosing several, as Vim's V; on a list v is V, a card having no
    // letters to choose among.
    { act: "select", keys: ["V", "v"], does: "choose cards" },
    { act: "reselect", keys: [["g", "v"]], does: "choose them again" },
    { act: "select", keys: ["V", "v"], modes: ["visual"], does: "stop choosing" },
    { act: "otherEnd", keys: ["o"], modes: ["visual"], does: "the other end" },

    // The find, Vim's / line, narrowing the cards as it's typed; ? is the
    // same, since a find has no direction. i a I A open it on what was
    // found, the caret at its start or its end.
    { act: "find", keys: ["/", "?"], does: "find among them" },
    { act: "findStart", keys: ["i", "I"], does: "find, at its start" },
    { act: "findEnd", keys: ["a", "A"], does: "find, at its end" },
    // The command line, with a menu of every command it takes.
    { act: "command", keys: [":"], hint: "commands", does: "every command" },
    { act: "command", keys: [":"], modes: ["visual"], hint: "commands", does: "commands on them" },

    // Letting go, as Vim closes a popup; q records nothing, and says so.
    { act: "close", keys: [["Z", "Z"], ["Space", "w"]], does: "let them go" },
    { act: "quiet", keys: ["q"] },
    { act: "keys", keys: [["Space", "?"], "F1"], modes: ["normal", "visual"], hint: "keys",
        does: "these keys" },
    { act: "numbers", keys: [["Space", "u", "l"]], does: "relative numbers" },
    { act: "back", keys: ["Escape"], does: "let them go" },
    { act: "back", keys: ["Escape"], modes: ["visual"], does: "stop choosing" },
    // While Space ? lists these, the foot says how to move through them
    // and how to leave; the moves are normal's, so these only hint.
    { act: "scroll", keys: ["j k"], modes: ["listing"], hint: "scroll" },
    { act: "back", keys: ["Escape"], modes: ["listing"], hint: "back" },

    // The find line: what types is the line's, these its own.
    { act: "open", keys: ["Return", "Enter"], modes: ["find"], hint: "go to",
        does: "go to its sender" },
    { act: "choose", keys: ["Down", "Tab", "Ctrl+N"], modes: ["find", "command"],
        does: "the next" },
    { act: "chooseBack", keys: ["Up", "Shift+Tab", "Ctrl+P"], modes: ["find", "command"],
        does: "the one before" },
    { act: "take", keys: ["Ctrl+Y"], modes: ["find", "command"], does: "take the chosen" },
    { act: "normal", keys: ["Escape", "Ctrl+C"], modes: ["find"], hint: "normal",
        does: "normal, as found" },
    // The command line: Return runs exactly what's typed.
    { act: "run", keys: ["Return", "Enter"], modes: ["command"], hint: "run",
        does: "run what's typed" },
    { act: "away", keys: ["Ctrl+E"], modes: ["command"], does: "put the menu away" },
    { act: "drop", keys: ["Escape", "Ctrl+C"], modes: ["command"], hint: "drop",
        does: "drop the line" },
    // A reply's prompt: Return hands its words over, from insert or from
    // normal over them, where Vim's editing works.
    { act: "send", keys: ["Return", "Enter"], modes: ["insert", "prompt"], hint: "send",
        does: "send the reply" },
    { act: "normal", keys: ["Escape", "Ctrl+C"], modes: ["insert"], hint: "normal",
        does: "normal over them" },
    { act: "insert", keys: ["i", "a"], modes: ["prompt"], does: "insert, or after" },
    { act: "insertEnds", keys: ["I", "A"], modes: ["prompt"], does: "insert at an end" },
    { act: "left", keys: ["h", "Backspace"], modes: ["prompt"], does: "a letter left" },
    { act: "right", keys: ["l"], modes: ["prompt"], does: "a letter right" },
    { act: "lineStart", keys: ["0", "Home"], modes: ["prompt"], does: "the line's start" },
    { act: "lineEnd", keys: ["$", "End"], modes: ["prompt"], does: "the line's end" },
    { act: "wordOn", keys: ["w"], modes: ["prompt"], does: "the next word" },
    { act: "wordBack", keys: ["b"], modes: ["prompt"], does: "the word before" },
    { act: "wordEnd", keys: ["e"], modes: ["prompt"], does: "the word's end" },
    { act: "cut", keys: ["x", "Delete"], modes: ["prompt"], does: "cut a letter" },
    { act: "cutWord", keys: [["d", "w"]], modes: ["prompt"], does: "cut a word" },
    { act: "changeWord", keys: [["c", "w"]], modes: ["prompt"], does: "change a word" },
    { act: "undo", keys: ["u"], modes: ["prompt"], does: "undo" },
    { act: "command", keys: [":"], modes: ["prompt"], does: "commands; :q drops" }
];

// How long a second press of an operator done from visual is taken with it.
const wait = 300;

const digit = /^[0-9]$/;

// Where the keys stand between presses: the count typed so far, the keys of
// a sequence begun, an operator waiting with its own count, and the
// operator just done from visual, which a second press within the wait
// joins rather than starting again (Joel's "visual ... then dd").
function rest() {
    return { count: "", typed: [], operator: "", operatorAct: "", operatorCount: 0, taken: null };
}

function same(a, b) {
    return a.length === b.length && a.every((key, at) => key === b[at]);
}

function begins(sequence, typed) {
    return typed.length < sequence.length && typed.every((key, at) => key === sequence[at]);
}

function inMode(row, mode) {
    return (row.modes ?? ["normal"]).includes(mode);
}

// The row a typed sequence matches in a mode, "begun" when it could go on
// to one, or null.
function matched(typed, mode) {
    let begun = false;
    for (const row of table.filter(one => inMode(one, mode))) {
        for (const keys of row.keys) {
            const sequence = Array.isArray(keys) ? keys : [keys];
            if (same(sequence, typed)) {
                return row;
            }
            begun = begun || begins(sequence, typed);
        }
    }
    return begun ? "begun" : null;
}

// What a key does from where the keys stand, in normal or visual: the act,
// its count (0 when none was typed), and for an operator done, which it is
// and what it takes (a motion, an object, "line" for the operator pressed
// again, or "selection"); how the keys stand after it; and whether the key
// was the stack's. A key with no meaning here is the stack's all the same,
// and says so (`notHere`), since none is silent. `repeated` says it's a
// held key's repeat, which only moves: operators, ., u, Ctrl+R and Return
// act only on a fresh press. `now`, in milliseconds, tells an operator
// pressed again within the wait of one done from visual, which it joins.
// Both are in `press`, { repeated, now }.
function read(stood, key, mode, press) {
    const repeated = press?.repeated ?? false;
    const now = press?.now ?? 0;
    // A key with no name, as Shift pressed alone for a capital, changes
    // nothing, so what was begun carries on past it.
    if (key === "") {
        return { act: "", count: 0, standing: stood, ours: false };
    }
    const taken = stood.taken;
    const after = Object.assign({}, stood, { taken: null });
    if (taken !== null && !repeated && key === taken.key && now <= taken.until) {
        return { act: "", count: 0, standing: after, ours: true };
    }
    const result = reading(after, key, mode ?? "normal", now);
    if (repeated && !(result.kind === "motion" && stood.operator === "")) {
        return { act: "", count: 0, standing: stood, ours: true };
    }
    return result;
}

function reading(stood, key, mode, now) {
    const pending = stood.operator !== "";
    const counted = stood.count !== "";
    if (key === "Escape" && (counted || stood.typed.length > 0 || pending)) {
        return { act: "", count: 0, standing: rest(), ours: true };
    }
    // A count before a motion, an operator or after one, as 5j, 3dd and d3j;
    // a lone 0 counts nothing.
    if (stood.typed.length === 0 && digit.test(key) && (counted || key !== "0")) {
        return { act: "", count: 0, ours: true,
            standing: Object.assign({}, stood, { count: stood.count + key }) };
    }
    const typed = stood.typed.concat([key]);
    // The operator pressed again takes the cursor card and those after it,
    // as Vim's dd and 3dd.
    if (pending && typed.length === 1 && key === stood.operator) {
        return operated(stood, "line", "line");
    }
    const row = matched(typed, pending ? "pending" : mode);
    if (row === "begun") {
        return { act: "", count: 0, standing: Object.assign({}, stood, { typed }), ours: true };
    }
    // A key that ends nothing lets what was begun go, and says so.
    if (row === null) {
        return { act: "notHere", count: 0, standing: rest(), ours: true };
    }
    if (pending) {
        return operated(stood, row.act, row.kind);
    }
    const count = counted ? Number(stood.count) : 0;
    if (row.kind === "operator" && mode === "visual") {
        return {
            act: "operate", operator: row.act, motion: "selection", count, kind: "operator",
            standing: Object.assign(rest(), { taken: { key, until: now + wait } }), ours: true
        };
    }
    if (row.kind === "operator") {
        return {
            act: "pending", operator: row.act, count: 0, kind: "operator", ours: true,
            standing: Object.assign(rest(),
                { operator: key, operatorAct: row.act, operatorCount: count })
        };
    }
    return { act: row.act, count, kind: row.kind ?? "act", standing: rest(), ours: true };
}

// An operator done on what a motion or object took, its count the
// operator's and the motion's together, as Vim's 2d3j takes six.
function operated(stood, motion, kind) {
    if (kind !== "motion" && kind !== "object" && kind !== "line") {
        return { act: "notHere", count: 0, standing: rest(), ours: true };
    }
    const counts = [stood.operatorCount, stood.count === "" ? 0 : Number(stood.count)]
        .filter(one => one > 0);
    const count = counts.reduce((all, one) => all * one, counts.length === 0 ? 0 : 1);
    return { act: "operate", operator: stood.operatorAct, motion, count, kind: "operator",
        standing: rest(), ours: true };
}

// A line for `typing` to start from, as LineEditing.js keeps one, so the
// stack reads its lines through the keys alone.
function line(text, caret) {
    return LineEditing.line(text, caret);
}

// What a key does on a line that's typed in, the find, the command line or
// a reply's prompt in insert, `typed` as LineEditing.js keeps one: the act
// the mode's table gives the key, if any, else the line as LineEditing.js
// edits it; and on the find, "command" for a colon as its first letter.
function typing(typed, key, mode, repeated) {
    if (!typed.register && mode === "find" && typed.text === "" && key === ":") {
        return { act: "command", line: typed };
    }
    const row = typed.register ? null : matched([key], mode);
    if (row !== null && row !== "begun") {
        return { act: row.act, line: typed };
    }
    return LineEditing.edit(typed, key, repeated);
}

// A key in normal over a prompt's words, Vim's editing: the act ("insert"
// to type again, "send", "command", "undo", "notHere"), the line after
// it, and how the keys stand, since dw and cw wait for their second. The
// caret sits on a letter here, never after the last.
function prompting(stood, typed, key) {
    const { text, caret } = typed;
    const last = Math.max(text.length - 1, 0);
    const keys = stood.typed.concat([key]);
    const row = matched(keys, "prompt");
    if (row === "begun") {
        return { act: "", line: typed, standing: Object.assign({}, stood, { typed: keys }) };
    }
    const at = (place, act) => ({ act: act ?? "", standing: rest(),
        line: { text, caret: Math.max(Math.min(place, text.length), 0), register: false } });
    const cut = (from, to, act, caretAt) => ({
        act: act ?? "", standing: rest(),
        line: { text: text.slice(0, from) + text.slice(to), caret: caretAt ?? from,
            register: false }
    });
    if (row === null) {
        return { act: "notHere", line: typed, standing: rest() };
    }
    switch (row.act) {
    case "insert":
        return at(key === "a" ? Math.min(caret + 1, text.length) : caret, "insert");
    case "insertEnds":
        return at(key === "A" ? text.length : 0, "insert");
    case "left":
        return at(caret - 1);
    case "right":
        return at(Math.min(caret + 1, last));
    case "lineStart":
        return at(0);
    case "lineEnd":
        return at(last);
    case "wordOn":
        return at(Math.min(LineEditing.nextWord(text, caret), last));
    case "wordBack":
        return at(LineEditing.wordBefore(text, caret));
    case "wordEnd":
        return at(Math.min(LineEditing.wordEnd(text, caret), last));
    case "cut":
        return cut(caret, caret + 1, "", Math.min(caret, Math.max(text.length - 2, 0)));
    case "cutWord":
        return cut(caret, LineEditing.nextWord(text, caret));
    case "changeWord": {
        // Vim's cw changes to the word's end, as ce, on a word.
        const end = LineEditing.onSpace(text, caret) ? LineEditing.nextWord(text, caret)
            : LineEditing.wordEnd(text, caret - 1) + 1;
        return cut(caret, end, "insert");
    }
    default:
        return { act: row.act, line: typed, standing: rest() };
    }
}

// Every mode's keys as Space ? lists them, a group each, named by its
// purpose; an act is listed in the first group whose modes hold it.
const groups = [
    { name: "Normal: moving and acting", modes: ["normal", "pending"] },
    { name: "The find: the / line", modes: ["find"] },
    { name: "Commands: the : line", modes: ["command"] },
    { name: "Visual: choosing several", modes: ["visual"] },
    { name: "Insert: writing words", modes: ["insert", "prompt"] }
];

// How a key is drawn: its glyph where Chord.qml has one by name, else its
// name lowercase, as the foot says them.
// Faces as the launcher's foot draws them (ListedKeys.js).
const faces = { Return: "\u21b5", Enter: "\u21b5", Escape: "Esc" };

// One key or sequence as the foot and the list say it: a sequence's keys
// a space apart, or none where it's two letters, as dd.
function spoken(keys) {
    const each = (Array.isArray(keys) ? keys : [keys]).map(key => faces[key] ?? key);
    return each.every(key => key.length === 1) ? each.join("") : each.join(" ");
}

// The keys listed: each group's heading, then a line for each act first
// done in it, its first two keys and what it does.
function keyList() {
    const out = [];
    for (const group of groups) {
        out.push({ heading: group.name, keys: "", does: "" });
        const listed = [];
        for (const row of table) {
            const home = groups.find(one => one.modes.some(mode => inMode(row, mode)));
            if (home !== group || !row.does || listed.includes(row.act)) {
                continue;
            }
            listed.push(row.act);
            const keys = table.filter(one => one.act === row.act
                && groups.find(other => other.modes.some(mode => inMode(one, mode))) === group)
                .reduce((all, one) => all.concat(one.keys), []);
            const said = keys.map(spoken).filter((one, at, all) => all.indexOf(one) === at);
            out.push({ heading: "", keys: said.slice(0, 2).join(" "),
                does: row.does, act: row.act });
        }
    }
    return out;
}

// Where Space ? opens for :help on a key or an act: the first line listing
// it, or the top.
function helpAt(list, topic) {
    const at = list.findIndex(one => one.act === topic
        || one.keys.split(" ").includes(topic));
    return Math.max(at, 0);
}

// The hints the foot gives in a mode, from the table: each act with a hint
// done there, its first key and its word, as the launcher's foot gives
// them: Return's at the right end and Space ? just before it. `has` says
// which acts the cursor card or the selection lacks (false), so a hint
// never names a key that would do nothing; `words` gives a hint's word
// where the card says it otherwise, as d's "read" or "forget".
function hints(mode, has, words) {
    const out = [];
    for (const row of table.filter(one => inMode(one, mode) && one.hint)) {
        if (out.some(one => one.act === row.act) || has[row.act] === false) {
            continue;
        }
        out.push({ act: row.act, key: spoken(row.keys[0]), word: words[row.act] ?? row.hint });
    }
    const last = one => (one.key === faces.Return ? 2 : 0) + (one.act === "keys" ? 1 : 0);
    // Sorted by where each must stand, the rest kept in the table's order.
    return out.map((one, at) => ({ one, at }))
        .sort((a, b) => last(a.one) - last(b.one) || a.at - b.at)
        .map(placed => placed.one);
}

// Each of the key language's five modes in three capitals, as the foot's
// start names it (Joel, 2026-11-05: "the mode should be 3 letters, in
// caps, and in a place where they are more obvious and not lost with other
// things"). A reply's normal, the key list and an operator waiting are
// normal, as Vim's status line names them.
const marks = { normal: "NOR", find: "FND", command: "CMD", insert: "INS", visual: "VIS" };

function mark(mode) {
    return marks[mode] ?? marks.normal;
}

// Where a list scrolled in a room is, as Neovim's ruler says it: All when
// the room shows it all, Top and Bot at its ends, and otherwise how far
// through it is, as what's above against all that doesn't show.
function ruler(top, room, total) {
    const below = Math.max(total - top - room, 0);
    if (top === 0) {
        return below === 0 ? "All" : "Top";
    }
    if (below === 0) {
        return "Bot";
    }
    return `${Math.floor((top * 100) / (top + below))}%`;
}
