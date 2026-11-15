// What polkit's prompt says for each ask, run in Qt's own JavaScript
// engine, the one the shell runs it in. The `polkits-prompt-says-what-runs`
// check copies Polkit.js beside this file and runs
// `qml -platform offscreen polkit.qml`. Each case gives the command line
// /proc keeps for the process calling polkit, NUL between its words, the
// action, polkit's message and details, and what the prompt has to say.
import QtQuick
import "Polkit.js" as Polkit

QtObject {
    function line(...words) {
        return words.map(word => `${word}\u0000`).join("");
    }

    Component.onCompleted: {
        const failures = [];
        let asked = 0;
        const expect = (what, ask, wanted) => {
            asked++;
            const got = `${Polkit.asker(ask.cmdline, ask.action)} | `
                + Polkit.reason(ask.message, ask.cmdline, ask.details ?? {});
            if (got !== wanted)
                failures.push(`${what} said\n  ${got}\nnot\n  ${wanted}`);
        };
        const units = "org.freedesktop.systemd1.manage-units";
        const exec = "org.freedesktop.policykit.exec";
        const transient = "Authentication is required to start transient unit 'run-u42.service'.";

        // An agent's run0, which systemd's words name only as a transient unit.
        expect("run0", {
            cmdline: line("/run/wrappers/bin/run0", "nix-collect-garbage", "-d"),
            action: units,
            message: transient
        }, "run0 nix-collect-garbage -d | To run it as root.");
        expect("run0 as another account", {
            cmdline: line("run0", "--unit=gc", "-u", "builder", "nix", "store", "gc"),
            action: units,
            message: transient
        }, "run0 --unit=gc -u builder nix store gc | To run it as builder.");
        expect("run0 with its account joined", {
            cmdline: line("run0", "--user=builder", "id"),
            action: units,
            message: transient
        }, "run0 --user=builder id | To run it as builder.");
        expect("run0 whose option takes the next word", {
            cmdline: line("run0", "--setenv", "A=b", "env"),
            action: units,
            message: transient
        }, "run0 --setenv A=b env | To run it as root.");
        expect("run0 keeping the account's own", {
            cmdline: line("run0", "--empower", "make", "install"),
            action: units,
            message: transient
        }, "run0 --empower make install | To run it with root's powers.");

        // pkexec, whose own words repeat the command with its store paths whole.
        expect("pkexec", {
            cmdline: line("pkexec", "/nix/store/0123456789abcdfghijklmnpqrsvwxyz-tlp-1.7/bin/tlp",
                "start"),
            action: exec,
            message: "Authentication is needed to run `/nix/store/0123456789abcdfghijklmnpqrsvwxyz"
                + "-tlp-1.7/bin/tlp start' as the super user",
            details: { user: "root" }
        }, "pkexec tlp-1.7/bin/tlp start | To run it as root.");
        expect("pkexec as another account", {
            cmdline: line("pkexec", "--user", "builder", "id"),
            action: exec,
            message: "Authentication is needed to run `id' as user builder",
            details: { user: "builder" }
        }, "pkexec --user builder id | To run it as builder.");

        // Anything else says polkit's own words for the action.
        expect("systemctl", {
            cmdline: line("systemctl", "start", "polkit-allowed.service"),
            action: units,
            message: "Authentication is required to start 'polkit-allowed.service'."
        }, "systemctl start polkit-allowed.service | To start 'polkit-allowed.service'.");
        expect("a word holding a space", {
            cmdline: line("timedatectl", "set-timezone", "America/New York"),
            action: "org.freedesktop.timedate1.set-timezone",
            message: "Authentication is required to set the system timezone"
        }, "timedatectl set-timezone 'America/New York' | To set the system timezone.");
        expect("a message that asks nothing", {
            cmdline: line("virsh", "start", "vm"),
            action: "org.libvirt.unix.manage",
            message: "System policy prevents management of local virtualized systems"
        }, "virsh start vm | System policy prevents management of local virtualized systems.");

        // An asker gone before the prompt read it is named by the action's owner.
        expect("an asker gone", {
            cmdline: "",
            action: units,
            message: "Authentication is required to start 'polkit-allowed.service'."
        }, "systemd | To start 'polkit-allowed.service'.");
        expect("libvirt, gone", { cmdline: "", action: "org.libvirt.unix.manage", message: "x" },
            "libvirt | X.");
        expect("pkexec, gone", { cmdline: "", action: exec, message: "y." }, "pkexec | Y.");
        expect("an action of no maker", { cmdline: "", action: "thing", message: "z" },
            "polkit | Z.");

        // A long command gives way at its middle, its start and its end kept,
        // and whole words left out rather than pieces of them: a piece of one
        // joined to a piece of another reads as a flag that isn't there. The
        // ellipsis holds to the words beside it, so no line can start or end
        // with it.
        const hash = "0123456789abcdfghijklmnpqrsvwxyz";
        const longs = [
            ["run0", "echo", ...Array(40).fill("abcdefg"), "end"],
            ["run0", "--setenv", "NIX_PATH=nixpkgs=flake:nixpkgs", "nix", "build", "--no-link",
                "--print-out-paths", "--rebuild", `/nix/store/${hash}-quickshell-0.3.2.drv^out`,
                `/nix/store/${hash}-qtdeclarative-6.12.0.drv^out`, "--keep-going"]];
        for (const words of longs) {
            const long = Polkit.asker(line(...words), units);
            const said = long.split(/[ \u00a0]/u);
            const whole = words.map(word => word.replace(`/nix/store/${hash}-`, ""));
            asked++;
            if (long.length > 120 || long.length < 100 || said[0] !== "run0"
                || said[said.length - 1] !== whole[whole.length - 1]
                || !long.includes("\u00a0\u2026\u00a0")
                || said.some(word => word !== "\u2026" && !whole.includes(word)))
                failures.push(`a long command said ${long}`);
        }
        // Past the longest, a runner's options give way first, as one, so the
        // program it runs and that program's first word follow the runner, as
        // they say what runs; a program run by none keeps its first word too
        // (VKCF3Z, 2026-11-15).
        const tail = Array(12).fill(`/nix/store/${hash}-qtdeclarative-6.12.0.drv^out`);
        const ahead = [
            [["run0", "--setenv", "NIX_PATH=nixpkgs=flake:nixpkgs", "--unit", "a-build", "--",
                "nix", "build", ...tail], "run0\u00a0\u2026\u00a0nix build"],
            [["pkexec", "--user", "joel", "--keep-cwd", "nix", "build", ...tail],
                "pkexec\u00a0\u2026\u00a0nix build"],
            [["run0", "nix", "build", ...tail], "run0 nix build"],
            [["systemctl", "restart", ...tail], "systemctl restart"]];
        for (const [argv, start] of ahead) {
            const long = Polkit.asker(line(...argv), units);
            asked++;
            if (!long.startsWith(start) || !/[ \u00a0]/u.test(long.charAt(start.length))
                || long.length > 120 || !long.endsWith("6.12.0.drv^out"))
                failures.push(`a long ${argv[0]} command said ${long}`);
        }
        // A word longer than a line gives way at its own middle, as it breaks
        // only at a space.
        const path = `/home/joel/${"d".repeat(60)}/end.qml`;
        const pathed = Polkit.asker(line("run0", "cat", path), units);
        asked++;
        if (pathed !== `run0 cat /home/joel/ddddddddd\u2026ddddddddddd/end.qml`)
            failures.push(`a command with a long word said ${pathed}`);

        // Lines break only at a command's spaces: its words' letters held
        // together, a path's slashes, a flag's dash and the ellipsis among them.
        const held = word => word.split("").join("\u2060");
        const joined = ["run0 /a/b -d", "tlp\u2026end x"];
        const wanted = [`${held("run0")} ${held("/a/b")} ${held("-d")}`,
            `${held("tlp\u2026end")} x`];
        for (const [at, text] of joined.entries()) {
            asked++;
            if (Polkit.joined(text) !== wanted[at])
                failures.push(`${text} was joined as ${escape(Polkit.joined(text))}`);
        }

        // The process shown: the subject a service asks for, pkexec itself for pkexec.
        const pids = [
            ["a service's", { "polkit.caller-pid": "1", "polkit.subject-pid": "42" }, "42"],
            ["pkexec's", { "polkit.caller-pid": "43", "polkit.subject-pid": "42",
                command_line: "id", program: "/run/current-system/sw/bin/id" }, "43"],
            ["none", {}, ""],
            ["no details", null, ""]
        ];
        for (const [what, details, wanted] of pids) {
            asked++;
            const got = Polkit.askerPid(details);
            if (got !== wanted)
                failures.push(`${what} pid was ${got}, not ${wanted}`);
        }

        // A command is shown as written, never read as rich text.
        const tagged = Polkit.plain("run0 echo '<b>a</b> & b'");
        asked++;
        if (tagged !== "run0 echo '&lt;b&gt;a&lt;/b&gt; &amp; b'")
            failures.push(`a tagged command said ${tagged}`);

        for (const failure of failures)
            console.log(failure);
        console.log(`${asked - failures.length} of ${asked} asks read as they should`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
