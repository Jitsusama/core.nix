.pragma library
// Each layer's shortcuts in groups, by what a chord acts on, so Shortcuts
// (KeyMap.qml) steps from the layers to their groups to the chords rather
// than listing hundreds at once (Joel, 2026-10-24: "it's hard to find the
// needles in the haystacks"). A group comes from a small table rather than
// a list of chords, so a new chord falls into one without being told: for
// each layer, rules in order, each reading one thing about a chord and
// taking it into a group when its pattern matches, the first to match
// winning. A chord the table puts wrong names its own group (`group`, the
// bind's option in Nix), and one no rule takes is listed loose, after its
// layer's groups. A group as big as a layer can hold groups of its own, by
// a table of its own. tests/key-groups.qml holds the table to the real
// binds, in Qt's own engine.

// A list of this many chords or fewer is short enough to read whole.
const FEW = 12;

// What of a niri action is only where focus goes on past an edge, as
// focus-column-or-monitor-left goes to the screen past the last column: the
// action acts on what comes before it.
const PAST_THE_EDGE = /-or-(to-)?(monitor|workspace)-[a-z]+$/;

// A table with no groups, whose chords are all loose.
const NONE = { order: [], rules: [] };

// The groups, layer by layer, in the order a layer lists them, and the rules
// that fill them. A rule reads one of a chord's facts:
//
//   action   niri's action, without where it goes on past an edge
//   command  what the chord spawns, its words joined by spaces
//   keys     the chord in its one spelling (space g h, super+shift+left)
//   when     where in its program it works ("in the outline")
//   does     what it does, in Joel's words
//
// The desktop's groups say what a chord acts on, the terminal's and pi's
// what it does in their words; Neovim's follow its leader's letters, and
// the window a key works in. `within` holds a group's own groups.
const TABLE = {
    desktop: {
        order: ["windows", "columns", "workspaces", "screens", "programs", "launcher", "capture",
            "sound", "brightness", "dictation", "session"],
        rules: [
            ["capture", "action", /^(screenshot|set-dynamic-cast|clear-dynamic-cast)/],
            ["capture", "command", /\b(recording|capture|camera)\b/],
            ["sound", "command", /\b(wpctl|pactl|playerctl|music)\b/],
            ["brightness", "command", /\b(brightnessctl|lights?)\b/],
            ["dictation", "command", /^dictation\b/],
            ["session", "command", /\bipc call (lock|session)\b|^(loginctl|systemctl)\b/],
            ["session", "action", /^quit$/],
            // A terminal opened anywhere is a program, though the shell
            // places it.
            ["programs", "command", /\bkitty\b/],
            // Where the next window opens.
            ["windows", "command", /\bipc call grid\b/],
            // The shell's own: the launcher's collections and the margin's.
            ["launcher", "command", /^(launcher-say|qs)\b/],
            ["screens", "action", /monitor/],
            ["workspaces", "action", /workspace|overview/],
            ["columns", "action", /column/],
            // Whatever else niri does acts on the window, and whatever else
            // is spawned is a program.
            ["windows", "action", /./],
            ["programs", "command", /./]
        ]
    },
    terminal: {
        order: ["tabs", "scrollback", "clipboard", "text"],
        rules: [
            ["tabs", "does", /\btabs?\b/i],
            ["scrollback", "does", /\bscroll|\boutput\b/i],
            ["clipboard", "does", /^(copy|paste)\b/i],
            ["text", "does", /\b(text|character)\b/i]
        ]
    },
    neovim: {
        order: ["files", "open", "splits", "tabs", "search", "moving", "code", "problems",
            "outline", "tests", "git", "github", "suggestions", "looks", "insides"],
        rules: [
            // A plugin's window, or a buffer of a kind, holds its own keys.
            ["github", "when", /github|pull request|review|workflow run/i],
            ["git", "when", /\bgit\b/],
            ["outline", "when", /\boutline\b/],
            ["files", "when", /\bfile browser\b/],
            ["problems", "when", /\bproblems list\b/],
            ["suggestions", "when", /\bsuggestions\b|\bcommand line\b|\bwhile typing\b/],
            ["code", "when", /\bin code\b|\boperator\b/],
            // The leader's groups, by their letters.
            ["github", "keys", /^space g h\b/],
            ["git", "keys", /^space g\b/],
            ["code", "keys", /^space [ck]\b/],
            ["problems", "keys", /^space e\b|^ctrl\+q$/],
            ["files", "keys", /^space f\b|^-$/],
            ["open", "keys", /^space (b|tab)\b/],
            ["search", "keys", /^space (s|\/|\.)( |$)/],
            ["tests", "keys", /^space t\b/],
            ["tabs", "keys", /^space (shift\+t|[1-9])\b/],
            ["looks", "keys", /^space u\b/],
            ["insides", "keys", /^space i\b/],
            ["splits", "keys", /^space (shift\+)?(w|up|down|left|right)$/],
            // [ and ] step to the one before and after, by the same letters.
            ["open", "keys", /^[[\]] b$/],
            ["tabs", "keys", /^[[\]] shift\+t$/],
            ["splits", "keys", /^[[\]] w$/],
            ["problems", "keys", /^[[\]] (q|l|d|shift\+d)$/],
            ["tests", "keys", /^[[\]] t$/],
            ["code", "keys", /^[[\]] /],
            ["moving", "keys", /^g (s|shift\+s|l|w)$|^(up|down)$/]
        ],
        within: {
            // GitHub's keys work in octo's windows, each kind its own.
            github: {
                order: ["pulls", "issues", "reviews", "runs"],
                rules: [
                    ["reviews", "when", /\breview\b/],
                    ["pulls", "when", /\bpull request\b/],
                    ["issues", "when", /\bissue\b/],
                    ["runs", "when", /\bworkflow run\b/],
                    ["pulls", "keys", /^space g h [ph]\b/],
                    ["issues", "keys", /^space g h i\b/]
                ]
            }
        }
    },
    pi: {
        order: ["typing", "conversation", "models"],
        rules: [
            ["models", "does", /\bmodels?\b|\bthinks\b/i],
            ["conversation", "does",
                /\b(conversation|thinking|tools|answer|your message)\b|\bscroll/i],
            ["typing", "does", /\b(messages?|line|paste|undo|delete|typed)\b/i]
        ]
    }
};

