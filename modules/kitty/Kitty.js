// kitty's own state, as Programs › kitty lists it in the launcher
// (Kitty.qml): every tab of every kitty OS window niri shows, as they were
// used, each tab's panes, what runs in each and where, the words each row
// says and the commands that go to one and close it. Kept apart from the
// QML so Qt's own engine runs it (tests/kitty-tabs.qml); what a new window
// opens is KittyOpens.js's.
//
// What kitty says of each of its windows, and where each goes here:
// `id` names it to Go To and Close It; `is_active` marks it; `title` names
// it where a program set one; `cwd` and `foreground_processes` say where it
// is and what runs, and `cmdline`, `last_reported_cmdline`,
// `last_cmd_exit_status`, `at_prompt`, `in_alternate_screen`, `lines`,
// `columns`, `created_at`, `session_name`, `user_vars` and the leader's
// `pid` are its drawer's; `needs_attention` and
// `has_activity_since_last_focus` are said beside it; each tab's
// `is_active`, `title` and `title_overridden` name it and mark it, and
// its `active_window_history` and each OS window's `active_tab_history`
// order them. Left out: `env`, the whole environment, secrets and all;
// `is_focused` on each, since the launcher holds the keys while it's
// open; `last_focused_at`, kitty's own monotonic clock, which means
// nothing as a time of day and orders no more than the histories do;
// `is_self`, since the launcher runs in no kitty; `neighbors`, the
// layout's, and each tab's `layout`, `layout_state`, `layout_opts`,
// `enabled_layouts` and `groups`, since kitty.conf allows only the stack,
// where nothing sits beside anything; `title_overridden` on a window,
// whose title is said either way; and each OS window's `is_active` and
// `last_focused`, which niri's focus times say across every kitty at
// once, `platform_window_id`, null on Wayland, `background_opacity`, the
// theme's, and `wm_class` and `wm_name`, which niri's window says already.
.pragma library
.import "Ago.js" as Ago

const NANOSECONDS_A_MILLISECOND = 1e6;
const NANOSECONDS_A_SECOND = 1e9;

// What's left of a program's file name: its path, a login shell's dash,
// and the wrapping Nix gives some (.nvim-wrapped).
const FOLDERS = /^.*\//;
const LOGIN = /^-/;
const WRAPPED = /^\.(.+)-wrapped$/;
// A setting given to env, NAME=value.
const SETTING = /^\w+=/;
// What a machine's name is said with in ssh's arguments.
const SCHEME = /^ssh:\/\//;
const PORT_OR_PATH = /[:/].*$/;

// What `open-now kitty` prints, a kitty a line: its process, a tab, and
// its `ls` as JSON. A line that isn't whole is passed over, so one kitty
// answering badly hides only itself.
function heard(text) {
    const kitties = [];
    for (const line of String(text ?? "").split("\n")) {
        const tab = line.indexOf("\t");
        const pid = Number(line.slice(0, tab));
        const windows = tab > 0 ? parsed(line.slice(tab + 1)) : null;
        if (Number.isInteger(pid) && pid > 0 && Array.isArray(windows)) {
            kitties.push({ pid, windows });
        }
    }
    return kitties;
}

function parsed(text) {
    try {
        return JSON.parse(text);
    } catch (_error) {
        return null;
    }
}

// When niri last focused a window, in nanoseconds, or 0 for never.
function focusedAt(window) {
    const stamp = window.focus_timestamp;
    return stamp ? stamp.secs * NANOSECONDS_A_SECOND + stamp.nanos : 0;
}

// A kitty OS window's title, which kitty takes from its active tab.
function titleOf(os) {
    return (os.tabs ?? []).find(tab => tab.is_active)?.title ?? "";
}

