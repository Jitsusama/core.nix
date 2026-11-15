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

        // A long command gives way at its middle, its start and its end kept.
        const long = Polkit.asker(line("run0", "echo", "a".repeat(200), "end"), units);
        asked++;
        if (long.length !== 120 || !long.startsWith("run0 echo aaa") || !long.endsWith("aa end")
            || !long.includes("\u2026"))
            failures.push(`a long command said ${long}`);

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
