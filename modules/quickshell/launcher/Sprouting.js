.pragma library

// Start New Work, Agents' act that begins new work from what Joel says: a
// link alone, or words in one of the repositories his open quests work in
// (Joel, 16 November: starting new work was too hard to do at all). It
// asks the agents daemon, which asks the realm's node for a `Sprout`, the
// request `mycelium sprout` makes, so the launcher and the command line
// never choose differently: a quest of its own, seeded from what he said,
// its session held. He asked for it, so its window opens for him as soon
// as it's up (Joel, 18 November: "start new work does not seem to work
// from the launcher", when it started in no window and said so only on
// the row he'd left); agents' own windows still go behind his.
// The daemon asks only for the shell (modules/agents/src/asker.rs), since
// the node takes it for Joel. The repositories come from the quests
// daemon (modules/quests/src/folders.rs).
//
// Only a sprout is asked, and only its own shape: what he said as one,
// and a folder only when it's a whole path.

// Whether a sprout pressed now is Joel's own Return in `launcher`, never
// a walk from outside (`qs ipc call launcher runWith`, `launcher do`),
// which any program of his can ask, an agent's among them. The agents
// daemon can't tell those apart, since the shell asks for both, so this
// is their only guard and fails closed: no launcher, or one that doesn't
// say it isn't walked, is no Return of his (Y2YBTS and ISXEG8 on #5456).
function fromJoel(launcher) {
    return launcher !== null && launcher !== undefined && launcher.cataloguing === false;
}

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

// How it ended, from the daemon's `answer` (modules/agents/src/sprout.rs):
// `{ started: true, machine, held }` once the node started it, `held` the
// id its holder keeps it by ("" where the node gave none), else
// `{ started: false, why }` in the node's or the daemon's words, or
// `said`'s where no daemon answered (null) or its answer can't be read.
function ended(answer, said) {
    if (answer === null || answer === undefined) {
        return { started: false, why: String(said.unanswered ?? "") };
    }
    const begun = answer.sprouted;
    if (typeof begun?.machine === "string" && begun.machine !== "") {
        return { started: true, machine: begun.machine,
            held: typeof begun.held === "string" ? begun.held : "" };
    }
    const why = answer.error?.message;
    if (typeof why === "string" && why !== "") {
        return { started: false, why };
    }
    return { started: false, why: String(said.unread ?? "") };
}

// The session among `running` (Sessions.running) that the sprout held by
// `held` started, or null before it's up: here by its holder's id, and on
// another machine, which keeps no holder's id here, by the words that
// attach it, which name that id.
function sproutedOf(running, held) {
    if (held === "") {
        return null;
    }
    return running.find(one => one.held === held
        || (Array.isArray(one.attach) && one.attach.includes(held))) ?? null;
}

// What the act that ran says of how it ended, in `said`'s words.
function told(how, said) {
    return how.started ? String(said.started ?? "").replace("{machine}", how.machine) : how.why;
}
