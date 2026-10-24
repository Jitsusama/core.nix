"""The desktop test's opens and sets area.

Here the desktop opens and sets things: the clipboard, pictures, documents, sound, the power
profile, displays and Wi-Fi. tests/desktop.nix runs it after desktop_prelude.py, the boot and
the helpers every area shares, so the names it takes from there are imported only for type
checkers.
"""

import json
import time
from datetime import timedelta
from typing import TYPE_CHECKING, cast

if TYPE_CHECKING:
    from desktop_prelude import (
        ACCOUNT,
        ASK_PORTAL,
        LAUNCHER_OPEN,
        PAGE,
        PICTURE,
        SIGNING_IN,
        STARTING,
        ZENITY,
        as_account,
        copied_then_closed,
        drawn_at,
        in_session,
        launcher_ready,
        launcher_until,
        mark,
        opens_in,
        pasted_the_picture,
        photograph,
        reported,
        showing,
        step_in,
        told,
        type_in,
        window_titled,
    )
    from driver_names import machine, retry, subtest, t

with subtest("Super+V finds what was copied earlier, and copies it again"):
    keeping = as_account("systemctl --user is-active clipboard-history")
    t.assertEqual(keeping.strip(), "active")
    # Each copy is waited on until the shell has heard it, so they're
    # kept in the order made.
    with told("heard copy Notes for Tuesday"):
        as_account("echo -n 'Notes for Tuesday' | wl-copy > /dev/null 2>&1")
    with told("heard copy the second copy"):
        as_account("printf 'the second copy\\nover two lines' | wl-copy > /dev/null 2>&1")
    machine.send_key("meta_l-v")
    launcher_ready()
    # The newest is selected, its two lines run together in its row and
    # read whole below the rows.
    launcher_until(
        lambda shown: (shown["selected"] or {}).get("reads") == "the second copy\nover two lines"
    )
    photograph("launcher-copies-all")
    type_in("tues", selecting="Notes for Tuesday")
    photograph("launcher-copies")
    machine.send_key("ret")

    def copied_again(_: bool) -> bool:
        """Say whether the clipboard holds the notes' title again."""
        return as_account("wl-paste --no-newline") == "Notes for Tuesday"

    retry(copied_again, timeout_seconds=10)


# Return pastes into the window Super+V opened over, with the keys it
# pastes by: a terminal's Ctrl+Shift+V, as a terminal takes Ctrl+V as a key
# of its own, and Ctrl+V in a GTK program's field.
with subtest("Super+V pastes into the window it opened over, a terminal or not"):
    as_account(
        "niri msg action spawn -- kitty --title paste-here "
        "sh -c 'stty -icanon -echo; cat > /tmp/pasted'"
    )
    machine.wait_until_succeeds(
        in_session("niri msg focused-window | grep -q paste-here"),
        timeout=timedelta(seconds=STARTING),
    )
    machine.send_key("meta_l-v")
    launcher_ready()
    type_in("tues", selecting="Notes for Tuesday")
    # Paste first, and Copy beside it for only copying.
    step_in()
    launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Paste")
    with told("pastes into kitty"):
        machine.send_key("ret")
    machine.wait_until_succeeds(
        in_session("grep -qx 'Notes for Tuesday' /tmp/pasted"), timeout=timedelta(seconds=10)
    )
    as_account("pkill -f 'cat > /tmp/pasted'")
    machine.wait_until_fails(
        in_session("niri msg windows | grep -q paste-here"), timeout=timedelta(seconds=30)
    )

    # zenity says what its field holds once Return closes it.
    entry = f"{ZENITY}/bin/zenity --entry --title=paste-gtk"
    as_account(f"systemd-run --user --collect sh -c '{entry} > /tmp/entered'")
    machine.wait_until_succeeds(
        in_session("niri msg focused-window | grep -q paste-gtk"),
        timeout=timedelta(seconds=STARTING),
    )
    machine.send_key("meta_l-v")
    launcher_ready()
    type_in("tues", selecting="Notes for Tuesday")
    before_paste = mark()
    machine.send_key("ret")
    reported("pastes into ", lambda program: program != "", before_paste)
    machine.wait_until_succeeds(
        in_session("niri msg focused-window | grep -q paste-gtk"), timeout=timedelta(seconds=10)
    )
    # The shell says so as it starts wtype, which then presses the keys;
    # Return any sooner would close the field still empty.
    time.sleep(1)
    machine.send_key("ret")
    machine.wait_until_succeeds(
        in_session("grep -qx 'Notes for Tuesday' /tmp/entered"), timeout=timedelta(seconds=10)
    )


