.pragma library
// Each layer's shortcuts in groups, by what a chord acts on, so Shortcuts
// (KeyMap.qml) steps from the layers to their groups to the chords rather
// than listing hundreds at once (Joel, 2026-10-24: "it's hard to find the
// needles in the haystacks", and later "we need just the right amount").
// The card shows eight rows, so a level aims at about eight: groups of
// about eight, never a group of one or two, and no more than two levels of
// groups below a layer. A group comes from a small table rather than a
// list of chords, so a new chord falls into one without being told: for
// each layer, rules in order, each reading one or more things about a
// chord and taking it into a group when every pattern matches, the first
// to match winning. A chord the table puts wrong names its own group
// (`group`, the bind's option in Nix), and one no rule takes is listed
// loose, after its layer's groups. A group as big as a layer holds groups
// of its own, by a table of its own. tests/key-groups.qml holds the table
// to the real binds, in Qt's own engine.

// A list of this many chords or fewer is short enough to read whole: as
// many as the card shows.
const FEW = 8;

// What of a niri action is only where focus goes on past an edge, as
// focus-column-or-monitor-left goes to the screen past the last column: the
// action acts on what comes before it.
const PAST_THE_EDGE = /-or-(to-)?(monitor|workspace)-[a-z]+$/;

// A table with no groups, whose chords are all loose.
const NONE = { order: [], rules: [] };

// Where a terminal's title says it runs Neovim or pi: Neovim titles it by a
// memo and the file (modules/neovim/init.lua's titlestring), or by its own
// name, and pi by its letter and the session's name. Each is read from the
// title's start, since what follows is a file, a folder or a quest's name,
// any of which may hold "vim".
const NEOVIM_TITLE = /^(\u{1F4DD}|n?vim\b)/iu;
const PI_TITLE = /^(\u03C0|pi)( |$)/u;

// The program whose keys are each layer's, by its window's app id.
const TERMINAL_APP = "kitty";

// The layers of programs in windows of their own, by their windows' app
// ids, each with its own keys over the desktop's.
const WINDOW_APPS = new Map([["swayimg", "swayimg"], ["inlyne", "inlyne"], ["org.pwmt.zathura", "zathura"]]);

// A place a chord works in, said after another: "in a pull request" after
// "in a GitHub issue" is "a pull request".
const IN = /^in /;

