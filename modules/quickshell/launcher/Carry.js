.pragma library

// How carrying a quest to another machine stands, read from what
// `mycelium carry <quest> --to <machine>` prints: one JSON object a line,
// each a step ("find", "push", "share", "start", "close") as it is
// "doing", "done", "skipped" or "failed", and last {"step": "carried"}
// naming each session carried there (mycelium's PROTOCOL.md). The foot
// says the step under way beside what's carried, then where it went or
// why it wasn't; what was done
// before a failure stays done, so nothing after one is read. Its words
// come from words.json, given by the caller, so this holds no wording.

function begun(quest, machine) {
    return {
        quest,
        machine,
        step: "",
        doing: null,
        done: "",
        ended: false,
        carried: false,
        failed: "",
        sessions: []
    };
}

// A step's words, a pushed tree named by its folder's last name.
function stepWords(words, step, tree) {
    const said = words.steps[step] ?? "";
    const folder = String(tree ?? "").replace(/\/+$/, "").split("/").pop();
    return said.replace("{tree}", folder);
}

function parsed(line) {
    try {
        const one = JSON.parse(line);
        return one !== null && typeof one === "object" && typeof one.step === "string"
            ? one : null;
    } catch (error) {
        return null;
    }
}

// The state once one more line is heard; a line that isn't a step, or
// anything after the carry has ended, changes nothing.
function heard(state, line) {
    const one = parsed(line);
    if (one === null || state.ended)
        return state;
    if (one.step === "carried")
        return Object.assign({}, state, {
            ended: true,
            carried: true,
            machine: one.machine ?? state.machine,
            sessions: Array.isArray(one.sessions) ? one.sessions : []
        });
    if (one.state === "failed")
        return Object.assign({}, state, {
            step: one.step,
            ended: true,
            failed: String(one.said ?? "")
        });
    if (one.state === "done")
        return Object.assign({}, state, { done: one.step });
    if (one.state !== "doing")
        return state;
    return Object.assign({}, state, { step: one.step, doing: one });
}

function filled(said, state) {
    return said.replace("{quest}", state.quest).replace("{machine}", state.machine);
}

// What the foot says: what's carried and the step under way, or once it
// has ended, that it was carried and where, or that it wasn't and why.
function told(state, words) {
    if (state.carried) {
        const many = state.sessions.length > 1;
        return [filled(words.carried, state),
            many ? words.thereMany.replace("{n}", state.sessions.length) : words.there];
    }
    // A failure says what was done before it, so nothing reads as undone that
    // isn't (VKCF3Z, 2026-11-07).
    if (state.ended) {
        const done = words.done?.[state.done] ?? "";
        return [filled(words.failed, state), state.failed].concat(done === "" ? [] : [done]);
    }
    const carrying = filled(words.carrying, state);
    if (state.doing === null)
        return [carrying];
    return [carrying, stepWords(words, state.doing.step, state.doing.tree)];
}

function ended(state) {
    return state.ended;
}

// mycelium ending before its last line is a failure, unless it had ended.
function stopped(state, words) {
    if (state.ended)
        return state;
    return Object.assign({}, state, { ended: true, failed: words.stopped });
}