// niri's window for each of one kitty's OS windows, by kitty's id. Both
// number their windows as they open them, so with as many of each they
// pair in that order, two of one title included. A kitty with more holds
// a hidden one niri never shows, as the sessions' kitty does, and then
// each is niri's window of its title, where only one of each has it.
function onScreen(kitty, niri) {
    const theirs = niri.filter(window => window.pid === kitty.pid)
        .sort((a, b) => a.id - b.id);
    const own = kitty.windows.slice().sort((a, b) => a.id - b.id);
    const placed = {};
    if (own.length === theirs.length) {
        own.forEach((os, at) => {
            placed[os.id] = theirs[at];
        });
        return placed;
    }
    for (const os of own) {
        const alike = theirs.filter(window => window.title === titleOf(os));
        const twins = own.filter(other => titleOf(other) === titleOf(os));
        if (alike.length === 1 && twins.length === 1) {
            placed[os.id] = alike[0];
        }
    }
    return placed;
}

// `items` in the order they were used: the active one, then kitty's own
// history of them, newest first, then the rest in the tab bar's order, as
// Ctrl+Tab walks them.
function byUse(items, history) {
    const ids = [items.find(item => item.is_active)?.id]
        .concat((history ?? []).slice().reverse(), items.map(item => item.id));
    const order = [];
    for (const id of ids) {
        if (!order.includes(id) && items.some(item => item.id === id)) {
            order.push(id);
        }
    }
    return order.map(id => items.find(item => item.id === id));
}

// Every tab of every kitty OS window niri shows, as the launcher lists
// them: the OS window niri focused last first, and in each its tabs as
// they were used. An OS window niri doesn't show, as a hidden one, isn't
// one Joel can go to, so its tabs aren't listed.
function tabs(kitties, niri) {
    const placed = [];
    for (const kitty of kitties) {
        const screen = onScreen(kitty, niri);
        for (const os of kitty.windows) {
            if (screen[os.id]) {
                placed.push({ pid: kitty.pid, os, window: screen[os.id] });
            }
        }
    }
    placed.sort((a, b) => focusedAt(b.window) - focusedAt(a.window) || a.window.id - b.window.id);
    const all = [];
    for (const one of placed) {
        const bar = one.os.tabs ?? [];
        for (const tab of byUse(bar, one.os.active_tab_history)) {
            all.push(tabOf(one, tab, bar));
        }
    }
    return all;
}

// The words a row is said in, by what it is: a tab is Tabs' one tab, as Chrome's is, with its
// title to copy (VKCF3Z, 2026-11-12 [f56eec6d]); a pane keeps kitty's own, with none.
function wordsOf(kind) {
    if (kind === "tab") {
        const verb = "verb:tabs";
        return { thing: "tabs:tab", go: `${verb}:go`, close: `${verb}:close`, copy: `${verb}:title` };
    }
    const verb = `verb:kitty:${kind}`;
    return { thing: `kitty:${kind}`, go: `${verb}:go`, close: `${verb}:close`, copy: "" };
}

function tabOf(placed, tab, bar) {
    const panes = byUse(tab.windows ?? [], tab.active_window_history)
        .map(window => Object.assign(paneOf(placed.pid, window), { window: placed.window.id }));
    return {
        key: `kitty:tab:${placed.pid}:${tab.id}`,
        pid: placed.pid,
        window: placed.window.id,
        id: tab.id,
        index: bar.indexOf(tab) + 1,
        count: bar.length,
        active: tab.is_active === true,
        title: tab.title ?? "",
        named: tab.title_overridden === true,
        panes
    };
}

function paneOf(pid, window) {
    const runs = running(window);
    return {
        key: `kitty:pane:${pid}:${window.id}`,
        pid,
        id: window.id,
        active: window.is_active === true,
        title: window.title ?? "",
        folder: runs.leader?.cwd || window.cwd || "",
        runs,
        command: (window.cmdline ?? []).join(" "),
        last: window.last_reported_cmdline ?? "",
        status: window.last_cmd_exit_status ?? 0,
        prompt: window.at_prompt === true,
        full: window.in_alternate_screen === true,
        columns: window.columns ?? 0,
        lines: window.lines ?? 0,
        opened: (window.created_at ?? 0) / NANOSECONDS_A_MILLISECOND,
        rang: window.needs_attention === true,
        output: window.has_activity_since_last_focus === true,
        session: window.session_name ?? "",
        vars: window.user_vars ?? {}
    };
}