// What a chord's rules read, from a bind as Shortcuts.qml gives it.
function facts(bind) {
    const run = bind.run ?? [];
    const niri = run[0] === "niri";
    return {
        action: niri ? String(run[3] ?? "").replace(PAST_THE_EDGE, "") : "",
        command: niri ? "" : run.join(" "),
        keys: bind.keys ?? "",
        when: bind.when ?? "",
        does: bind.does ?? ""
    };
}

// The groups a layer lists, in order, or none for a layer the table hasn't.
function groupsOf(layer) {
    return (TABLE[layer] ?? NONE).order;
}

// The group a chord is in, `depth` groups down: the one it names there, a
// group and the one inside it written group/inner, or else the first the
// table's rules give it, or "" for one none takes.
function groupIn(table, bind, depth) {
    const named = String(bind.group ?? "").split("/").filter(part => part !== "");
    if (depth < named.length) {
        return named[depth];
    }
    const known = facts(bind);
    const rule = table.rules.find(each => known[each[1]] !== "" && each[2].test(known[each[1]]));
    return rule ? rule[0] : "";
}

// The group a chord is in at the top of its layer.
function groupOf(layer, bind) {
    return groupIn(TABLE[layer] ?? NONE, bind, 0);
}

// Chords by their places in `binds`, as a table lists them: its groups in
// order, each with its chords and what's inside it, then those in none. A
// list short enough to read whole is all loose; a group a bind names that
// the table hasn't comes after the table's.
function sorted(table, binds, places, depth) {
    if (places.length <= FEW) {
        return { groups: [], loose: places.slice() };
    }
    const held = {};
    const loose = [];
    const order = table.order.slice();
    for (const at of places) {
        const group = groupIn(table, binds[at], depth);
        if (group === "") {
            loose.push(at);
        } else if (held[group]) {
            held[group].push(at);
        } else {
            held[group] = [at];
            if (!order.includes(group)) {
                order.push(group);
            }
        }
    }
    const inner = table.within ?? {};
    return {
        groups: order.filter(group => held[group]).map(group => ({
            id: group,
            at: held[group],
            inside: sorted(inner[group] ?? NONE, binds, held[group], depth + 1)
        })),
        loose
    };
}

// A layer's chords as it lists them, by their places in `binds`: each group
// is { id, at, inside }, `at` every chord in it and `inside` how it lists
// them, as a layer does.
function arrange(layer, binds) {
    return sorted(TABLE[layer] ?? NONE, binds, binds.map((_bind, at) => at), 0);
}

// Every group a table can list, as paths: desktop/windows,
// neovim/github/pulls.
function paths() {
    const out = [];
    const walk = (table, prefix) => {
        for (const group of table.order) {
            out.push(`${prefix}/${group}`);
            walk(table.within?.[group] ?? NONE, `${prefix}/${group}`);
        }
    };
    for (const layer of Object.keys(TABLE)) {
        walk(TABLE[layer], layer);
    }
    return out;
}
