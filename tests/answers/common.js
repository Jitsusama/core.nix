// What more than one place's cases for the launcher's answers use: how a
// value is quoted in a failure, an instant from its minute in UTC, a row's
// parts as lines, the check every figure takes, and a file read beside
// ../answers.qml.
.pragma library

function quoted(value) {
    return JSON.stringify(value);
}

// An instant, from its minute in UTC written as ISO writes it.
function at(when) {
    return Date.parse(`${when}:00Z`);
}

// Each part as a line: its id, or a split's subnet or a character of
// several, then what it says and its note.
function parted(rows) {
    return rows.map(r => [r.id ?? r.name, r.side, r.note].filter(said => said).join(" "));
}

// A figure's check: counted as asked, and a failure saying what it gave
// when it isn't what's expected.
function checker(failures, ask) {
    return (what, got, expected) => {
        ask();
        if (quoted(got) !== quoted(expected)) {
            failures.push(`${what} gave ${quoted(got)}, not ${quoted(expected)}`);
        }
    };
}

// A file the check copies beside ../answers.qml, as its text, found from
// there since that's the file the check runs.
function read(fromRoot, file) {
    const request = new XMLHttpRequest();
    request.open("GET", fromRoot.url(file), false);
    request.send();
    return request.responseText;
}
