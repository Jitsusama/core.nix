// Vim's n and N, f and F, ; and , on the launcher's rows, which
// QueryKeys.js reads and Launcher.qml moves by. Not yet made.
.pragma library

function matchTo(going) {
    return { to: going.at, wrapped: "" };
}

function letterTo(going) {
    return going.at;
}

function again(last) {
    return last;
}