with subtest("a copy outlives the program that made it, unless it wasn't kept"):
    with told("heard copy gone with its program"):
        copied_then_closed("gone with its program")

    # Empty for a moment between its program going and the helper's
    # offer, which the retry waits out.
    def still_there(_: bool) -> bool:
        """Say whether the clipboard still holds the copy its program made."""
        return as_account("wl-paste --no-newline 2>&1 || true") == "gone with its program"

    retry(still_there, timeout_seconds=10)
    # One the history didn't keep, such as a password manager's, goes
    # with its program, leaving the clipboard empty rather than handing
    # back an older copy. The helper answers the moment it empties, so
    # empty twice over is empty for good.
    copied_then_closed(
        "a password",
        flags="--type x-kde-passwordManagerHint",
        served="wl-paste --list-types 2>/dev/null | grep -qx x-kde-passwordManagerHint",
    )
    machine.wait_until_fails(in_session("wl-paste --list-types > /dev/null 2>&1"), timeout=10)
    machine.fail(in_session("wl-paste --list-types > /dev/null 2>&1"))

with subtest("a copy made as another's program closes is the one kept"):
    # The helper reads one event at a time, so the clipboard emptying
    # can wait behind a newer copy, as it does while a slow program
    # hands over what it copied. Paused, it hears both late.
    with told("heard copy closing as another copies"):
        as_account(
            "systemd-run --user --unit=closing sh -c "
            "\"printf %s 'closing as another copies' | wl-copy --foreground\""
        )
    as_account("systemctl --user kill --signal=SIGSTOP clipboard-history")
    as_account("systemctl --user stop closing")
    as_account("echo -n 'copied meanwhile' | wl-copy > /dev/null 2>&1")
    with told("heard copy copied meanwhile"):
        as_account("systemctl --user kill --signal=SIGCONT clipboard-history")
    # Nothing takes it back a moment later.
    machine.sleep(1)
    t.assertEqual(as_account("wl-paste --no-newline"), "copied meanwhile")


with subtest("a picture copied is kept, shown as it is, and copied again"):
    with told("heard copy Image 256 \u00d7 256"):
        as_account(f"wl-copy --type image/png < {PICTURE} > /dev/null 2>&1")
    with told("heard copy something after the picture"):
        as_account("echo -n 'something after the picture' | wl-copy > /dev/null 2>&1")
    machine.send_key("meta_l-v")
    launcher_ready()
    with told("showed Image 256 \u00d7 256"):
        type_in("image", selecting="Image 256 \u00d7 256")
    photograph("launcher-copies-picture")
    machine.send_key("ret")
    retry(pasted_the_picture, timeout_seconds=10)

with subtest("a picture outlives the program that copied it"):
    # The helper holds the same picture already, so wl-copy serves it
    # once the clipboard's types no longer include the helper's own.
    with told("heard copy Image 256 \u00d7 256"):
        as_account(
            f"wl-copy --foreground --type image/png < {PICTURE} > /dev/null 2>&1 & p=$!; "
            "for i in $(seq 100); do t=$(wl-paste --list-types 2>/dev/null); "
            'echo "$t" | grep -qx image/png && '
            '! echo "$t" | grep -qx application/x-clipboard-history-kept && break; '
            "sleep 0.1; done; kill $p"
        )
    retry(pasted_the_picture, timeout_seconds=10)

with subtest("Forget the Clipboard forgets every copy, and what it held"):
    with told("heard copy kept until forgotten"):
        copied_then_closed("kept until forgotten")

    def still_kept(_: bool) -> bool:
        """Say whether the clipboard still holds the copy its program made."""
        return as_account("wl-paste --no-newline 2>&1 || true") == "kept until forgotten"

    retry(still_kept, timeout_seconds=10)
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    type_in("forget the clip", selecting="Forget the clipboard")
    photograph("launcher-forget-copies")
    with told("forgot the copies"):
        machine.send_key("ret")
    t.assertEqual(as_account("wl-paste --no-newline 2>&1 || true"), "")
    # The pictures' files go with them.
    t.assertEqual(as_account("ls -A $XDG_RUNTIME_DIR/clipboard-history"), "")