// What runs in a pane: the leader of what's in front, the process whose
// group it is (`pgrp`, which open-now adds), as what Joel ran, rather than
// what it started, as pi's git-engine. kitty guesses the newest for its own
// titles, and ids wrap, so neither the highest nor the lowest is sure. A
// leader gone, as the first of a pipe that finished, leaves the pane's own
// process where it's in front, then the lowest id.
function leaderOf(processes, window) {
    return processes.find(one => one.pgrp === one.pid)
        ?? processes.find(one => one.pid === window.pid)
        ?? processes[0] ?? null;
}

// Its program, and over ssh the machine.
function running(window) {
    const processes = (window.foreground_processes ?? [])
        .filter(one => Array.isArray(one.cmdline) && one.cmdline.length > 0)
        .sort((a, b) => a.pid - b.pid);
    const leader = leaderOf(processes, window);
    const words = unwrapped(leader ? leader.cmdline : window.cmdline ?? []);
    const ssh = sshArgs(words);
    return {
        leader,
        processes,
        program: ssh ? "ssh" : program(words),
        machine: ssh ? destination(ssh) : ""
    };
}

// A program by the name it's run as: its file's name, without the dash a
// login shell's has or the wrapping Nix gives some.
function base(word) {
    const name = String(word ?? "").replace(FOLDERS, "").replace(LOGIN, "");
    const wrapped = WRAPPED.exec(name);
    return wrapped ? wrapped[1] : name;
}

// What a command runs once what only starts it is taken away: env and its
// settings, and a session's mycelium, which holds what follows its `--`
// (Sessions.qml starts pi so).
function unwrapped(words) {
    const name = base(words[0]);
    if (name === "env") {
        const at = commandAfterEnv(words);
        return at < 0 ? words : unwrapped(words.slice(at));
    }
    const after = words.indexOf("--");
    if (name === "mycelium" && words.includes("hold") && after > 0 && after < words.length - 1) {
        return unwrapped(words.slice(after + 1));
    }
    return words;
}

// env's options that take the word after them, as env(1) lists them.
const ENV_VALUED = ["-C", "-S", "-u", "--chdir", "--split-string", "--unset"];

// Where the command env runs starts, past its options and its settings, or
// -1 where it runs none.
function commandAfterEnv(words) {
    for (let at = 1; at < words.length; at += 1) {
        const word = words[at];
        if (ENV_VALUED.includes(word)) {
            at += 1;
        } else if (!(word.startsWith("-") || SETTING.test(word))) {
            return at;
        }
    }
    return -1;
}

// The programs that run a script, which is then what runs.
const INTERPRETERS = ["bash", "bun", "deno", "node", "perl", "python", "python3", "ruby", "sh"];

function program(words) {
    const name = base(words[0]);
    if (!INTERPRETERS.includes(name)) {
        return name;
    }
    const script = words.slice(1).find(word => !word.startsWith("-"));
    return script ? base(script.split(" ")[0]) : name;
}

// The ways ssh is run: itself, or kitty's ssh kitten, as `kitten ssh` or
// `kitty +kitten ssh`.
const SSH = [["ssh"], ["kitten", "ssh"], ["kitty", "+kitten", "ssh"]];

// ssh's own arguments, where a command is ssh, or null.
function sshArgs(words) {
    const name = base(words[0]);
    const way = SSH.find(each => each[0] === name
        && each.every((word, at) => at === 0 || words[at] === word));
    return way ? words.slice(way.length) : null;
}

