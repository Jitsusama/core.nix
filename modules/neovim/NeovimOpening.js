// How the launcher opens a file in Neovim (NeovimProgram.qml): the
// command, at a line and a column where it's given them, run in a new
// window of the kitty used last, through its remote control, in the
// folder it's opened from, or in a new kitty there while none runs. Kept
// apart from Quickshell's types, so the
// `the-launcher-acts-only-on-what-it-can` check runs it in Qt's own
// engine (tests/quick-acts.qml).
.pragma library

// Neovim given a file, at its line, and its column after that, as the
// cursor puts it, where they're given (0 for none).
function neovim(path, line, column) {
    if (line > 0 && column > 0) {
        return ["nvim", `+call cursor(${line}, ${column})`, path];
    }
    return line > 0 ? ["nvim", `+${line}`, path] : ["nvim", path];
}

// The kitty process whose window was used last, by niri's windows, or 0
// while none is open: the one its remote control listens for, as kitty's
// Recent opens a folder through (modules/kitty/home.nix).
function kitty(windows) {
    const focused = window => (window.focus_timestamp?.secs ?? 0) * 1e9
        + (window.focus_timestamp?.nanos ?? 0);
    const newest = windows.filter(window => window.app_id === "kitty" && (window.pid ?? 0) > 0)
        .sort((a, b) => focused(b) - focused(a))[0];
    return newest === undefined ? 0 : newest.pid;
}

// A command run in a new window of the kitty whose process is `pid`, in
// `folder`, or in a new kitty there for 0.
function inKitty(pid, folder, command) {
    if (pid > 0) {
        return ["kitty", "@", "--to", `unix:@kitty-${pid}`, "launch", "--type=os-window", "--cwd",
            folder].concat(command);
    }
    return ["kitty", "--directory", folder].concat(command);
}