# Home is a shelf that keeps its shape: what's in front and the likely
# picks, then the same three collections, in the same order, then the
# categories to browse. The rest are a word away.
with subtest("the launcher's home keeps its shelf, and a collection opens by name"):
    windows = len(json.loads(as_account("niri msg --json windows")))
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    photograph("launcher-home")
    rows = showing()["rows"]
    shelf = rows.index("Windows")
    t.assertEqual(rows[shelf : shelf + 4], ["Windows", "Clipboard", "Notifications", "Computer"])
    # A page down stops on the last of the categories.
    for _page in range(6):
        machine.send_key("pgdn")
    launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Commands")
    # Opening a collection keeps the launcher open, where a program
    # picked instead would close it.
    type_in("shortcuts", selecting="Shortcuts")
    step_in("ret")
    type_in("open a terminal")
    machine.send_key("ret")

    def another_opened(_: bool) -> bool:
        """Say whether a window has opened since the launcher did."""
        return len(json.loads(as_account("niri msg --json windows"))) > windows

    retry(another_opened, timeout_seconds=15)

with subtest("a screenshot just taken is at the launcher's home, and opens in GIMP"):
    pictures = f"/home/{ACCOUNT}/Pictures/Screenshots"
    # Made at login, since niri makes it only with its first screenshot
    # and the launcher watches it from the start.
    machine.succeed(f"test -d {pictures}")
    as_account("niri msg action screenshot-screen")
    machine.wait_until_succeeds(f"ls {pictures}/*.png", timeout=15)
    picture = machine.succeed(f"ls {pictures}/*.png").strip()
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    # Just below the window in front, which home always puts first.
    machine.send_key("down")
    launcher_until(
        lambda shown: (shown["selected"] or {}).get("name") == "Edit the screenshot in GIMP"
    )
    machine.send_key("ret")
    machine.wait_until_succeeds(f"pgrep -u {ACCOUNT} -f '[g]imp.*{picture}'", timeout=STARTING)
    # GIMP goes before it draws, so no window of its own takes the keys
    # meant for what follows, and the picture too, since the region
    # screenshot's subtest waits for one of its own. GIMP takes any other
    # signal as a crash, and writes its backtrace to the log.
    machine.succeed(f"pkill -KILL -u {ACCOUNT} -f '[g]imp'")
    machine.wait_until_fails(f"pgrep -u {ACCOUNT} -f '[g]imp'", timeout=30)
    machine.succeed(f"rm {pictures}/*.png")

