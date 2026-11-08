// What a new kitty window opens from Programs › kitty in the launcher
// (Kitty.qml): one of the repositories fd finds under ~/src, or ssh to one
// of the machines ~/.ssh/config names, each list most recent first, with
// the words each row says and the command that opens it. Kept apart from
// the QML so Qt's own engine runs it (tests/kitty-tabs.qml); Kitty.js
// reads kitty's own state.
//
// A machine shows every setting its own Host block gives it, and when it
// was last reached; a `Host *` block's settings, which are every
// machine's, aren't said on each. A repository is its path, as every
// folder is; zoxide's score for it isn't shown, since its order says it.
.pragma library
.import "Ago.js" as Ago
.import "Kitty.js" as Kitty

const MILLISECONDS_A_SECOND = 1000;

// A line of ~/.ssh/config: a keyword, then its value after spaces or an
// equals sign; and what a Host line's names are told apart by.
const SETTING = /^([A-Za-z]+)(?:\s*=\s*|\s+)(.*)$/;
const SPACES = /\s+/;
const QUOTES = /^"|"$/g;
const PATTERN = /[*?!]/;
// A line of zsh's extended history: when it started, how long it took,
// and the command.
const HISTORY = /^: (\d+):\d+;(.*)$/;
// The .git a repository's found by, as fd prints it.
const GIT = /\/?\.git\/?$/;

// The kitty behind the kitty window niri focused last, which a new window
// opens in, or 0 with none open.
function latest(niri) {
    const kitties = niri.filter(window => window.app_id === "kitty" && window.pid)
        .sort((a, b) => Kitty.focusedAt(b) - Kitty.focusedAt(a));
    return kitties.length > 0 ? kitties[0].pid : 0;
}

// A new kitty window, in a folder or running a command: through the kitty
// running (`pid`) as a new OS window of it, or a new kitty while none runs.
function opening(tools, pid, where) {
    const command = where.command ?? [];
    if (pid > 0) {
        const cwd = where.folder ? ["--cwd", where.folder] : [];
        return [tools.kitten, "@", "--to", Kitty.socketOf(pid), "launch", "--type=os-window"]
            .concat(cwd, command);
    }
    const directory = where.folder ? ["--directory", where.folder] : [];
    return [tools.kitty].concat(directory, command);
}

// The names a Host line gives, each without the quotes it may wear.
function hostsOf(value, found) {
    return value.split(SPACES).map(word => word.replace(QUOTES, ""))
        .filter(name => name !== "" && !PATTERN.test(name)
            && !found.some(one => one.name === name));
}

// A line's setting, as { keyword, value }, or null for a comment or none.
function settingOf(raw) {
    const line = raw.trim();
    const said = line.startsWith("#") ? null : SETTING.exec(line);
    return said ? { keyword: said[1], value: said[2].trim() } : null;
}

// Each machine ~/.ssh/config names on a Host line, in the order it names
// them, with its own block's settings: a pattern with a wildcard or a
// negation names no one machine, so it's passed over, and a Match block's
// settings are no Host's. An Include isn't followed.
function machines(text) {
    const found = [];
    let block = null;
    const settings = String(text ?? "").split("\n").map(settingOf).filter(one => one !== null);
    for (const setting of settings) {
        const keyword = setting.keyword.toLowerCase();
        if (keyword === "host") {
            block = { options: [] };
            for (const name of hostsOf(setting.value, found)) {
                found.push({ name, block });
            }
        } else if (keyword === "match") {
            block = null;
        } else if (block) {
            block.options.push(setting);
        }
    }
    return found.map(one => ({ name: one.name, options: one.block.options }));
}

// When each machine was last reached, in milliseconds since the epoch, as
// zsh's extended history keeps each command with when it started
// (`: <start>:<elapsed>;<command>`), by the name ssh was given.
function connected(history) {
    const last = {};
    for (const line of String(history ?? "").split("\n")) {
        const entry = HISTORY.exec(line);
        const args = entry ? Kitty.sshArgs(Kitty.unwrapped(entry[2].trim().split(SPACES))) : null;
        const name = args ? Kitty.destination(args) : "";
        if (name !== "") {
            last[name] = Math.max(last[name] ?? 0, Number(entry[1]) * MILLISECONDS_A_SECOND);
        }
    }
    return last;
}

// The machines, most recent first: one an ssh session is open to now,
// then each by when it was last reached, then the rest as written.
function machinesInOrder(all, last, now) {
    const at = one => now.includes(one.name) ? Number.POSITIVE_INFINITY : last[one.name] ?? -1;
    return all.map((one, written) => ({ one, written }))
        .sort((a, b) => at(b.one) - at(a.one) || a.written - b.written)
        .map(each => each.one);
}

// One of a machine's settings, by ssh's keyword in any case, or "".
function option(one, keyword) {
    const set = one.options.find(each => each.keyword.toLowerCase() === keyword);
    return set ? set.value : "";
}

// Where a machine's settings take ssh, as ssh would be told it by hand:
// its user, its address and its port, or "" where they say none of them.
function whereTo(one) {
    const user = option(one, "user");
    const port = option(one, "port");
    const address = option(one, "hostname");
    if (user === "" && port === "" && address === "") {
        return "";
    }
    const at = user === "" ? "" : `${user}@`;
    const on = port === "" ? "" : `:${port}`;
    return `${at}${address === "" ? one.name : address}${on}`;
}

// When a machine was reached, in its row's words: connected now while an
// ssh session to it is open (`known.open`), or how long ago it was last.
function reached(one, known, said) {
    if (known.open.includes(one.name)) {
        return said.now;
    }
    const last = known.last[one.name];
    return last ? Ago.ago(last, known.time) : "";
}

// What's said beside a machine: when it was reached, then where it goes,
// so a side cut short keeps when.
function machineSide(one, known, said) {
    return [reached(one, known, said), whereTo(one)].filter(part => part !== "")
        .join(" \u00b7 ");
}

// A machine's drawer: when it was last reached, then every setting its own
// block gives it, by ssh's own keyword, as written.
function machineDrawer(one, known, words) {
    const when = reached(one, known, words.said);
    return {
        title: one.name,
        facts: (when === "" ? [] : [{ label: words.named("connected"), value: when }])
            .concat(one.options.map(each => ({ label: each.keyword, value: each.value })))
    };
}

// The repositories fd finds under ~/src, each by the folder holding its
// .git: those zoxide knows first, in its order, where a visit to a folder
// inside one counts as the repository's, then the rest by path.
function repositories(found, visited) {
    const all = String(found ?? "").split("\n").map(line => line.replace(GIT, ""))
        .filter(path => path !== "");
    const seen = String(visited ?? "").split("\n").filter(path => path !== "");
    const rank = repository => {
        const at = seen.findIndex(path => path === repository
            || path.startsWith(`${repository}/`));
        return at < 0 ? seen.length : at;
    };
    return Array.from(new Set(all)).map(path => ({ path, rank: rank(path) }))
        .sort((a, b) => a.rank - b.rank || (a.path < b.path ? -1 : Number(a.path > b.path)))
        .map(each => each.path);
}