// The groups, layer by layer, in the order a layer lists them, and the rules
// that fill them. A rule names its group, then a fact and its pattern, and
// any more facts and patterns that must match as well. The facts:
//
//   action   niri's action, without where it goes on past an edge
//   command  what the chord spawns, its words joined by spaces
//   keys     the chord in its one spelling (space g h, super+shift+left)
//   when     where in its program it works ("in the outline")
//   does     what it does, in Joel's words
//
// Groups are named for what their keys act on. The desktop's say it by
// niri's action or the program spawned, the terminal's and pi's by what a
// chord does, and Neovim's by its leader's letters and the window a key
// works in. `within` holds a group's own groups.
const TABLE = {
    desktop: {
        order: ["windows", "columns", "workspaces", "screens", "launcher", "typing", "capture",
            "sound"],
        rules: [
            ["capture", "action", /^(screenshot|set-dynamic-cast|clear-dynamic-cast)/],
            ["capture", "command", /\b(recording|capture|camera)\b/],
            // Brightening and locking act on the screen itself.
            ["screens", "command", /\b(brightnessctl|lights?)\b|\bipc call lock\b/],
            ["sound", "command", /\b(wpctl|pactl|playerctl|music)\b/],
            ["typing", "command", /^dictation\b|\bcollection emoji$|\bipc call keyboard\b/],
            // Where the next window opens is the shell's, but acts on the
            // windows.
            ["windows", "command", /\bipc call grid\b/],
            // The shell's own: the launcher's collections and the notices.
            ["launcher", "command", /^(launcher-say|qs)\b/],
            ["screens", "action", /monitor/],
            ["workspaces", "action", /workspace|overview/],
            ["columns", "action", /column/],
            // Whatever else niri does acts on the window, and whatever else
            // is spawned, as a terminal is, opens a new one.
            ["windows", "action", /./],
            ["windows", "command", /./]
        ],
        within: {
            windows: {
                order: ["focus", "place", "size", "new"],
                rules: [
                    ["new", "command", /./],
                    ["focus", "action", /^(focus|switch-focus)/],
                    ["place", "action", /^(move|consume|center|toggle-window-floating)/],
                    ["size", "action", /height|maximize|fullscreen/]
                ]
            },
            columns: {
                order: ["focus", "place", "width"],
                rules: [
                    ["focus", "action", /^focus/],
                    ["place", "action", /^(move|center)/],
                    ["width", "action", /width|maximize|expand/]
                ]
            },
            // Each workspace by its number, to go to or to move a column or
            // a window to, and the ones above and below for all three.
            // Moving the workspace itself and showing them all are loose.
            workspaces: {
                order: ["focus", "column", "window", "near"],
                rules: [
                    ["near", "action", /^(focus|move-column-to|move-window-to)-workspace-(up|down)$/],
                    ["focus", "action", /^focus/],
                    ["column", "action", /^move-column/],
                    ["window", "action", /^move-window/]
                ]
            },
            screens: {
                order: ["focus", "column", "window", "this"],
                rules: [
                    ["this", "command", /./],
                    ["focus", "action", /^focus/],
                    ["column", "action", /^move-column/],
                    ["window", "action", /^move-window/]
                ]
            },
            launcher: {
                order: ["notices"],
                rules: [["notices", "command", /\bnotices\b|\bquiet\b/]]
            },
            capture: {
                order: ["sharing"],
                rules: [["sharing", "action", /cast/]]
            }
        }
    },
    terminal: {
        order: ["tabs", "numbers", "scrollback", "clipboard", "text"],
        rules: [
            ["numbers", "does", /^Go to tab \d/],
            // A window opens as a tab does, beside it.
            ["tabs", "does", /\b(tabs?|window)\b/i],
            ["scrollback", "does", /\bscroll|\boutput\b/i],
            // Picking a word copies it.
            ["clipboard", "does", /^(copy|paste|pick a word)\b/i],
            ["text", "does", /\b(text|character)\b/i]
        ]
    },
    neovim: {
        order: ["files", "search", "code", "problems", "outline", "tests", "git", "pulls",
            "github", "editor"],
        rules: [
            // GitHub's conversation is the same keys wherever it is: people
            // and labels, comments and reactions, in a pull request, an
            // issue or a review's thread alike.
            ["github", "keys", /^space g h ([aclt]|i g)\b/],
            ["github", "keys", /^[[\]] shift\+c$/, "when", /issue|pull request|thread/],
            // A plugin's window, or a buffer of a kind, holds its own keys.
            ["pulls", "when", /pull request|review/],
            ["github", "when", /github|workflow run/i],
            ["git", "when", /\bgit\b/],
            ["outline", "when", /\boutline\b/],
            ["files", "when", /\bfile browser\b/],
            ["problems", "when", /\bproblems list\b/],
            ["editor", "when", /\bsuggestions\b|\bcommand line\b|\bwhile typing\b|\bterminal\b/],
            ["code", "when", /\bin code\b|\boperator\b/],
            // The leader's groups, by their letters.
            ["pulls", "keys", /^space g h ([hpvmk]|shift\+t)\b/],
            ["github", "keys", /^space g h\b/],
            ["git", "keys", /^space g\b/],
            ["outline", "keys", /^space c o\b/],
            // Finding anything, or a symbol, is searching.
            ["search", "keys", /^space (space|c space)$/],
            ["code", "keys", /^space ([ck]|i l)\b/],
            ["problems", "keys", /^space e\b|^ctrl\+q$/],
            ["files", "keys", /^space f\b|^-$/],
            ["search", "keys", /^space (s|\/|\.)( |$)/],
            ["files", "keys", /^space (b|tab|w|shift\+t|[1-9]|(shift\+)?(up|down|left|right))\b/],
            ["tests", "keys", /^space t\b/],
            ["editor", "keys", /^space (u|i|\?|enter|shift\+[qu])( |$)/],
            // [ and ] step to the one before and after, by the same letters.
            ["files", "keys", /^[[\]] (b|w|shift\+t)$/],
            ["problems", "keys", /^[[\]] (q|l|d|shift\+d)$/],
            ["tests", "keys", /^[[\]] t$/],
            ["code", "keys", /^[[\]] /],
            ["search", "keys", /^g (s|shift\+s|l|w)$|^(up|down)$/]
        ],
        within: {
            // Finding a file, the open ones, the file browser and the folders
            // it shows, and the splits and tabs files are shown in.
            files: {
                order: ["finding", "open", "browser", "folders", "splits", "tabs", "numbers"],
                rules: [
                    ["folders", "keys", /^(-|_|`|~|ctrl\+l|g h|g \\)$/],
                    ["browser", "when", /\bfile browser\b/],
                    ["browser", "keys", /^space f e$/],
                    ["finding", "keys", /^space f\b/],
                    ["open", "keys", /^space (b|tab)\b|^[[\]] b$/],
                    ["numbers", "keys", /^space [1-9]$/],
                    ["tabs", "keys", /^space shift\+t\b|^[[\]] shift\+t$/],
                    ["splits", "keys", /./]
                ]
            },
            search: {
                order: ["text", "symbols", "lists", "jumps"],
                rules: [
                    ["symbols", "keys", /^space (s [ys]|c space)$/],
                    ["lists", "keys", /^space s (shift\+)?[hqt]$/],
                    ["jumps", "keys", /^g |^(up|down)$/],
                    ["text", "keys", /^space (s (?!space$)\S+|\/|\.)$/]
                ]
            },
            // What the language server knows of the code, by the leader's
            // letters, the pieces of it an operator takes, and [ and ]
            // through it.
            code: {
                order: ["definitions", "hints", "refactoring", "formatting", "selecting",
                    "pieces", "through", "server"],
                rules: [
                    ["pieces", "when", /\boperator\b/],
                    ["selecting", "keys", /^space c s\b/],
                    ["refactoring", "keys", /^space c (r|shift\+r)\b/],
                    ["hints", "keys", /^space c ([ahlt]|shift\+h)\b/],
                    ["formatting", "keys", /^space c (f|shift\+f|n)\b/],
                    ["definitions", "keys", /^space c (u|c)\b|^g /],
                    ["through", "keys", /^[[\]] /],
                    ["server", "keys", /./]
                ]
            },
            problems: {
                order: ["finding", "showing", "through", "opening", "list", "folding"],
                rules: [
                    ["showing", "keys", /^space e t\b/],
                    ["finding", "keys", /^space e\b|^ctrl\+q$/],
                    ["through", "keys", /^[[\]] /],
                    ["folding", "keys", /^z /],
                    ["opening", "keys", /^(enter|o|ctrl\+[sv]|p|shift\+p|escape|[{}])$/],
                    ["list", "when", /./]
                ]
            },
            // Showing the outline, then its window's keys, and walking it.
            outline: {
                order: ["showing", "symbols", "through", "holding", "folding", "walking"],
                rules: [
                    ["walking", "when", /^walking\b/],
                    ["showing", "keys", /^space c o\b|^[?q]$/],
                    ["symbols", "keys", /^(enter|ctrl\+[svjk]|p)$/],
                    ["through", "keys", /^([{}]|\[ \[|\] \])$/],
                    ["folding", "keys", /^z /],
                    ["holding", "keys", /^(shift\+)?[hl]$/]
                ]
            },
            tests: {
                order: ["running", "results"],
                rules: [
                    ["results", "keys", /^space t (o|shift\+[os])$|^[[\]] t$/],
                    ["running", "keys", /^space t (?!space$)/]
                ]
            },
            // Git's leader's letters, and its status window's keys.
            git: {
                order: ["changes", "staging", "commits", "branches", "stashes", "remote",
                    "status"],
                rules: [
                    ["status", "when", /\bgit's status\b/],
                    ["status", "keys", /^space g g$/],
                    ["commits", "keys", /^space g (c|w b)\b/],
                    ["changes", "keys", /^space g [wd]\b|^[[\]] g$|^i h$/],
                    ["staging", "keys", /^space g s\b/],
                    ["branches", "keys", /^space g [bt]\b/],
                    ["stashes", "keys", /^space g a\b/],
                    ["remote", "keys", /^space g n\b/]
                ]
            },
            // A pull request's own keys, and a review's windows.
            pulls: {
                order: ["finding", "pull", "merging", "checks", "reviewing", "threads",
                    "files", "submitting"],
                rules: [
                    ["finding", "keys", /^space g h [hp]\b/],
                    ["merging", "keys", /^space g h m (?!o$)/],
                    ["checks", "keys", /^space g h k /],
                    ["threads", "keys", /^space g h shift\+t\b|^[[\]] shift\+c$/],
                    ["submitting", "keys", /^space g h v (shift\+s|d|c)$|^ctrl\+[amrc]$/,
                        "when", /\breview\b/],
                    ["pull", "keys", /^(space g h b shift\+y|g f)$/],
                    ["files", "when", /\breview's\b/],
                    ["reviewing", "keys", /^space g h v\b/],
                    ["pull", "when", /./]
                ]
            },
            // What's alike in a pull request, an issue and a review's
            // thread, then GitHub's other things.
            github: {
                order: ["issues", "people", "comments", "reactions", "runs", "repositories",
                    "notifications"],
                rules: [
                    ["people", "keys", /^space g h [al]\b/],
                    ["comments", "keys", /^space g h c\b|^[[\]] shift\+c$/],
                    ["reactions", "keys", /^space g h t\b/],
                    ["runs", "when", /\bworkflow run\b/],
                    ["notifications", "when", /\bnotifications\b/],
                    ["notifications", "keys", /^space g h n\b/],
                    ["issues", "keys", /^space g h [ib]\b/],
                    ["issues", "when", /\bissue\b/],
                    ["repositories", "keys", /^space g h [rsfg]\b/]
                ]
            },
            editor: {
                order: ["suggestions", "commands", "looks", "insides"],
                rules: [
                    ["suggestions", "when", /\bwhile typing\b/],
                    ["commands", "when", /\bcommand line\b/],
                    // Spelling is how the words look.
                    ["looks", "keys", /^space (u|i x)( |$)/],
                    ["insides", "keys", /^space (i|\?)( |$)/]
                ]
            }
        }
    },
    pi: {
        order: ["message", "editing", "conversation", "scrollback", "models"],
        rules: [
            ["models", "does", /\bmodels?\b|\bthinks\b/i],
            ["scrollback", "does", /\bscroll|^Go to (the start|the end|your message)/i],
            ["conversation", "does", /\b(conversation|thinking|tools|answer)\b/i],
            ["message", "does", /\bmessages?\b|\bnew line\b/i],
            ["editing", "does", /\b(paste|undo|delete|typed)\b/i]
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

// Whether a rule takes a chord: every fact it reads says something, and
// matches.
function takes(rule, known) {
    for (let at = 1; at < rule.length; at += 2) {
        const fact = known[rule[at]];
        if (fact === "" || !rule[at + 1].test(fact)) {
            return false;
        }
    }
    return true;
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
    const rule = table.rules.find(each => takes(each, known));
    return rule ? rule[0] : "";
}

// The group a chord is in at the top of its layer.
function groupOf(layer, bind) {
    return groupIn(TABLE[layer] ?? NONE, bind, 0);
}

// The places a chord works in as one: everywhere if any is, and otherwise
// each once, the last after "or", "in" said once for a run of them.
function joined(whens) {
    const each = whens.filter((when, at) => whens.indexOf(when) === at);
    if (each.includes("")) {
        return "";
    }
    const said = each.map((when, at) => at > 0 && IN.test(each[at - 1]) ? when.replace(IN, "")
        : when);
    const last = said.pop();
    return said.length === 0 ? last : `${said.join(", ")} or ${last}`;
}

// A layer's chords as Shortcuts lists them: one chord doing one thing in
// several of a group's places, as reacting does in a pull request, an issue
// and a review's thread, is one chord saying every place it works, since
// Joel sees nothing twice. Each says where its binds were in `binds`, as
// `places`. And one thing done by several chords in one place, as Super+L
// and the screensaver key both lock the screen, is one row too, its first
// chord drawn and the rest in `also`.
function merged(layer, binds) {
    const at = {};
    const alike = {};
    const out = [];
    binds.forEach((bind, place) => {
        const group = groupOf(layer, bind);
        // A chord is a string in the key map and drawn key by key in the
        // shell, so it's compared as JSON.
        const chord = JSON.stringify(bind.chord);
        const id = `${chord}\n${bind.does}\n${group}`;
        const twin = `${bind.does}\n${group}\n${bind.when ?? ""}`;
        if (at[id] !== undefined) {
            out[at[id]].places.push(place);
            out[at[id]].whens.push(bind.when ?? "");
        } else if (alike[twin] === undefined) {
            at[id] = out.length;
            alike[twin] = out.length;
            out.push({ bind, places: [place], whens: [bind.when ?? ""], also: [] });
        } else {
            at[id] = alike[twin];
            out[alike[twin]].places.push(place);
            out[alike[twin]].also.push(bind.chord);
        }
    });
    return out.map(one => Object.assign({}, one.bind, {
        when: joined(one.whens),
        places: one.places,
        also: one.also
    }));
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

// A layer's chords as it lists them, by their places in `binds`, as
// merged gives them: each group is { id, at, inside }, `at` every chord in
// it and `inside` how it lists them, as a layer does.
function arrange(layer, binds) {
    return sorted(TABLE[layer] ?? NONE, binds, binds.map((_bind, at) => at), 0);
}

// Every group a table can list, as paths: desktop/windows,
// neovim/pulls/files.
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

// The layers whose keys work in a window, its own first: a terminal's, or
// Neovim's or pi's and then the terminal's when its title says it runs one,
// and then the desktop's, which work over any window. A window of a program
// with a layer of its own has that first, and any other, or none, has only
// the desktop's.
function over(app, title) {
    if (app !== TERMINAL_APP) {
        const own = WINDOW_APPS.get(app);
        return own === undefined ? ["desktop"] : [own, "desktop"];
    }
    if (PI_TITLE.test(title)) {
        return ["pi", "terminal", "desktop"];
    }
    return NEOVIM_TITLE.test(title) ? ["neovim", "terminal", "desktop"] : ["terminal", "desktop"];
}