# A program asks the portal what to open a picture with, as one does when
# more than one app could, and the launcher asks rather than GNOME's
# dialog, holding the apps the portal offers. Return opens the picture
# with the one taken, and the portal says it opened; asked again, Escape
# opens nothing, and the portal says it was refused; and each ask's
# socket goes once it's answered. Each time the apps offered change
# while it waits, as the portal says when apps are installed, so a
# pick and a refusal still answer the ask as it was first held.
with subtest("the launcher asks what to open a picture with, for the portal"):
    portal_png = f"/home/{ACCOUNT}/portal-picked.png"
    machine.succeed(
        "echo iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk"
        "+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=="
        f" | base64 -d > {portal_png} && chown {ACCOUNT} {portal_png}"
    )
    opened_with = f"pgrep -u {ACCOUNT} -af '[p]ortal-picked.png' | grep -v ask-portal"
    sockets_gone = "! ls /run/user/1000/open-with/*.sock"
    portal_said = "/run/user/1000/portal-said"

    def portal_told() -> str:
        """Give what the shell, the chooser, the portal and the caller made of an ask.

        It's said when the launcher never holds the apps.
        """
        _, said = machine.execute(
            in_session(
                "qs ipc show | grep -A4 open-with; ls -la /run/user/1000/open-with; "
                f"cat {portal_said}; pgrep -af ask-portal; "
                "busctl --user list | grep -i -e portal -e ask; "
                "journalctl --user --no-pager --output=cat --lines=80 "
                "--unit=ask-portal --unit=open-with --unit=xdg-desktop-portal"
            )
        )
        return said

    def ask_portal() -> None:
        """Ask the portal to open the picture, and wait until the launcher holds its apps."""
        machine.succeed(f"rm -f {portal_said}")
        # Under the account's manager, on the bus the portal is on.
        as_account(f"systemd-run --user --collect --unit=ask-portal {ASK_PORTAL} {portal_png}")
        try:
            launcher_until(
                lambda shown: (
                    shown["keys"] and shown["held"] == ["apps"] and shown["selected"] is not None
                ),
                timeout=60,
            )
        except AssertionError as unheard:
            unanswered = f"{unheard}; what was said:\n{portal_told()}"
            raise AssertionError(unanswered) from None

    def offer_only_swayimg() -> None:
        """Offer only swayimg now, one of the apps first offered."""
        reply = machine.succeed("ls /run/user/1000/open-with/*.sock").strip()
        as_account(f"qs ipc call open-with update {reply} swayimg")
        launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Swayimg")

    ask_portal()
    photograph("open-with")
    offer_only_swayimg()
    machine.send_key("ret")
    machine.wait_until_succeeds(f"grep -qx 0 {portal_said}", timeout=60)
    machine.wait_until_succeeds(opened_with, timeout=60)
    launcher_until(lambda shown: not shown["open"])
    machine.wait_until_succeeds(sockets_gone, timeout=10)
    machine.succeed(f"pkill -KILL -u {ACCOUNT} -f '[p]ortal-picked.png'")
    machine.wait_until_fails(f"pgrep -u {ACCOUNT} -f '[p]ortal-picked.png'", timeout=30)

    ask_portal()
    offer_only_swayimg()
    machine.send_key("esc")
    launcher_until(lambda shown: not shown["open"])
    machine.wait_until_succeeds(f"grep -qx 1 {portal_said}", timeout=60)
    machine.wait_until_succeeds(sockets_gone, timeout=10)
    machine.fail(opened_with)
    machine.succeed(f"rm {portal_png}")


with subtest("a picture opens floating in swayimg, and e takes it to GIMP"):
    # Beside the screenshots, never among them, since the region
    # screenshot's subtest waits for a picture there of its own.
    looked = f"/home/{ACCOUNT}/snowflake.png"
    as_account(f"cp {PICTURE} {looked}")
    opens_in(looked, "image/png", "swayimg")
    swayimg = window_titled("Swayimg: snowflake.png")
    assert swayimg["is_floating"], swayimg
    # Four fifths of the screen, centred, at the size niri gave it,
    # less the gaps and border niri takes out of it. Without swayimg's
    # size given again, it's twice that, off the screen.
    left, top, width, height = drawn_at(swayimg)
    wide = json.loads(as_account("niri msg --json focused-output"))["logical"]["width"]
    assert abs(width - wide * 0.8) <= wide * 0.02, swayimg
    assert abs(left + width / 2 - wide / 2) <= wide * 0.02, swayimg
    # The steps before leave notices, which go on their own.
    machine.wait_until_fails(in_session("niri msg layers | grep -q notifications"), timeout=30)
    photograph("swayimg")
    machine.send_key("e")
    machine.wait_until_succeeds(f"pgrep -u {ACCOUNT} -f '[g]imp.*{looked}'", timeout=STARTING)
    machine.wait_until_fails(f"pgrep -u {ACCOUNT} -x swayimg", timeout=10)
    # As in the screenshot's subtest: GIMP goes before it draws.
    machine.succeed(f"pkill -KILL -u {ACCOUNT} -f '[g]imp'")
    machine.wait_until_fails(f"pgrep -u {ACCOUNT} -f '[g]imp'", timeout=30)
    machine.succeed(f"rm {looked}")

