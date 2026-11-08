.pragma library

// How carrying a quest to another machine stands, read from what
// `mycelium carry` prints. Not written yet.

function begun(quest, machine) {
    return { quest, machine };
}

function heard(state) {
    return state;
}

function told() {
    return [];
}

function ended() {
    return false;
}

function stopped(state) {
    return state;
}