// ssh's options that take a value, as ssh(1) lists them.
const VALUED = "B b c D E e F I i J L l m O o P p Q R S W w".split(" ");

// The machine ssh's arguments reach: the first that isn't an option or an
// option's value, without a user or a port, as a name ~/.ssh/config gives.
// The kitten's own options come as `--kitten` and a value.
function destination(args) {
    let skip = false;
    for (const word of args) {
        if (skip) {
            skip = false;
        } else if (word === "--kitten") {
            skip = true;
        } else if (word.startsWith("-") && word !== "-" && word !== "--") {
            skip = takesValue(word);
        } else if (word !== "--") {
            return machine(word);
        }
    }
    return "";
}

// Whether a cluster of ssh's flags ends on one waiting for its value.
function takesValue(word) {
    for (let at = 1; at < word.length; at += 1) {
        if (VALUED.includes(word[at])) {
            return at === word.length - 1;
        }
    }
    return false;
}

function machine(given) {
    const bare = given.replace(SCHEME, "");
    const host = bare.slice(bare.lastIndexOf("@") + 1);
    return host.startsWith("[") ? host.slice(1, host.indexOf("]"))
        : host.replace(PORT_OR_PATH, "");
}

// A path as Joel reads it, from home as ~.
function shown(path, home) {
    if (path === home) {
        return "~";
    }
    return path.startsWith(`${home}/`) ? `~${path.slice(home.length)}` : path;
}

function fill(said, values) {
    let text = said ?? "";
    for (const key of Object.keys(values)) {
        text = text.replace(`{${key}}`, String(values[key]));
    }
    return text;
}

// What runs in a pane in its row's words: its program, or ssh and the
// machine.
function runsOf(pane, said) {
    return pane.runs.machine === "" ? pane.runs.program
        : fill(said.ssh, { machine: pane.runs.machine });
}

// Whether a title says more than what runs: a title that's no more than a
// program's name, as kitty gives a window none has titled, says nothing.
function titled(pane) {
    const names = pane.runs.processes.map(one => base(one.cmdline[0]))
        .concat([pane.runs.program, base(pane.command.split(" ")[0]), "kitty", ""]);
    return !names.includes(pane.title.trim());
}

// A tab's or a pane's name: the title Joel gave a tab, else the machine an
// ssh session is on, whatever the far shell titles it, else the title its
// program gave it, else what runs there. The far title stays in the
// drawer.
function nameOf(one, said) {
    const pane = one.panes ? one.panes[0] : one;
    if (one.named) {
        return one.title;
    }
    return pane.runs.machine === "" && titled(pane) ? pane.title : runsOf(pane, said);
}

// What's said at a row's right: how it stands first, then how many panes a
// tab holds, what runs where the name doesn't say it, and the folder, so a
// side cut short keeps the state. An ssh session's folder is this
// machine's, not the one it's on, so it's left unsaid.
function sideOf(one, said, home) {
    const panes = one.panes ?? [one];
    const [pane] = panes;
    const states = [one.active ? said.active : "",
        panes.some(each => each.rang) ? said.rang : "",
        panes.some(each => each.output) ? said.output : ""];
    const runs = nameOf(one, said) === runsOf(pane, said) ? "" : runsOf(pane, said);
    const folder = pane.runs.machine === "" && pane.folder !== "" ? shown(pane.folder, home) : "";
    return states.concat([panes.length > 1 ? fill(said.panes, { count: panes.length }) : "", runs,
        folder]).filter(part => part !== "").join(" \u00b7 ");
}

// A closed tab's or pane's side, said so where it was until the launcher
// opens again.
function closedSide(one, said, home) {
    const pane = one.panes ? one.panes[0] : one;
    return [said.closed, pane.runs.machine === "" ? shown(pane.folder, home) : ""]
        .filter(part => part !== "").join(" \u00b7 ");
}

