// kitty's own state as Programs › kitty lists it, run in Qt's own
// JavaScript engine, the one the shell runs it in: what open-now says of
// each kitty read whole or passed over, each kitty OS window matched to
// niri's window, the tabs in the order they were used, what runs in each
// pane through what only starts it, an ssh session's machine, each row's
// words and drawer, and the commands that go to one, close one and open a
// new window (Kitty.js); and the machines ~/.ssh/config and
// ~/.ssh/known_hosts name, how often and when each was reached, their
// order, their rows and drawers, the ssh that opens one, and the
// repositories under ~/src in zoxide's order (KittyOpens.js), as the
// programs daemon reads each file they come from. The
// `kitty-lists-its-tabs` check copies both, with the launcher's Ago.js,
// beside this file and runs `qml -platform offscreen kitty-tabs.qml`.
import QtQuick
import "Kitty.js" as Kitty
import "KittyOpens.js" as Opens

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    // One of kitty's windows, a pane, as its `ls` says it, with what's in
    // front, each [its id, its command line, its folder, its group], in
    // `cwd`.
    function pane(id, cwd, front, more) {
        return Object.assign({
            id: id,
            is_active: false,
            title: "zsh",
            pid: 100 + id,
            cwd: cwd,
            cmdline: ["-zsh"],
            last_reported_cmdline: "",
            last_cmd_exit_status: 0,
            at_prompt: false,
            in_alternate_screen: false,
            lines: 47,
            columns: 120,
            created_at: 1794099697431705746,
            needs_attention: false,
            has_activity_since_last_focus: false,
            session_name: "",
            user_vars: {},
            env: { SECRET: "never shown" },
            foreground_processes: front.map(one => ({
                pid: one[0], cmdline: one[1], cwd: one[2] ?? cwd, pgrp: one[3]
            }))
        }, more ?? {});
    }

    function run() {
        const failures = [];
        let asked = 0;
        const check = (what, said, meant) => {
            asked += 1;
            if (JSON.stringify(said) !== JSON.stringify(meant))
                failures.push(
                    `${what}: said ${JSON.stringify(said)}, meant ${JSON.stringify(meant)}`);
        };
        const home = "/home/joel";
        const said = {
            active: "active", rang: "rang", output: "new output", closed: "closed",
            panes: "{count} panes", ssh: "ssh {machine}", tab: "tab {index} of {count}"
        };

        // open-now's lines: a kitty a line, one not answering whole passed
        // over, and nothing for nothing.
        check("nothing heard", Kitty.heard(""), []);
        check("each kitty heard",
            Kitty.heard("10\t[]\n11\t[{\"id\":1}]\nbroken\t[\n12\tnot json\n")
            .map(kitty => [kitty.pid, kitty.windows.length]), [[10, 0], [11, 1]]);

        // What runs: the leader of what's in front, through what only starts
        // it, by its own name, and over ssh the machine.
        const runs = (front, cmdline) => {
            const own = cmdline ? { cmdline: cmdline } : {};
            const runs = Kitty.running(pane(1, "/tmp", front, own));
            return [runs.program, runs.machine];
        };
        check("a shell at its prompt", runs([[5, ["-zsh"]]]), ["zsh", ""]);
        check("Neovim, wrapped", runs([[5, ["/nix/store/x/bin/.nvim-wrapped", "a.nix"]]]),
            ["nvim", ""]);
        // The group's leader, though an id that wrapped puts what it started
        // first; else the pane's own process; else the lowest.
        check("the leader, not pi's children", runs([[3, ["git-engine", "-s", "1"], "/", 5],
            [5, ["pi"], "/", 5]]), ["pi", ""]);
        check("the pane's own, its group unknown", runs([[3, ["git-engine"]], [101, ["pi"]]]),
            ["pi", ""]);
        check("the lowest, knowing nothing more", runs([[7, ["less"]], [3, ["git", "log"]]]),
            ["git", ""]);
        check("a session mycelium holds", runs([[5, ["mycelium", "--realm", "work", "hold", "--",
            "pi", "--session", "01a1"]]]), ["pi", ""]);
        check("a script by its name", runs([[5, ["node", "--max-old-space-size=4096",
            "/nix/store/x/bin/pi"]]]), ["pi", ""]);
        check("a shell's command by its first word", runs([[5, ["sh", "-c", "make test"]]]),
            ["make", ""]);
        check("env and its settings", runs([[5, ["env", "-u", "X", "TERM=dumb", "nvim"]]]),
            ["nvim", ""]);
        check("nothing in front, the pane's own", runs([], ["-zsh"]), ["zsh", ""]);
        check("ssh", runs([[5, ["ssh", "-tt", "-J", "jump", "-p22", "joel@devbox", "uptime"]]]),
            ["ssh", "devbox"]);
        check("the ssh kitten", runs([[5, ["/nix/store/x/bin/kitten", "ssh", "--kitten",
            "login_shell=zsh", "ssh://joel@build.example:2222"]], [6, ["ssh", "-t", "x"]]]),
            ["ssh", "build.example"]);
        check("kitty's ssh kitten", runs([[5, ["kitty", "+kitten", "ssh", "-o", "Port=1",
            "-l", "joel", "[::1]"]]]), ["ssh", "::1"]);
        check("pi's git over ssh isn't a session", runs([[5, ["pi"]], [8, ["ssh",
            "git@github.com", "git-upload-pack"]]]), ["pi", ""]);

        // Two kitties: the first with two OS windows, which niri numbers as
        // kitty does, one of them titled as the other's; the second with a
        // hidden one niri never shows. niri focused the second's last.
        const tab = (id, active, windows, more) => Object.assign({
            id: id, is_active: active, title: windows[0].title, title_overridden: false,
            windows: windows, active_window_history: windows.map(one => one.id)
        }, more ?? {});
        const first = { pid: 10, windows: [
            { id: 1, active_tab_history: [3, 1], tabs: [
                tab(1, false, [pane(1, `${home}/src/core.nix`, [[51, ["nvim", "home.nix"]]],
                    { title: "\ud83d\udcdd modules/kitty/home.nix", columns: 80, lines: 24,
                        user_vars: { neovim_server: "/run/user/1000/nvim.51.0" } })]),
                tab(2, true, [pane(2, `${home}/src/core.nix`, [[52, ["-zsh"]]],
                    { is_active: true, at_prompt: true, last_reported_cmdline: "make test",
                        last_cmd_exit_status: 2 })]),
                tab(3, false, [pane(3, home, [[53, ["-zsh"]]]),
                    pane(4, "/tmp", [[54, ["kitten", "ssh", "devbox"]]],
                        { is_active: true, needs_attention: true, title: "joel@devbox: ~" })],
                    { title: "Deploys", title_overridden: true, active_window_history: [3, 4] })
            ] },
            { id: 2, active_tab_history: [], tabs: [
                tab(4, true, [pane(5, home, [[55, ["-zsh"]]],
                    { is_active: true, has_activity_since_last_focus: true })])
            ] }
        ] };
        const second = { pid: 20, windows: [
            { id: 1, tabs: [tab(1, true, [pane(1, home, [[61, ["sleep", "infinity"]]],
                { title: "sleep" })])] },
            { id: 2, tabs: [tab(2, true, [pane(2, `${home}/world`, [[62, ["mycelium", "hold",
                "--", "pi"]]], { title: "\u03c0 - Faster Refs - world", is_active: true })])] }
        ] };
        const niri = [
            { id: 31, pid: 10, app_id: "kitty", title: "zsh",
                focus_timestamp: { secs: 100, nanos: 0 } },
            { id: 32, pid: 10, app_id: "kitty", title: "zsh",
                focus_timestamp: { secs: 50, nanos: 0 } },
            { id: 41, pid: 20, app_id: "kitty", title: "\u03c0 - Faster Refs - world",
                focus_timestamp: { secs: 200, nanos: 0 } },
            { id: 50, pid: 30, app_id: "google-chrome", title: "zsh" }
        ];
        const all = Kitty.tabs([first, second], niri);
        // The window niri focused last first, its hidden twin left out; then
        // each OS window's tabs as they were used: the active, then kitty's
        // history newest first, then the rest in the bar's order.
        check("the tabs in order", all.map(one => [one.key, one.window, one.index, one.count]),
            [["kitty:tab:20:2", 41, 1, 1], ["kitty:tab:10:2", 31, 2, 3],
                ["kitty:tab:10:1", 31, 1, 3], ["kitty:tab:10:3", 31, 3, 3],
                ["kitty:tab:10:4", 32, 1, 1]]);
        check("a tab's panes, the active first", all[3].panes.map(one => [one.key, one.window]),
            [["kitty:pane:10:4", 31], ["kitty:pane:10:3", 31]]);
        check("one of two OS windows of one title shown names neither",
            Kitty.tabs([first], niri.slice(1, 2)), []);
        check("none shown, none listed", Kitty.tabs([first, second], []), []);

        // Its drawer in Programs: its tabs and the OS windows holding them,
        // then its repositories and machines, each left out at none.
        const held = { inWindow: "{count} in 1 window", inWindows: "{count} in {windows} windows" };
        check("what kitty holds", Kitty.held(all, 34, 12, held),
            [{ id: "kitty:tabs", value: "5 in 3 windows" },
                { id: "kitty:repositories", value: "34" }, { id: "kitty:machines", value: "12" }]);
        check("one OS window's", Kitty.held(all.slice(1, 4), 0, 0, held),
            [{ id: "kitty:tabs", value: "3 in 1 window" }]);
        check("nothing open or known, nothing said", Kitty.held([], 0, 0, held), []);

        // Each row's words: the name a title gives, else what runs; at the
        // right how it stands, how many panes, what runs where the name
        // doesn't say it, and the folder, never for an ssh session.
        const row = one => [Kitty.nameOf(one, said), Kitty.sideOf(one, said, home)];
        check("a pi session", row(all[0]),
            ["\u03c0 - Faster Refs - world", "active \u00b7 pi \u00b7 ~/world"]);
        check("a shell at its prompt", row(all[1]), ["zsh", "active \u00b7 ~/src/core.nix"]);
        check("Neovim, titled", row(all[2]),
            ["\ud83d\udcdd modules/kitty/home.nix", "nvim \u00b7 ~/src/core.nix"]);
        check("a tab Joel named", row(all[3]),
            ["Deploys", "rang \u00b7 2 panes \u00b7 ssh devbox"]);
        // An ssh session is named by its machine, whatever the far shell
        // titles it.
        check("a pane over ssh", row(all[3].panes[0]), ["ssh devbox", "active \u00b7 rang"]);
        check("its other pane", row(all[3].panes[1]), ["zsh", "~"]);
        check("new output", row(all[4]), ["zsh", "active \u00b7 new output \u00b7 ~"]);
        check("closed", Kitty.closedSide(all[1], said, home), "closed \u00b7 ~/src/core.nix");
        check("closed over ssh", Kitty.closedSide(all[3].panes[0], said, home), "closed");
        check("where a tab is", Kitty.placeOf(all[2], said), "tab 1 of 3");
        check("what finds a tab", Kitty.keywordsOf(all[3], said),
            "Deploys joel@devbox: ~ ssh devbox /tmp zsh /home/joel");

        // A pane's drawer: every figure kitty gives, labelled, and none it
        // hasn't, never its environment.
        const words = {
            said: { yes: "yes", no: "no", size: "{columns} columns, {lines} lines" },
            home: home,
            named: id => `<${id}>`
        };
        const opened = 1794099697431705746 / 1e6;
        check("a pane's drawer", Kitty.drawerOf(all[1].panes[0], words, opened + 7200000), {
            title: "zsh",
            facts: [
                { label: "<folder>", value: "~/src/core.nix" },
                { label: "<running>", value: "-zsh" },
                { label: "<title>", value: "zsh" },
                { label: "<command>", value: "-zsh" },
                { label: "<last>", value: "make test" },
                { label: "<status>", value: "2" },
                { label: "<prompt>", value: "yes" },
                { label: "<full>", value: "no" },
                { label: "<size>", value: "120 columns, 47 lines" },
                { label: "<opened>", value: "2 h ago" },
                { label: "<process>", value: "52" }
            ]
        });
        check("an ssh pane's machine and a program's variables",
            Kitty.drawerOf(all[2].panes[0], words, opened).facts
                .filter(fact => ["<machine>", "<vars>", "<last>", "<size>"].includes(fact.label)),
            [{ label: "<size>", value: "80 columns, 24 lines" },
                { label: "<vars>", value: "neovim_server=/run/user/1000/nvim.51.0" }]);
        check("over ssh", Kitty.drawerOf(all[3].panes[0], words, opened).facts[1],
            { label: "<machine>", value: "devbox" });

        // Going to a tab goes to its active pane; closing a tab closes it
        // whole, and a pane alone; each through the kitty holding it.
        check("go to a tab", Kitty.going("kitten", all[3]),
            ["kitten", "@", "--to", "unix:@kitty-10", "focus-window", "--match", "id:4"]);
        check("close a tab", Kitty.closing("kitten", all[3]),
            ["kitten", "@", "--to", "unix:@kitty-10", "close-tab", "--match", "id:3"]);
        check("close a pane", Kitty.closing("kitten", all[3].panes[1]),
            ["kitten", "@", "--to", "unix:@kitty-10", "close-window", "--match", "id:3"]);

        // A new window opens in the kitty behind the window used last, or in
        // a new kitty while none is open.
        const tools = { kitten: "kitten", kitty: "kitty" };
        check("the kitty used last", Opens.latest(niri), 20);
        check("none open", Opens.latest(niri.slice(3)), 0);
        check("a folder in kitty", Opens.opening(tools, 20, { folder: "/src/a" }),
            ["kitten", "@", "--to", "unix:@kitty-20", "launch", "--type=os-window", "--cwd",
                "/src/a"]);
        check("ssh in kitty", Opens.opening(tools, 20, { command: ["kitten", "ssh", "devbox"] }),
            ["kitten", "@", "--to", "unix:@kitty-20", "launch", "--type=os-window", "kitten",
                "ssh", "devbox"]);
        check("a folder in a new kitty", Opens.opening(tools, 0, { folder: "/src/a" }),
            ["kitty", "--directory", "/src/a"]);
        check("ssh in a new kitty", Opens.opening(tools, 0, { command: ["kitten", "ssh", "x"] }),
            ["kitty", "kitten", "ssh", "x"]);

        // The machines ~/.ssh/config names: each Host line's names but a
        // pattern's, once each, with its own block's settings.
        const config = [
            "# a comment",
            "Host *",
            "  ServerAliveInterval 30",
            "Host devbox build",
            "  HostName 10.0.0.5",
            "  User joel",
            "  Port 2222",
            "",
            "Host=\"quoted\" web-? !banned",
            "  ProxyJump=devbox",
            "Match host *.example",
            "  User elsewhere",
            "Host devbox",
            "  User twice",
            "host lower"
        ].join("\n");
        // And the machines ~/.ssh/known_hosts names: a machine the config
        // names takes the keys held where it reaches, its HostName on its
        // Port; the rest are machines of their own, the other names on a
        // line its own, one name on two ports two; a comment, a hashed
        // line, a marked one, a pattern and a line with no key name none.
        const hosts = [
            "# a comment",
            "10.0.0.5 ssh-ed25519 AAAA",
            "[10.0.0.5]:2222 ssh-ed25519 AAAA",
            "[10.0.0.5]:2222 ssh-rsa AAAA",
            "quoted ecdsa-sha2-nistp256 AAAA",
            "nas,192.168.1.9 ssh-ed25519 AAAA",
            "nas,192.168.1.9 ssh-rsa AAAA",
            "[nas]:2200 ssh-ed25519 AAAA",
            "|1|c2FsdA==|aGFzaA== ssh-ed25519 AAAA",
            "@cert-authority *.example ssh-ed25519 AAAA",
            "*.corp ssh-rsa AAAA",
            "lower ssh-ed25519 AAAA",
            "alone"
        ].join("\n");
        const machines = Opens.machines(config, hosts);
        check("the machines", machines.map(one => [one.name, one.port, one.from]),
            [["devbox", "", "config"], ["build", "", "config"], ["quoted", "", "config"],
                ["lower", "", "config"], ["nas", "", "known"], ["nas", "2200", "known"]]);
        check("their settings",
            machines.map(one => one.options.map(each => `${each.keyword}=${each.value}`)),
            [["HostName=10.0.0.5", "User=joel", "Port=2222"],
                ["HostName=10.0.0.5", "User=joel", "Port=2222"], ["ProxyJump=devbox"], [], [],
                []]);
        check("their keys and other names", machines.map(one => [one.keys, one.also]),
            [[["ssh-ed25519", "ssh-rsa"], []], [["ssh-ed25519", "ssh-rsa"], []],
                [["ecdsa-sha2-nistp256"], []], [["ssh-ed25519"], []],
                [["ssh-ed25519", "ssh-rsa"], ["192.168.1.9"]], [["ssh-ed25519"], []]]);
        check("none named", Opens.machines("", ""), []);
        check("known_hosts alone", Opens.machines("", "[::1]:22 ssh-ed25519 AAAA")
            .map(one => [one.name, one.port]), [["::1", ""]]);

        // ssh is told the port known_hosts gave, and a config's machine by
        // its name alone, its block saying the rest.
        check("ssh to a machine", Opens.sshing("kitten", machines[0]),
            ["kitten", "ssh", "devbox"]);
        check("ssh on a port", Opens.sshing("kitten", machines[5]),
            ["kitten", "ssh", "-p", "2200", "nas"]);

        // How many times each was reached and when last, by zsh's extended
        // history: not sshfs, and not a line without its time.
        const history = [
            ": 1700000000:0;ssh devbox",
            ": 1700000500:3;kitten ssh -p 22 joel@build",
            ": 1700000100:0;ssh devbox uptime",
            "ssh lower",
            ": 1700000900:0;sshfs quoted:/ /mnt",
            ": 1700000800:0;kitty +kitten ssh quoted",
            ": 1700000700:0;ssh 192.168.1.9",
            ": 1700000600:0;ssh nas"
        ].join("\n");
        const reached = Opens.connected(history);
        check("when and how often each was reached", reached, {
            devbox: { times: 2, last: 1700000100000 },
            build: { times: 1, last: 1700000500000 },
            quoted: { times: 1, last: 1700000800000 },
            "192.168.1.9": { times: 1, last: 1700000700000 },
            nas: { times: 1, last: 1700000600000 }
        });

        // Most used first, by every name a machine has, then by when, then
        // as written. The history names no port, so one name on two ports
        // counts for both.
        check("in order", Opens.machinesInOrder(machines, reached)
            .map(one => `${one.name}${one.port}`),
            ["nas", "devbox", "quoted", "nas2200", "build", "lower"]);
        check("as written, knowing nothing", Opens.machinesInOrder(machines, {})
            .map(one => `${one.name}${one.port}`),
            ["devbox", "build", "quoted", "lower", "nas", "nas2200"]);
        const state = { open: ["build"], reached: reached, time: 1700000100000 + 3 * 86400000 };
        const now = {
            now: "connected now", once: "once", times: "{count} times", never: "never",
            config: "~/.ssh/config", known: "~/.ssh/known_hosts",
            eyebrow: "in {file} \u00b7 machine"
        };
        // Its use alone, its address being the drawer's, or that it never
        // was.
        check("a machine reached", Opens.machineSide(machines[0], state, now),
            "3 d ago \u00b7 2 times");
        check("one connected now", Opens.machineSide(machines[1], state, now),
            "connected now \u00b7 once");
        check("one never reached says so, not where it goes",
            Opens.machineSide(machines[0], { open: [], reached: {}, time: state.time }, now),
            "never");
        check("one never reached, with nowhere set",
            Opens.machineSide(machines[3], state, now), "never");
        check("one known by two names", Opens.machineSide(machines[4], state, now),
            "2 d ago \u00b7 2 times");
        check("one on a port", Opens.machineSide(machines[5], state, now),
            "2 d ago \u00b7 once");
        const named = { said: now, named: id => `<${id}>` };
        // The file naming it is its eyebrow's, so no fact says it again;
        // its HostName, User and Port are its one Address, in the Name face.
        check("a configured machine's drawer", Opens.machineDrawer(machines[1], state, named), {
            eyebrow: "in ~/.ssh/config \u00b7 machine", eyebrowNames: true,
            title: "build", facts: [
                { label: "<connected>", value: "connected now" },
                { label: "<times>", value: "once" },
                { label: "<address>", value: "joel@10.0.0.5:2222", identifies: true },
                { label: "<keys>", value: "ssh-ed25519 \u00b7 ssh-rsa" }] });
        check("a machine's drawer", Opens.machineDrawer(machines[2], state, named), {
            eyebrow: "in ~/.ssh/config \u00b7 machine", eyebrowNames: true,
            title: "quoted", facts: [
                { label: "<connected>", value: "2 d ago" },
                { label: "<times>", value: "once" },
                { label: "ProxyJump", value: "devbox" },
                { label: "<keys>", value: "ecdsa-sha2-nistp256" }] });
        check("a known machine's drawer", Opens.machineDrawer(machines[5], state, named), {
            eyebrow: "in ~/.ssh/known_hosts \u00b7 machine", eyebrowNames: true,
            title: "nas", facts: [
                { label: "<connected>", value: "2 d ago" },
                { label: "<times>", value: "once" },
                { label: "<address>", value: "nas:2200", identifies: true },
                { label: "<keys>", value: "ssh-ed25519" }] });
        check("its other names", Opens.machineDrawer(machines[4], state, named).facts
            .filter(fact => fact.label === "<also>"), [{ label: "<also>", value: "192.168.1.9" }]);

        // The repositories under ~/src: those zoxide knows first, a visit
        // inside one counting, then the rest by path.
        const found = ["/src/b/.git/", "/src/a/.git/", "/src/c/.git", "/src/d/.git/",
            "/src/a/.git/"].join("\n");
        const visited = ["/src/c/docs", "/elsewhere", "/src/d", "/src/c"].join("\n");
        check("the repositories", Opens.repositories(found, visited),
            ["/src/c", "/src/d", "/src/a", "/src/b"]);
        check("none found", Opens.repositories("", visited), []);
        check("the repositories by their folders",
            Opens.repositories(["/src/a", "/src/c"].join("\n"), visited), ["/src/c", "/src/a"]);

        // A repository says when it, or a folder in it, was seen last, and
        // the folder holding it only where that isn't ~/src.
        const seen = { "/h/src/a": 1700000000000, "/h/src/a/docs": 1700003600000 };
        const later = 1700003600000 + 2 * 3600000;
        const places = { src: "/h/src", home: "/h" };
        check("a repository in ~/src",
            Opens.repositorySide("/h/src/a", places, seen, { now: later }), "2 h ago");
        const when = { now: later, never: "never opened" };
        check("one deeper, never seen", Opens.repositorySide("/h/src/work/b", places, seen, when),
            "~/src/work \u00b7 never opened");
        check("one outside home", Opens.repositorySide("/srv/c", places, seen, when),
            "/srv \u00b7 never opened");
        check("one in ~/src never seen", Opens.repositorySide("/h/src/e", places, seen, when),
            "never opened");

        // The agents' windows' row says how many, and where they are.
        const agents = { one: "1 agent's window", many: "{count} agents' windows" };
        const tells = { where: "Each is in Agents." };
        check("one agent's window", Kitty.agentsDrawer(1, agents, tells),
            { title: "1 agent's window", body: "Each is in Agents." });
        check("many agents' windows", Kitty.agentsDrawer(85, agents, tells).title,
            "85 agents' windows");

        for (const failure of failures)
            console.warn(failure);
        console.log(`${asked - failures.length} of ${asked} held`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
