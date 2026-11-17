.pragma library

// Start New Work, Agents' act that begins new work from what Joel says: a
// link alone, or words in one of the repositories his open quests work in
// (Joel, 16 November: starting new work was too hard to do at all). It
// runs `mycelium sprout`, the same code a terminal runs, so the launcher
// and the command line never choose differently: a quest of its own,
// seeded from what he said, its session held in no window, which Go To
// opens when he wants it (pivot of 16 November, 18:55: no session needs a
// window of its own). The shell runs it itself, so the node judges who
// asks by Joel's own login.
//
// Only a sprout is asked, and only its own shape: what he said as one
// word after `--`, so nothing in it reads as an option, never `--join`,
// and a folder only when it's a whole path.
const REALM = /^[A-Za-z0-9][A-Za-z0-9._-]*$/u;
const SAID_BY = /^(?:mycelium(?: sprout)?|error):\s*/u;

// The words that ask `realm`'s node to sprout `text`, trimmed, in `folder`
// when one is given, or null when nothing is said, no realm can be asked,
// or the folder isn't a whole path (refused says which).
function command(realm, text, folder) {
    return refused(realm, text, folder, null) === null
        ? ["mycelium", "--realm", realm, "sprout", "-o", "json"].concat(
            folder ? ["--in", String(folder)] : [], ["--", String(text).trim()])
        : null;
}

// Why `command` would ask nothing, in `said`'s words (`empty`,
// `unrealmed` or `unplaced`), or null when it asks; with `said` null, the
// key alone.
function refused(realm, text, folder, said) {
    const where = String(folder ?? "");
    let why = null;
    if (String(text ?? "").trim() === "") {
        why = "empty";
    } else if (!REALM.test(String(realm ?? ""))) {
        why = "unrealmed";
    } else if (where !== "" && !where.startsWith("/")) {
        why = "unplaced";
    }
    if (why === null || said === null) {
        return why;
    }
    return String(said[why] ?? "");
}

// `path` as plain steps: no empty step, no `.`, and `..` taking back the
// step before, so two spellings of one folder read the same.
function plain(path) {
    const steps = [];
    for (const step of String(path).split("/")) {
        if (step === "..") {
            steps.pop();
        } else if (step !== "" && step !== ".") {
            steps.push(step);
        }
    }
    return `/${steps.join("/")}`;
}

// The repositories words can be said in: each folder `quests` work in,
// once, in their order, as `{ folder, name }`, its name read from `home`
// as ~ as a shell says it. Home itself never, since a session begun there
// has none of a repository's skills, nor one that isn't a whole path; and
// none while home isn't known, since home couldn't be told from them.
function folders(quests, home) {
    const said = String(home ?? "");
    if (!said.startsWith("/")) {
        return [];
    }
    const homeAt = plain(said);
    const seen = new Set();
    const found = [];
    for (const quest of quests ?? []) {
        const given = String(quest?.folder ?? "");
        const folder = given.startsWith("/") ? plain(given) : "";
        if (folder !== "" && folder !== homeAt && folder !== "/" && !seen.has(folder)) {
            seen.add(folder);
            const name = folder.startsWith(`${homeAt}/`)
                ? `~${folder.slice(homeAt.length)}` : folder;
            found.push({ folder, name });
        }
    }
    return found;
}

// What's been heard of a run, `heard`, with `stream` ("out" or "errors")
// heard to its end as `text`; a new record, the old one left as it was.
function hear(heard, stream, text) {
    return Object.assign({}, heard, { [stream]: String(text ?? "") });
}

// Whether both streams have ended, so how it ended can be said: a refusal
// before the node is asked comes on stderr alone, and the node's answer
// on stdout alone.
function whole(heard) {
    return typeof heard?.out === "string" && typeof heard?.errors === "string";
}

// How it ended, from what `mycelium sprout -o json` printed, `text`, and
// said on stderr, `errors`: `{ started: true, machine }` once the node
// started it, else `{ started: false, why }` in the node's words, or
// sprout's own where it refused before asking, or `said`'s where nothing
// came back or what came can't be read.
function ended(text, errors, said) {
    const printed = String(text ?? "").trim();
    if (printed === "") {
        return { started: false, why: reason(errors) || String(said.unanswered ?? "") };
    }
    let answer;
    try {
        answer = JSON.parse(printed);
    } catch (_unreadable) {
        answer = null;
    }
    const begun = answer?.Ok?.Started;
    if (typeof begun?.machine === "string" && begun.machine !== "") {
        return { started: true, machine: begun.machine };
    }
    if (typeof answer?.Err === "string" && answer.Err !== "") {
        return { started: false, why: answer.Err };
    }
    return { started: false, why: String(said.unread ?? "") };
}

// The reason in what was said on stderr: its last line said by mycelium
// or as an error, so a footer such as a usage hint isn't taken for it,
// else its last line; without who said it.
function reason(errors) {
    const lines = String(errors ?? "").split("\n").map(line => line.trim())
        .filter(line => line !== "");
    const said = lines.filter(line => SAID_BY.test(line));
    // pop(), since Qt's engine has no at().
    return ((said.length > 0 ? said : lines).pop() ?? "").replace(SAID_BY, "");
}

// How a run that never ended its streams ended: it couldn't be run at
// all, or its node took longer than the launcher waits.
function unended(lasted, said) {
    return { started: false, why: String((lasted ? said.unanswered : said.unrun) ?? "") };
}

// What the act that ran says of how it ended, in `said`'s words.
function told(how, said) {
    return how.started ? String(said.started ?? "").replace("{machine}", how.machine) : how.why;
}