with subtest("a document opens in zathura, beside the rest, and q closes it"):
    opens_in(PAGE, "application/pdf", "org.pwmt.zathura")
    windows = json.loads(as_account("niri msg --json windows"))
    zathura = next(window for window in windows if window["app_id"] == "org.pwmt.zathura")
    assert not zathura["is_floating"], zathura
    photograph("zathura")
    machine.send_key("q")
    machine.wait_until_fails(f"pgrep -u {ACCOUNT} -f '[z]athura'", timeout=10)

with subtest("the launcher's Sound moves sound to another output, and back"):
    for output in ["stand-in-headphones", "stand-in-speakers"]:
        as_account("qs ipc call launcher collection sound")
        launcher_ready()
        photograph(f"launcher-sound-{output}")
        kind = output.removeprefix("stand-in-")
        type_in(kind, selecting=kind)
        machine.send_key("ret")
        used = f"wpctl inspect @DEFAULT_AUDIO_SINK@ | grep -q 'node.name = \"{output}\"'"
        machine.wait_until_succeeds(in_session(used), timeout=10)
        machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)

with subtest("the launcher sets the power profile"):
    # By its name, or a word a person would call it.
    for typed, profile, name in [
        ("eco", "power-saver", "Power saver"),
        ("balanced power", "balanced", "Balanced power"),
        # Its old name, a hidden word, still finds it.
        ("use power saver", "power-saver", "Power saver"),
    ]:
        as_account("qs ipc call launcher toggle")
        launcher_ready()
        type_in(typed, selecting=name)
        machine.send_key("ret")
        machine.wait_until_succeeds(f"powerprofilesctl get | grep -qx {profile}", timeout=10)
        machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)

# The right arrow on one of a collection's members offers its own
# actions, as on anything at the top.
with subtest("the launcher's Displays scales a display by keys, and back"):

    def scale() -> float:
        """Give the scale niri draws the display at."""
        outputs = json.loads(as_account("niri msg --json outputs"))
        return cast("float", next(iter(outputs.values()))["logical"]["scale"])

    for wanted in [1.5, scale()]:
        as_account("qs ipc call launcher collection displays")
        launcher_ready()
        step_in()
        # Its settings lead, each saying how it's set, their choices a step
        # in, and found from here by what's typed.
        launcher_until(
            lambda shown: shown["rows"][:4] == ["Scale", "Refresh", "Resolution", "Rotation"]
        )
        photograph("launcher-displays")
        type_in(f"scale {round(wanted * 100)}")
        machine.send_key("ret")
        retry(lambda _, wanted=wanted: scale() == wanted, timeout_seconds=15)
        machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)

    # A part is asked by its words from the top: the refresh rate.
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    type_in("refresh rate")
    launcher_until(
        lambda shown: (
            shown["kind"] == "answer"
            and str((shown["selected"] or {}).get("name", "")).endswith(" Hz")
        )
    )
    machine.send_key("esc")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)

