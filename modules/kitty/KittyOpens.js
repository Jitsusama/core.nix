// What a new kitty window opens from Programs › kitty in the launcher
// (Kitty.qml): one of the repositories under ~/src, most recent
// first, or ssh to one of the machines ~/.ssh/config and
// ~/.ssh/known_hosts name, most used first, with the words each row says
// and the command that opens it. Kept apart from the QML so Qt's own
// engine runs it (tests/kitty-tabs.qml); Kitty.js reads kitty's own state.
//
// A machine shows which file named it, how many times zsh's history ran
// ssh to it and when last, every setting its own Host block gives it, and
// the kinds of key known_hosts holds for it, and the other names a
// known_hosts line gives it. Left out: a `Host *` block's settings, which
// are every machine's, so say nothing of one; the keys themselves, whose
// base64 nobody reads and whose fingerprint would need ssh-keygen run for
// each; a known_hosts line's comment, which ssh writes none of; a hashed
// line (HashKnownHosts), which hides its name by design, and an
// @cert-authority or @revoked line, which names a signer or a key
// refused rather than a machine reached; and a Host block's own
// UserKnownHostsFile, whose keys aren't said, since each block may name
// another. A repository is its path, as every folder is, and says when
// zoxide last saw it, which the programs daemon reads from zoxide's own
// database, since `query` prints only a score; that score is left out, since its frecency has no
// unit Joel could read.
//
// Each figure of a machine is from a file read ahead of the launcher
// opening, so nothing here asks the network or ssh-agent anything.
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
// A known_hosts name with its port, as ssh writes one off port 22.
const BRACKETED = /^\[([^\]]+)\]:(\d+)$/;
// The port ssh reaches when nothing names one.
const SSH_PORT = "22";
// The .git a repository's found by, where a line names it.
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
function configured(text) {
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

// A known_hosts name as { name, port }, without the brackets ssh puts
// round one off port 22.
function knownName(word) {
    const bracketed = BRACKETED.exec(word);
    return bracketed ? { name: bracketed[1], port: bracketed[2] } : { name: word, port: SSH_PORT };
}

// Each machine ~/.ssh/known_hosts names, in the order it first names it,
// as { name, port, also, keys }: the first name of a line, the other names
// it gives the same machine (ssh adds its address beside its name with
// CheckHostIP), and each kind of key held for it, one a line. A hashed
// line, a marked one and a pattern are passed over, as the header says.
function known(text) {
    const found = [];
    for (const raw of String(text ?? "").split("\n")) {
        const [names, kind] = raw.trim().split(SPACES);
        const passed = (names ?? "") === "" || (kind ?? "") === "" || PATTERN.test(names)
            || ["#", "|", "@"].includes(names[0]);
        if (!passed) {
            const [first, ...others] = names.split(",").map(knownName);
            let one = found.find(each => each.name === first.name && each.port === first.port);
            if (!one) {
                one = { name: first.name, port: first.port, also: [], keys: [] };
                found.push(one);
            }
            one.also = Array.from(new Set(one.also.concat(others.map(other => other.name))));
            one.keys = Array.from(new Set(one.keys.concat([kind])));
        }
    }
    return found;
}

// Where ssh reaches a machine ~/.ssh/config names: its HostName, else its
// name, on its Port, else 22.
function reachedAt(one) {
    return { name: option(one, "hostname") || one.name, port: option(one, "port") || SSH_PORT };
}

// Every machine to ssh to, as { name, options, port, also, keys, from }:
// those ~/.ssh/config names, as written, each with the keys known_hosts
// holds where ssh reaches it; then each other machine known_hosts names,
// where ssh has been without its config naming it. `from` says which file
// named it, by its words' id; a known_hosts machine off port 22 keeps the
// port, which ssh is told.
function machines(config, hosts) {
    const all = known(hosts);
    const own = configured(config).map(one => {
        const at = reachedAt(one);
        const key = all.find(each => each.port === at.port
            && (each.name === at.name || each.also.includes(at.name)));
        const keys = key ? key.keys : [];
        return Object.assign(one, { port: "", also: [], keys, from: "config" });
    });
    const named = own.reduce((names, one) => names.concat([one.name, reachedAt(one).name]), []);
    const others = all.filter(each => !(named.includes(each.name)
        || each.also.some(name => named.includes(name))))
        .map(each => ({
            name: each.name,
            options: [],
            port: each.port === SSH_PORT ? "" : each.port,
            also: each.also,
            keys: each.keys,
            from: "known"
        }));
    return own.concat(others);
}

// The command that opens ssh to a machine, through kitty's ssh kitten, by
// the name it's listed by, and its port where known_hosts gave one.
function sshing(kitten, one) {
    const port = one.port === "" ? [] : ["-p", one.port];
    return [kitten, "ssh"].concat(port, [one.name]);
}

// How many times zsh's history ran ssh to each machine, and when it last
// did, in milliseconds since the epoch, as zsh's extended history keeps
// each command with when it started (`: <start>:<elapsed>;<command>`), by
// the name ssh was given: { <name>: { times, last } }.
function connected(history) {
    const reached = {};
    for (const line of String(history ?? "").split("\n")) {
        const entry = HISTORY.exec(line);
        const args = entry ? Kitty.sshArgs(Kitty.unwrapped(entry[2].trim().split(SPACES))) : null;
        const name = args ? Kitty.destination(args) : "";
        if (name !== "") {
            const was = reached[name] ?? { times: 0, last: 0 };
            reached[name] = {
                times: was.times + 1,
                last: Math.max(was.last, Number(entry[1]) * MILLISECONDS_A_SECOND)
            };
        }
    }
    return reached;
}

// What the history says of one machine, by every name it has: how many
// times it was reached, and when last.
function reachedBy(one, reached) {
    return [one.name].concat(one.also).reduce((all, name) => {
        const by = reached[name] ?? { times: 0, last: 0 };
        return { times: all.times + by.times, last: Math.max(all.last, by.last) };
    }, { times: 0, last: 0 });
}

// The machines, most used first: by how many times the history reached
// each, then by when it last did, then as written.
function machinesInOrder(all, reached) {
    const by = all.map((one, written) => ({ one, written, use: reachedBy(one, reached) }));
    return by.sort((a, b) => b.use.times - a.use.times || b.use.last - a.use.last
        || a.written - b.written).map(each => each.one);
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

// Where a machine's settings take ssh, or the port known_hosts gave it.
function goesTo(one) {
    return one.port === "" ? whereTo(one) : `${one.name}:${one.port}`;
}

// The settings its Address says, as ssh's keywords spell them, which its
// drawer says once there rather than again apart.
const ADDRESSED = ["hostname", "user", "port"];

// When a machine was reached, in its row's words: connected now while an
// ssh session to it is open (`state.open`), or how long ago it was last.
function lastReached(one, state, said) {
    if ([one.name].concat(one.also).some(name => state.open.includes(name))) {
        return said.now;
    }
    const { last } = reachedBy(one, state.reached);
    return last ? Ago.ago(last, state.time) : "";
}

// How many times it was reached, in its row's words, or "" for never.
function timesOf(one, state, said) {
    const { times } = reachedBy(one, state.reached);
    if (times === 0) {
        return "";
    }
    return times === 1 ? said.once : Kitty.fill(said.times, { count: times });
}

// What's said beside a machine: when it was reached and how many times,
// which order the list, or that it never was. Its address is its
// drawer's, said once there rather than again in a side too long to fit
// beside it (VKCF3Z, 2026-11-08).
function machineSide(one, state, said) {
    const use = [lastReached(one, state, said), timesOf(one, state, said)]
        .filter(part => part !== "");
    return use.length > 0 ? use.join(" \u00b7 ") : said.never ?? "";
}

// A machine's drawer: over its name, the file naming it and what it is,
// "in ~/.ssh/config \u00b7 machine", in Note since a path is the world's
// (VKCF3Z, 2026-11-08); then when it was last reached and how many times,
// its Address, as ssh would be told it by hand and in the Name face, as an
// identifier is, rather than its HostName, User and Port apart, every
// other setting its own block gives it, by ssh's own keyword, as written,
// and its other names and kinds of key.
function machineDrawer(one, state, words) {
    const { said } = words;
    const named = fact => ({ label: words.named(fact[0]), value: fact[1] });
    const facts = [
        ["connected", lastReached(one, state, said)],
        ["times", timesOf(one, state, said)]
    ].map(named)
        .concat([Object.assign(named(["address", goesTo(one)]), { identifies: true })])
        .concat(one.options.filter(each => !ADDRESSED.includes(each.keyword.toLowerCase()))
            .map(each => ({ label: each.keyword, value: each.value })))
        .concat([
            ["also", one.also.join(" \u00b7 ")],
            ["keys", one.keys.join(" \u00b7 ")]
        ].map(named));
    return {
        eyebrow: Kitty.fill(said.eyebrow ?? "", { file: said[one.from] ?? "" }),
        eyebrowNames: true,
        title: one.name,
        facts: facts.filter(fact => fact.value !== "")
    };
}

// A folder as Joel says it, from ~ where it's in his home.
function fromHome(folder, home) {
    const inHome = home !== "" && (folder === home || folder.startsWith(`${home}/`));
    return inHome ? `~${folder.slice(home.length)}` : folder;
}

// What's said beside a repository: the folder holding it, said from ~,
// where that isn't `places.src`, which every row would say again, then
// when zoxide last saw it or a folder inside it, as of `when.now`, or
// `when.never` where it never has, so the row isn't bare (VKCF3Z,
// 2026-11-08).
function repositorySide(path, places, seen, when) {
    const { now, never } = when;
    const parent = path.slice(0, path.lastIndexOf("/"));
    const folder = parent === places.src ? "" : fromHome(parent, places.home);
    const last = Object.keys(seen ?? {})
        .filter(each => each === path || each.startsWith(`${path}/`))
        .reduce((most, each) => Math.max(most, seen[each]), 0);
    return [folder, last > 0 ? Ago.ago(last, now) : never ?? ""].filter(part => part !== "")
        .join(" \u00b7 ");
}

// The repositories under ~/src, each by its folder or the folder holding
// its .git: those zoxide knows first, in its order, where a visit to a folder
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
