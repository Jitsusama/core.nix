// The launcher's command line, the : line, apart from drawing it
// (CommandLine.qml), so tests/command-line.qml holds it in Qt's own engine.
.pragma library

const standIn = {};

function notation(command, short) {
    return "";
}

function listed(from) {
    return [];
}

function narrowed(menu, typed) {
    return [];
}

function typing(typed) {
    return { typed, at: -1, line: typed };
}

function chosen(choosing, step, menu) {
    return choosing;
}

function dropped(choosing) {
    return choosing;
}

function taken(choosing) {
    return choosing;
}

function ran(line, menu) {
    return { error: "" };
}