with subtest("a Wi-Fi network is joined from the launcher, a new one asking its password"):
    machine.wait_for_unit("lantern.service")
    machine.wait_for_unit("wifid.service")
    wifi = "varlinkctl call /run/wifid/io.jitsusama.Wifi io.jitsusama.Wifi"
    machine.wait_until_succeeds(
        f"{wifi}.Scan '{{}}' && {wifi}.Describe '{{}}' | grep -q '\"Lantern\"'", timeout=60
    )
    as_account("qs ipc call launcher collection wifi")
    launcher_ready()
    photograph("launcher-wifi")
    type_in("lant", selecting="Lantern")
    # Never joined, it asks for its password, which a wrong one doesn't
    # satisfy; the launcher stays to say so.
    step_in("ret")
    type_in("not the password")
    photograph("launcher-wifi-password")
    with told("refused wifi Lantern", timeout=60):
        machine.send_key("ret")
    machine.succeed('journalctl -u wifid | grep -q "couldn\'t join Lantern: WrongPassword"')
    photograph("launcher-wifi-refused")
    machine.succeed(in_session(LAUNCHER_OPEN))
    type_in("glowing in the dark")
    machine.send_key("ret")
    # Joined means an address too, and wifid says so once it's checked
    # how far the network reaches.
    machine.wait_until_succeeds(
        "journalctl -u wifid | grep -q 'joined Lantern on wlan1'", timeout=60
    )
    machine.succeed("networkctl status wlan1 | grep -q 10.42.0.")
    # Once it's joined, the launcher's done.
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)
    # Lantern wants a sign-in, so Sign in to it comes first at home, and
    # Return opens the window on its front desk.
    described = "varlinkctl --json=short call /run/wifid/io.jitsusama.Wifi"
    described += " io.jitsusama.Wifi.Describe '{}'"
    machine.wait_until_succeeds(f'{described} | grep -qF \'"reach":"signIn"\'', timeout=60)
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    photograph("launcher-sign-in")
    type_in("sign in", selecting="Sign in to Lantern")
    machine.send_key("ret")
    # The window is GTK and WebKit started cold, drawn in software,
    # which on a machine running other tests too has taken 42 seconds
    # to show and more than 60, so it's given three minutes for both.
    window = 'niri msg --json windows | grep -qF \'"app_id":"io.jitsusama.WifiSignIn"\''
    machine.wait_until_succeeds(in_session(window), timeout=SIGNING_IN)
    # Its button can only be pressed once the front desk has served
    # the page, which a window that has to try again does later.
    machine.wait_until_succeeds(
        "journalctl -u lantern-front-desk | grep -q 'GET /sign-in'", timeout=SIGNING_IN
    )
    photograph("sign-in", timeout=20)
    # Its one button signs in, and the window closes as Lantern lets
    # the machine through, its notice taken back with it.
    with told("withdrew sign-in Lantern", timeout=60):
        machine.send_key("ret")
    machine.wait_until_fails(in_session(window), timeout=60)
    machine.succeed(f'{described} | grep -qF \'"reach":"online"\'')
    # The Office asks for an account: met for the first time, it asks
    # the account's name, its server's domain and its password in
    # turn, and joins.
    machine.wait_until_succeeds(
        f"{wifi}.Scan '{{}}' && {wifi}.Describe '{{}}' | grep -q '\"Office\"'", timeout=60
    )
    as_account("qs ipc call launcher collection wifi")
    launcher_ready()
    type_in("office", selecting="Office")
    step_in("ret")
    type_in("joel")
    step_in("ret")
    type_in("radius.office.test")
    step_in("ret")
    type_in("hunter22")
    photograph("launcher-wifi-account")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        "journalctl -u wifid | grep -q 'joined Office on wlan1'", timeout=60
    )
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)
    # Stepped into, a network says after what can be done to it what's
    # heard of it, a row each, the selected row's footer saying more:
    # the card's link to it once the card has read it, and its access
    # points a step further in.
    as_account("qs ipc call launcher collection wifi")
    launcher_ready()
    type_in("office", selecting="Office")
    step_in()
    # The link's row by its signal, which is always read; its rates
    # say in and out only while they differ, which the simulated
    # radio's don't always.
    type_in("dbm", selecting="dBm \u00b7")
    launcher_until(lambda shown: "over the noise" in shown["footer"])
    for _ in "dbm":
        machine.send_key("backspace")
    launcher_until(lambda shown: shown["typed"] == "")
    photograph("launcher-wifi-detail")
    type_in("access", selecting="1 access point")
    step_in()
    launcher_until(lambda shown: shown["footer"].startswith("02:00:00:00:02:00 \u00b7 "))
    assert showing()["selected"]["name"].startswith("2.4 GHz ch 1\u00a0"), showing()
    photograph("launcher-wifi-points")
    machine.send_key("esc")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)
    # Turning Wi-Fi off is an action like any.
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    type_in("wifi off")
    machine.send_key("ret")
    machine.wait_until_succeeds("rfkill list wifi | grep -q 'Soft blocked: yes'", timeout=15)
    machine.succeed("rfkill unblock wifi")

# A binding that throws or assigns the wrong type only warns, and
# only once a surface draws it, which can be long after the start.
with subtest("Quickshell warned of nothing it drew all along"):
    # Quickshell colours its levels and categories even in the journal,
    # so a colour code may stand between the two words.
    log = as_account(
        "journalctl --user -u quickshell --no-pager -o cat | grep -a -E 'WARN[^ ]* scene' || true"
    )
    assert log.strip() == "", f"Quickshell warned of a scene: {log}"
