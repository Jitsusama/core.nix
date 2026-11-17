.pragma library

// Start a Cell, Agents' act that begins new work from what Joel says: a
// link alone, or words in one of the repositories his open quests work in
// (Joel, 16 November: starting new work was too hard to do at all). The
// realm's node is asked to sprout it, as `mycelium sprout` does from a
// terminal: a quest of its own, seeded from what he said, its session
// held in no window, which Go To opens when he wants it (pivot of
// 16 November, 18:55: no session needs a window of its own). The shell
// asks it itself, so the node judges who asks by Joel's own login.
//
// Only a sprout is asked, and only its own shape: what he said as one
// word after `--`, so nothing in it reads as an option, never `--join`,
// and a folder only when it's a whole path.
const REALM = /^[A-Za-z0-9._-]+$/u;
const ENDING_SLASHES = /\/+$/u;
const FOLDER_ENDING_SLASHES = /(?<kept>.)\/+$/u;
const SAID_BY_MYCELIUM = /^mycelium:\s*/u;

// The words that ask `realm`'s node to sprout `text`, trimmed, in `folder`
// when one is given, or null when nothing is said, no realm can be asked,
// or the folder isn't a whole path.
function command(realm, text, folder) {
    const said = String(text ?? "").trim();
    const where = String(folder ?? "");
    if (said === "" || !REALM.test(String(realm ?? "")) || (where !== "" && !where.startsWith("/"))) {
        return null;
    }
    const within = where === "" ? [] : ["--in", where];
    return ["mycelium", "--realm", realm, "sprout", "-o", "json"].concat(within, ["--", said]);
}

// The repositories words can be said in: each folder `quests` work in,
// once, in their order, as `{ folder, name }`, its name read from `home`
// as ~ as a shell says it. Home itself never, since a session begun there
// has none of a repository's skills, nor one that isn't a whole path.
function folders(quests, home) {
    const homeAt = String(home ?? "").replace(ENDING_SLASHES, "");
    const seen = new Set();
    const found = [];
    for (const quest of quests ?? []) {
        // A function, since Qt's engine leaves `$<kept>` unexpanded.
        const folder = String(quest?.folder ?? "")
            .replace(FOLDER_ENDING_SLASHES, (_ending, kept) => kept);
        if (folder.startsWith("/") && folder !== homeAt && !seen.has(folder)) {
            seen.add(folder);
            const name = homeAt !== "" && folder.startsWith(`${homeAt}/`)
                ? `~${folder.slice(homeAt.length)}` : folder;
            found.push({ folder, name });
        }
    }
    return found;
}

// How it ended, from what `mycelium sprout -o json` printed, `text`, and
// said on stderr, `errors`: `{ started: true, machine }` once the node
// started it, else `{ started: false, why }` in the node's words, or
// sprout's own last line where it refused before asking, or `said`'s
// where nothing came back or what came can't be read.
function ended(text, errors, said) {
    const printed = String(text ?? "").trim();
    if (printed === "") {
        const lines = String(errors ?? "").split("\n").map(line => line.trim())
            .filter(line => line !== "");
        // pop(), since Qt's engine has no at().
        const last = (lines.pop() ?? "").replace(SAID_BY_MYCELIUM, "");
        return { started: false, why: last === "" ? said.unanswered ?? "" : last };
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
    return { started: false, why: said.unread ?? "" };
}
