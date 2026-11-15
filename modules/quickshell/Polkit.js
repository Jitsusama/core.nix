// What polkit's prompt says (Polkit.qml): who asks, and why. polkit tells
// its agent the pids of the process asking and of the one calling polkit
// for it, so who asks is a command line as the kernel keeps it (run0's,
// pkexec's, systemctl's), where a bare action could say only which program
// owns it. tests/polkit.qml says what each ask reads, in Qt's own engine.
.pragma library

// The longest command said whole. Past it, its middle gives way, keeping
// its start and its end, as a path does (docs/design.md, Space Available).
const longest = 120;
// What a store path's hash and dash take, so a program is named by its
// package rather than its hash.
const storeHash = /\/nix\/store\/[0-9a-z]{32}-/gu;
// An action's id is its maker's reversed domain, then its program, then
// the action: org.freedesktop.systemd1.manage-units.
const programAt = 2;
// A D-Bus name's version, which its program's name drops.
const version = /\d+$/u;
// A word a space would split, said between quotes.
const spaced = /\s/u;
// run0's account given in the same word as its option.
const accountJoined = /^(?:--user=|-u)(?<account>.+)$/u;
// polkit's words for asking, which the prompt's own reason drops.
const askedFor = /^authentication is (?:required|needed) (?:in order )?to /iu;
// A sentence's end.
const ended = /[.!?]$/u;
// The programs that run a command as another account, each saying so.
const runners = ["run0", "pkexec"];
// run0's options that take the word after them, so that word is no
// command; the account is the user option's.
const run0Takes = new Set([
    "-u", "--user", "-g", "--group", "--unit", "-p", "--property", "--description",
    "--slice", "--nice", "-D", "--chdir", "--setenv", "--background", "--machine", "--area"
]);

// A command line as /proc keeps it, each word ended by a NUL, as words.
function words(cmdline) {
    return (cmdline || "").split("\u0000").filter(word => word !== "");
}

// The program a word names: its file's name.
function program(word) {
    return word.slice(word.lastIndexOf("/") + 1);
}

// A word as it's shown: a store path without its hash, a word holding a
// space between single quotes.
function shown(word) {
    const bare = word.replace(storeHash, "");
    return spaced.test(bare) ? `'${bare}'` : bare;
}

// A command, its program by name and its words as shown, cut at its middle
// past the longest said whole.
function command(argv) {
    if (argv.length === 0) {
        return "";
    }
    const said = [program(argv[0])].concat(argv.slice(1).map(shown)).join(" ");
    if (said.length <= longest) {
        return said;
    }
    const kept = (longest - 1) / 2;
    return `${said.slice(0, Math.ceil(kept))}\u2026${said.slice(said.length - Math.floor(kept))}`;
}

// The program owning an action, for an ask whose asker can't be read: its
// name after its maker's reversed domain, without the version a D-Bus name
// ends in. libvirt's is named for the socket it guards,
// org.libvirt.unix.manage, as a VM on the system's connection asks.
function owner(actionId) {
    const parts = (actionId || "").split(".");
    const known = { policykit: "pkexec", login: "logind", unix: "libvirt" };
    const name = parts.length > programAt + 1 ? parts[programAt].replace(version, "") : "";
    return known[name] ?? (name || "polkit");
}

// The process whose command is shown. A service such as systemd calls
// polkit for the program that asked it, run0 or systemctl, which polkit
// names as its subject. pkexec calls polkit for itself, naming the shell
// that started it as the subject, so for its ask, the one whose details
// hold the command it runs, the caller is the one shown.
function askerPid(details) {
    const pid = details?.command_line === undefined ? details?.["polkit.subject-pid"]
        : details["polkit.caller-pid"];
    return pid ?? "";
}

// Who asks: the command asking, or the action's owner where it's
// gone or can't be read.
function asker(cmdline, actionId) {
    return command(words(cmdline)) || owner(actionId);
}

// Text as rich text shows it, as written: a command is the asker's to
// choose, so a tag in it would otherwise restyle the prompt or hide words.
function plain(text) {
    return text.replace(/&/gu, "&amp;").replace(/</gu, "&lt;").replace(/>/gu, "&gt;");
}

// The account run0 runs as: its user option's, root without one, or null
// where it runs with root's powers as the account asking (--empower).
function run0As(argv) {
    for (let at = 1; at < argv.length; at += 1) {
        const word = argv[at];
        if (word === "--" || !word.startsWith("-")) {
            return "root";
        }
        if (word === "--empower") {
            return null;
        }
        // Qt's engine names no groups, so the account is read by its place.
        const joined = accountJoined.exec(word);
        if (joined !== null) {
            return joined[1];
        }
        if (word === "-u" || word === "--user") {
            return argv[at + 1] ?? "root";
        }
        if (run0Takes.has(word)) {
            at += 1;
        }
    }
    return "root";
}

// polkit's message for an action says what it's for as "Authentication is
// required to do this"; the prompt says "To do this."
function reworded(message) {
    const trimmed = (message || "").trim();
    const rest = trimmed.replace(askedFor, "");
    const sentence = rest.charAt(0).toUpperCase() + rest.slice(1);
    const said = rest === trimmed ? sentence : `To ${rest}`;
    return ended.test(said) ? said : `${said}.`;
}

// Why it asks. A runner's own words name a transient unit, or the command
// again with its store paths whole, so a runner's ask says only the
// account the command runs as, which the command above it doesn't; pkexec
// names it in its details. Every other ask says polkit's message.
function reason(message, cmdline, details) {
    const argv = words(cmdline);
    const runner = argv.length > 0 ? program(argv[0]) : "";
    if (!runners.includes(runner)) {
        return reworded(message);
    }
    const account = runner === "pkexec" ? details?.user ?? "root" : run0As(argv);
    return account === null ? "To run it with root's powers." : `To run it as ${account}.`;
}