// What else finds a tab or a pane: its title, what runs and its folder.
function keywordsOf(one, said) {
    const panes = one.panes ?? [one];
    // Qt's engine has no flatMap.
    const words = panes.reduce((all, pane) => all.concat([pane.title, runsOf(pane, said),
        pane.folder]), [one.title]).filter(word => word !== "");
    return Array.from(new Set(words)).join(" ");
}

// Where a tab sits in its OS window's bar, as kitty numbers it.
function placeOf(tab, said) {
    return fill(said.tab, { index: tab.index, count: tab.count });
}

// What's in front, what Joel ran first and what it started after it.
function leaderFirst(runs) {
    return runs.leader ? [runs.leader].concat(runs.processes.filter(one => one !== runs.leader))
        : runs.processes;
}

// A pane's drawer: its name, and every figure kitty gives of it, each
// labelled by `named(id)`, in the order Joel would ask: where, what runs,
// how it stands, then when and how big.
function drawerOf(pane, words, now) {
    const { said } = words;
    const yes = on => on ? said.yes : said.no;
    const facts = [
        ["folder", shown(pane.folder, words.home)],
        ["machine", pane.runs.machine],
        ["running", leaderFirst(pane.runs).map(one => one.cmdline.join(" ")).join(" \u00b7 ")],
        ["title", pane.title],
        ["command", pane.command],
        ["last", pane.last],
        ["status", pane.last === "" ? "" : String(pane.status)],
        ["prompt", yes(pane.prompt)],
        ["full", yes(pane.full)],
        ["size", fill(said.size, { columns: pane.columns, lines: pane.lines })],
        ["opened", pane.opened > 0 ? Ago.ago(pane.opened, now) : ""],
        ["session", pane.session],
        ["process", pane.runs.leader ? String(pane.runs.leader.pid) : ""],
        ["vars", Object.keys(pane.vars).map(key => `${key}=${pane.vars[key]}`).join(" \u00b7 ")]
    ];
    return {
        title: nameOf(pane, said),
        facts: facts.filter(fact => fact[1] !== "")
            .map(fact => ({ label: words.named(fact[0]), value: fact[1] }))
    };
}

// The drawer of kitty's one row for the agents' windows, whose panes the
// programs daemon leaves out of Tabs (agents.rs): how many there are, and
// that Agents is where each is, by the launcher's words for it: what it
// says, and the sentence it tells.
function agentsDrawer(count, said, tells) {
    return {
        title: fill(count === 1 ? said.one : said.many, { count }),
        body: tells.where ?? ""
    };
}

// What kitty's drawer in Programs says it holds, from what was read ahead
// (VKCF3Z, 2026-11-08): its tabs and how many OS windows hold them, "7 in
// 3 windows", then how many repositories and machines it opens a window
// in, each left out at none, by its id in the launcher's words.
function held(open, repositories, machines, said) {
    const windows = new Set(open.map(tab => tab.window)).size;
    const inWindows = (windows === 1 ? said.inWindow : said.inWindows) ?? "";
    return [
        ["kitty:tabs", open.length === 0 ? ""
            : inWindows.replace("{count}", String(open.length)).replace("{windows}", String(windows))],
        ["kitty:repositories", repositories > 0 ? String(repositories) : ""],
        ["kitty:machines", machines > 0 ? String(machines) : ""]
    ].filter(fact => fact[1] !== "").map(fact => ({ id: fact[0], value: fact[1] }));
}

// The commands that go to a pane and close a tab or a pane, through the
// kitty that holds it, as its remote control listens (kitty.conf).
function socketOf(pid) {
    return `unix:@kitty-${pid}`;
}

function going(kitten, one) {
    const pane = one.panes ? one.panes[0] : one;
    return [kitten, "@", "--to", socketOf(one.pid), "focus-window", "--match", `id:${pane.id}`];
}

function closing(kitten, one) {
    const what = one.panes ? "close-tab" : "close-window";
    return [kitten, "@", "--to", socketOf(one.pid), what, "--match", `id:${one.id}`];
}
