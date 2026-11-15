// What polkit's prompt says (Polkit.qml): who asks, and why. polkit tells
// its agent the pids of the process asking and of the one calling polkit
// for it, so who asks is a command line as the kernel keeps it (run0's,
// pkexec's, systemctl's), where a bare action could say only which program
// owns it. tests/polkit.qml says what each ask reads, in Qt's own engine.
.pragma library

// The longest command said whole. Past it, its middle gives way, keeping
// its start and its end, as a path does (docs/design.md, Space Available).
const longest = 120;
// The longest word said whole, a path most often: a line of the prompt's
// mono holds about 45 letters at its width, and a word breaks only at its
// spaces, so one longer gives way at its middle too (VKCF3Z, 2026-11-15).
const longestWord = 40;
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
// A word's letters, each but its last, which a word joiner holds to the
// next so Qt breaks a line only at a space, never after a path's slash or
// a flag's dash, nor beside the ellipsis a long command gives way at
// (VKCF3Z, 2026-11-15).
const letterInWord = /(?<letter>\S)(?=\S)/gu;
// The joiner itself, U+2060, which draws nothing.
const joiner = "\u2060";
// The ellipsis that stands for words left out, held to the words either
// side by no-break spaces, so a line never starts or ends with it.
const wordsLeftOut = "\u00a0\u2026\u00a0";
// The spaces a cut's start ends with and its end starts with, taken off by
// hand, as Qt's engine has no trimEnd or trimStart.
const spacesAfter = /\s+$/u;
const spacesBefore = /^\s+/u;
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
// Each runner's options that take the word after them; pkexec's only
// such is its user.
const runnerTakes = new Map([["run0", run0Takes], ["pkexec", new Set(["--user"])]]);
// The words of what runs said whatever gives way: its program and that
// program's first word, which say what runs (VKCF3Z, 2026-11-15).
const whatRuns = 2;

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
// past the longest said whole. A runner's options give way first, all of
// them as one, so what it runs follows it: they say how it runs, where the
// program and its first word say what (VKCF3Z, 2026-11-15).
function command(argv) {
    if (argv.length === 0) {
        return "";
    }
    const parts = [program(argv[0])].concat(argv.slice(1).map(shown))
        .map(word => middleCut(word, longestWord));
    const said = parts.join(" ");
    if (said.length <= longest) {
        return said;
    }
    const runs = programIndex(argv);
    if (runs === 0) {
        return wordsCut(parts, whatRuns);
    }
    if (runs === 1) {
        return wordsCut(parts, whatRuns + 1);
    }
    if (runs >= parts.length) {
        return wordsCut(parts, 1);
    }
    const runner = `${parts[0]}${wordsLeftOut}${parts[runs]}`;
    return wordsCut([runner].concat(parts.slice(runs + 1)), whatRuns);
}

// Where what a runner runs starts in its command: past the runner's options
// and the words they take, or past "--". A command no runner runs is what
// runs, from its first word.
function programIndex(argv) {
    const takes = runnerTakes.get(program(argv[0]));
    if (takes === undefined) {
        return 0;
    }
    for (let at = 1; at < argv.length; at += 1) {
        const word = argv[at];
        if (word === "--") {
            return at + 1;
        }
        if (!word.startsWith("-")) {
            return at;
        }
        if (takes.has(word)) {
            at += 1;
        }
    }
    return argv.length;
}

// Words said with spaces, those past `longest` letters given way from the
// middle. Each word is said whole or not at all, since a piece of one
// joined to a piece of another reads as a flag that isn't there
// (VKCF3Z, 2026-11-15). The first `kept` words are said whatever gives
// way; the rest are kept from each end in turn while they fit.
function wordsCut(parts, kept) {
    const said = parts.join(" ");
    if (said.length <= longest) {
        return said;
    }
    const front = parts.slice(0, kept);
    const back = [];
    const fits = (start, end) => start.join(" ").length + wordsLeftOut.length
        + end.join(" ").length <= longest;
    let first = kept;
    let last = parts.length - 1;
    let frontFits = true;
    let backFits = true;
    let fromBack = true;
    while (first <= last && (frontFits || backFits)) {
        fromBack = backFits && (fromBack || !frontFits);
        if (fromBack && fits(front, [parts[last]].concat(back))) {
            back.unshift(parts[last]);
            last -= 1;
        } else if (!fromBack && fits(front.concat([parts[first]]), back)) {
            front.push(parts[first]);
            first += 1;
        } else if (fromBack) {
            backFits = false;
        } else {
            frontFits = false;
        }
        fromBack = !fromBack;
    }
    return `${front.join(" ")}${wordsLeftOut}${back.join(" ")}`;
}

// A word past `most` letters, its middle given way to an ellipsis, its start
// and end kept. The ellipsis joins what it keeps either side, so a line
// never starts or ends with it.
function middleCut(text, most) {
    if (text.length <= most) {
        return text;
    }
    const kept = (most - 1) / 2;
    const start = text.slice(0, Math.ceil(kept)).replace(spacesAfter, "");
    const end = text.slice(text.length - Math.floor(kept)).replace(spacesBefore, "");
    return `${start}\u2026${end}`;
}

// Text whose lines break only at its spaces: each word's letters held
// together by a joiner.
function joined(text) {
    // By its place, as Qt's engine fills no named group (accountJoined).
    return text.replace(letterInWord, `$1${joiner}`);
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

// A quest's id as the harness gives it, its short last.
const questShape = /^QEST-\d{8}-(?<short>[A-Z0-9]{6})$/u;

// What the asking process says of itself, from asker-says's answer: the
// quest it says it runs under, by its short, as its own word, since any
// process can say any quest, so it's never proof of who asks; the command
// above it stays what's judged. Nothing for no answer, one not parsed, a
// process that says none, or a quest not in the harness's shape, which
// asker-says leaves out too, so no process puts words of its own here
// (ISXEG8, 2026-11-15).
function says(answer) {
    let said = null;
    try {
        said = JSON.parse(answer);
    } catch (_error) {
        return "";
    }
    const quest = questShape.exec(said?.quest ?? "");
    // Read by index, since Qt's engine has no named groups' object.
    return quest ? `It says it's from ${quest[1]}.` : "";
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
        const account = accountJoined.exec(word);
        if (account !== null) {
            return account[1];
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
