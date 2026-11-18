.pragma library

// Start New Work, Agents' act that begins new work from what Joel says: a
// link alone, or words in one of the repositories his open quests work in
// (Joel, 16 November: starting new work was too hard to do at all). It
// asks the agents daemon, which asks the realm's node for a `Sprout`, the
// request `mycelium sprout` makes, so the launcher and the command line
// never choose differently: a quest of its own, seeded from what he said,
// its session held in no window, which Go To opens when he wants it
// (pivot of 16 November, 18:55: no session needs a window of its own).
// The daemon asks only for the shell (modules/agents/src/asker.rs), since
// the node takes it for Joel.
//
// Only a sprout is asked, and only its own shape: what he said as one,
// and a folder only when it's a whole path.

// The ask that sprouts `text`, trimmed, in `folder` when one is given, or
// null when nothing is said or the folder isn't a whole path (refused
// says which).
function ask(text, folder) {
    if (refused(text, folder, null) !== null) {
        return null;
    }
    const where = String(folder ?? "");
    return Object.assign({ ask: "sprout", text: String(text).trim() },
        where !== "" ? { folder: where } : {});
}

// Why `ask` would ask nothing, in `said`'s words (`empty` or
// `unplaced`), or null when it asks; with `said` null, the key alone.
function refused(text, folder, said) {
    const where = String(folder ?? "");
    let why = null;
    if (String(text ?? "").trim() === "") {
        why = "empty";
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

// How it ended, from the daemon's `answer` (modules/agents/src/sprout.rs):
// `{ started: true, machine }` once the node started it, else
// `{ started: false, why }` in the node's or the daemon's words, or
// `said`'s where no daemon answered (null) or its answer can't be read.
function ended(answer, said) {
    if (answer === null || answer === undefined) {
        return { started: false, why: String(said.unanswered ?? "") };
    }
    const begun = answer.sprouted;
    if (typeof begun?.machine === "string" && begun.machine !== "") {
        return { started: true, machine: begun.machine };
    }
    const why = answer.error?.message;
    if (typeof why === "string" && why !== "") {
        return { started: false, why };
    }
    return { started: false, why: String(said.unread ?? "") };
}

// What the act that ran says of how it ended, in `said`'s words.
function told(how, said) {
    return how.started ? String(said.started ?? "").replace("{machine}", how.machine) : how.why;
}
