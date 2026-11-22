"""The system page reads live from the system daemon, through the sites' server, in a browser.

The page is the one Nix builds, served by the real local-sites server beside a
stand-in system daemon (modules/local-sites/tests/stand_in_daemon.py) holding
a summary, four parts and two machines under test names, system.json. It must
say it's live with every machine drawn and counted, follow a machine that
turns and one that goes, say broken when the daemon stops and live again
when it's back, say stale when the server goes, and pass the web-design skill's
page checks at a phone's width and a desk's in both looks. It must open with
what is worth a look, each leading where it can be seen whole, a machine's
long name leaving its look's words room at both widths, a machine's look
saying why it matters with its one act, the launcher's row, at both widths,
and a good part read
from a kept answer never among them, its note said where it belongs. Its keys and
places must move the cursor from section to section, and a machine's address
must put it on that machine. Anything the page
logs fails it, since that is how it drew four empty parts unseen on 16 November.
The server, the browser and the tab are modules/local-sites/tests/live_page.py's.
"""

import json
import os
import re
from pathlib import Path
from typing import TYPE_CHECKING

import pytest
from live_page import LOOKS, SIZES, Budget, Opened, Served, serving
from playwright.sync_api import Browser, Page, expect
from stand_in_daemon import StandIn

if TYPE_CHECKING:
    from collections.abc import Iterator

HERE = Path(__file__).parent
# The system daemon reads its sources every few seconds, so a change must show well inside this.
PACE_MS = 6000
# The feed waits up to 30 seconds between tries once the daemon has been gone a while.
RETRY_MS = 35000
# What system.json comes to: two machines, each worth a look for what it says of itself.
COUNTED = "Two machines, both worth a look."
# What a first visit to the page may cost, drawn from system.json: 25 files,
# about 437 KB and 397 elements since the list of what is worth a look, six to a row,
# and since no comment is sent, its styles' included; about 445 KB since machines
# out of reach are named apart, a cost worth the page reading well with many down;
# 403 elements since each machine's look carries its act, opening it in the launcher;
# about 490 KB since the builders' code, which adds 45 KB and a card when one is named.
BUDGET = Budget(files=25, kilobytes=495, elements=415)


@pytest.fixture
def system_daemon(tmp_path: Path) -> Iterator[StandIn]:
    reading = json.loads((HERE / "system.json").read_text())
    stand_in = StandIn(tmp_path / "system.sock", "system", reading["kinds"]).start()
    yield stand_in
    stand_in.leave()


@pytest.fixture
def system_served(tmp_path: Path, system_daemon: StandIn) -> Iterator[Served]:
    kinds: dict[str, dict[str, object]] = {"summary": {}, "part": {}, "machine": {}, "builder": {}}
    whole = {"socket": str(system_daemon.path), "kinds": kinds}
    with serving(tmp_path, {"system": os.environ["SYSTEM_SITE"]}, {"system": whole}) as served:
        yield served


@pytest.fixture
def system_opened(browser: Browser, system_served: Served) -> Iterator[Opened]:
    root, _ = system_served
    opened = Opened(browser, f"{root}system/", SIZES[-1], "dark")
    yield opened
    opened.close()


@pytest.fixture
def system_page(system_opened: Opened) -> Page:
    return system_opened.page


def says_it_stands(page: Page, state: str, timeout: int = PACE_MS) -> None:
    expect(page.locator("#standing"), f"the page never said it was {state}").to_have_attribute(
        "data-state", state, timeout=timeout
    )


def counted(page: Page, said: str = COUNTED) -> None:
    expect(page, f"the page's title never counted {said}").to_have_title(
        f"{said} Your machines, now", timeout=PACE_MS
    )


def heard(page: Page, address: str) -> None:
    """Follow `address` on the open page, once the page has heard it."""
    # The page's own listener was added first, so it has run when this one does.
    page.evaluate(
        """(address) => new Promise((done) => {
            addEventListener("hashchange", () => done(), { once: true });
            location.hash = address;
        })""",
        address,
    )


def machine_called(name: str) -> dict[str, object]:
    """Make a fine machine's record, as the daemon would send one for `name`."""
    return {
        "key": f"machine:{name}",
        "kind": "machine",
        "part": "machines",
        "name": name,
        "health": "good",
        "says": "is fine",
        "side": [],
    }


def test_the_system_page_draws_every_machine_live(system_page: Page) -> None:
    says_it_stands(system_page, "live")
    counted(system_page)
    expect(system_page.locator("#standing-words"), "the page didn't say where").to_contain_text(
        "kestrel"
    )
    for machine in ["kestrel", "osprey"]:
        row = system_page.locator(f'#parts a[data-row="machine:{machine}"]')
        expect(row, f"the page drew no row for {machine}").to_be_visible()


# How far under the masthead's foot a row scrolled up to it may land, its
# own margin and a rounding: further, and it never scrolled to the edge.
SCROLLED_TO = 40
# How far over its foot a row may land, from the browser snapping a scroll
# to whole pixels; a row under the masthead would be over by its height.
SNAPPED = 1


def section_of_the_cursor(page: Page) -> str | None:
    """Say which section holds what has focus, by its id, or None."""
    held = page.evaluate("document.activeElement?.closest('section[id]')?.id ?? null")
    return held if isinstance(held, str) else None


def test_the_keys_and_places_move_the_cursor(system_page: Page) -> None:
    says_it_stands(system_page, "live")
    counted(system_page)
    keyboard = system_page.keyboard
    keyboard.press("j")
    assert section_of_the_cursor(system_page) == "looking", "j didn't start at the looks"
    keyboard.press("1")
    assert section_of_the_cursor(system_page) == "machine-1"
    keyboard.press("2")
    assert section_of_the_cursor(system_page) == "machine-2"
    # With machine 1's row scrolled up out of sight, 1 scrolls back to it and
    # lands it at the top edge: clear of the masthead only while the scroll
    # padding holds it there.
    size = system_page.viewport_size
    system_page.set_viewport_size({"width": 1280, "height": 400})
    above = system_page.evaluate(
        "(() => { const row = document.querySelector('#machine-1 a[data-row]');"
        " window.scrollTo(0, row.getBoundingClientRect().bottom + window.scrollY + 50);"
        " return row.getBoundingClientRect().bottom < 0; })()"
    )
    assert above is True, "machine 1's row couldn't be scrolled out of sight"
    keyboard.press("1")
    assert section_of_the_cursor(system_page) == "machine-1"
    gap = system_page.evaluate(
        "document.activeElement.getBoundingClientRect().top"
        " - document.querySelector('.mast').getBoundingClientRect().bottom"
    )
    assert isinstance(gap, int | float), f"the gap measured {gap!r}, not a number"
    assert -SNAPPED < gap < SCROLLED_TO, f"the row landed {gap}px below the masthead's foot"
    if size is not None:
        system_page.set_viewport_size(size)
    keyboard.press("2")
    keyboard.press("k")
    assert section_of_the_cursor(system_page) == "machine-1"
    place = system_page.locator('#places a[data-place="machine-2"]')
    place.click()
    assert section_of_the_cursor(system_page) == "machine-2"
    expect(place, "the place the cursor is in wasn't marked").to_have_attribute(
        "aria-current", "location"
    )
    keyboard.press("y")
    # A headless browser may refuse the clipboard; either way y says what it
    # did, and with the command of the machine the cursor is on.
    expect(system_page.locator("#note"), "y didn't copy osprey's command").to_have_text(
        re.compile(r"^(Copied|Couldn't copy).*mycelium sprout --on osprey\b")
    )
    # Off every machine, y copies nothing and says so.
    system_page.evaluate("document.activeElement?.blur()")
    keyboard.press("y")
    expect(system_page.locator("#note"), "y off a machine didn't say why").to_have_text(
        re.compile(r"^Nothing to copy here")
    )


# What system.json's sides come to, worst first: whose each is, then what.
LOOKED_AT = [
    ("kestrel", "Booted 3 hours ago after a firmware error"),
    ("osprey", "Code wants a look"),
    ("Mycelium", "4 sessions say they run with their processes gone"),
    ("Network", "The attic's radio is busy 53% of the time"),
    ("Network", "And 3 more worth a look"),
]
# The rows' keys a steps through: whose each is, then how many the same came
# before it, then its own words.
KESTREL_LOOK = "look:machine:kestrel:0:booted 3 hours ago after a firmware error"
OSPREY_LOOK = "look:machine:osprey:0:code wants a look"
MYCELIUM_LOOK = "look:part:mycelium:0:4 sessions say they run with their processes gone"


def focused_row(page: Page) -> str | None:
    """Say which row has focus, by its key, or None."""
    held = page.evaluate("document.activeElement?.dataset?.row ?? null")
    return held if isinstance(held, str) else None


def test_what_is_worth_a_look_leads_the_page(system_page: Page, system_daemon: StandIn) -> None:
    says_it_stands(system_page, "live")
    counted(system_page)
    whose = system_page.locator("#looks .whose").all_inner_texts()
    words = system_page.locator("#looks .words").all_inner_texts()
    assert list(zip(whose, words, strict=True)) == LOOKED_AT, "the looks weren't listed in turn"
    above = system_page.evaluate(
        "document.querySelector('#looking').getBoundingClientRect().bottom"
        " <= document.querySelector('#bridge').getBoundingClientRect().top"
    )
    assert above is True, "the looks weren't over the machines"
    network = system_page.locator("#looks a[data-row^='look:part:network:']").first
    expect(network, "a part's look didn't lead to its site").to_have_attribute(
        "href", "../network/"
    )
    keyboard = system_page.keyboard
    keyboard.press("a")
    assert focused_row(system_page) == KESTREL_LOOK, "a didn't go to the first look"
    keyboard.press("a")
    assert focused_row(system_page) == OSPREY_LOOK, "a didn't go to the next"
    keyboard.press("Enter")
    assert section_of_the_cursor(system_page) == "machine-2", "osprey's look didn't go to it"
    assert system_page.evaluate("location.hash") == "", "a look's way there was left in the URL"
    # Off the list, a starts again at the first; a press for each look then
    # passes the last and comes round to it.
    keyboard.press("a")
    assert focused_row(system_page) == KESTREL_LOOK, "a off the list didn't start over"
    for _ in LOOKED_AT:
        keyboard.press("a")
    assert focused_row(system_page) == KESTREL_LOOK, "a didn't go round to the first"
    # A look clearing before the cursor's leaves it on its own look, not on
    # whichever now stands where it stood.
    keyboard.press("a")
    system_daemon.tell("machine", changed=[{"key": "machine:kestrel", "side": []}])
    expect(
        system_page.locator("#looks a[data-row^='look:machine:kestrel:']"), "kestrel's look stayed"
    ).to_have_count(0, timeout=PACE_MS)
    # The machines part still names it until its own record changes.
    expect(
        system_page.locator("#looks a", has_text="booted 3 hours ago"),
        "kestrel's look stayed as the machines part's",
    ).to_have_count(0)
    assert focused_row(system_page) == OSPREY_LOOK, "the cursor left a look that still stands"
    # One that clears under the cursor leaves it on the one now in its place.
    system_daemon.tell("machine", changed=[{"key": "machine:osprey", "side": []}])
    system_page.wait_for_function(
        "row => document.activeElement?.dataset?.row === row", arg=MYCELIUM_LOOK, timeout=PACE_MS
    )
    quieted(system_page, system_daemon)
    # The cursor was on a look; with none left it rests on the list, not a row below.
    held = system_page.evaluate("document.activeElement?.id ?? null")
    assert held == "looks", "focus left the emptied list for another region"


def quieted(page: Page, daemon: StandIn) -> None:
    """Clear every side, and see the list say nothing is worth a look, in one line."""
    quiet: list[dict[str, object]] = [
        {"key": key, "side": []} for key in ["machine:kestrel", "machine:osprey"]
    ]
    daemon.tell("machine", changed=quiet)
    parts = ["machines", "mycelium", "network"]
    daemon.tell("part", changed=[{"key": f"part:{part}", "side": []} for part in parts])
    expect(page.locator("#looks"), "an empty list didn't say so").to_have_text(
        "Nothing is worth a look.", timeout=PACE_MS
    )


def test_a_says_what_the_list_cannot_know(system_page: Page, system_daemon: StandIn) -> None:
    says_it_stands(system_page, "live")
    quieted(system_page, system_daemon)
    keyboard = system_page.keyboard
    keyboard.press("a")
    expect(system_page.locator("#note"), "a with no look didn't say why").to_have_text(
        "Nothing is worth a look."
    )
    # A machine named before its record may hold a look, so the list says that
    # isn't known yet, a says so, and that goes once the list is known again.
    named = ["machine:kestrel", "machine:osprey"]
    coming: list[dict[str, object]] = [{"key": "part:machines", "things": [*named, "machine:wren"]}]
    system_daemon.tell("part", changed=coming)
    expect(system_page.locator("#looks"), "a machine to come was taken as nothing").to_have_text(
        "What is worth a look isn't known yet.", timeout=PACE_MS
    )
    expect(system_page.locator("#note"), "a's note outlived what the list said").not_to_have_text(
        "Nothing is worth a look."
    )
    keyboard.press("a")
    expect(system_page.locator("#note"), "a claimed more than the list knew").to_have_text(
        "What is worth a look isn't known yet."
    )
    system_daemon.tell("part", changed=[{"key": "part:machines", "things": named}])
    expect(system_page.locator("#looks"), "the list didn't settle again").to_have_text(
        "Nothing is worth a look.", timeout=PACE_MS
    )
    expect(system_page.locator("#note"), "a stale note outlived the list").not_to_have_text(
        "What is worth a look isn't known yet."
    )
    # A record that can't be read leaves the list not known, and a's answer
    # keeps the note's word on which record it was.
    system_daemon.tell("machine", changed=[{"key": "machine:wren", "kind": "machine"}])
    system_daemon.tell("part", changed=coming)
    unread = system_page.locator("#note")
    expect(unread, "a record not read went unsaid").to_contain_text(
        "Couldn't read: machine:wren", timeout=PACE_MS
    )
    expect(system_page.locator("#looks"), "a record not read was taken as nothing").to_have_text(
        "What is worth a look isn't known yet."
    )
    keyboard.press("a")
    expect(unread, "a didn't answer").to_contain_text("What is worth a look isn't known yet.")
    expect(unread, "a hid which record couldn't be read").to_contain_text(
        "Couldn't read: machine:wren"
    )


def test_a_kept_answer_is_no_look(system_page: Page, system_daemon: StandIn) -> None:
    says_it_stands(system_page, "live")
    quieted(system_page, system_daemon)
    # The daemon stands a part on its owner's last answer only while that is
    # fresh enough to, so the note it carries says how the page was read: the
    # machines' in the page's note, a part's in its own section.
    machines = "as read at 00:19; just now mycelium machines gave no answer in 20 s"
    code = "as read at 00:19; just now repos --json gave no answer in 20 s"
    system_daemon.tell(
        "part",
        changed=[
            {"key": "part:machines", "side": [machines]},
            {"key": "part:code", "side": [code]},
        ],
    )
    expect(system_page.locator("#note"), "the machines' kept answer went unsaid").to_contain_text(
        f"Machines {machines}.", timeout=PACE_MS
    )
    aside = system_page.locator("[data-part='part-code'] .aside")
    expect(aside, "the code part's kept answer went unsaid").to_have_text(f"A{code[1:]}")
    expect(aside, "the code part's kept answer was hidden").to_be_visible()
    expect(system_page.locator("#looks"), "a kept answer was listed as a look").to_have_text(
        "Nothing is worth a look."
    )
    # An answer the keys ask for is said before the note, never in its place.
    system_page.locator('#places a[data-place="machine-2"]').click()
    system_page.keyboard.press("y")
    expect(system_page.locator("#note"), "y's answer hid the kept note").to_have_text(
        re.compile(rf"^(Copied|Couldn't copy).* Machines {re.escape(machines)}\.$")
    )


# A thumb's target, as WCAG 2.5.5 sizes one, in CSS pixels.
THUMB = 44


# A why told afresh, so the list redraws under a focused act.
ANEW = "What ran there may have ended without warning, as it did again."
# The look's words told afresh, which changes the key its row is found by.
REWORDED = "booted 2 hours ago after a firmware error"
# A look of kestrel's listed before the one with the act, so a step back
# from the act that forgot it stands after its own look would land here,
# rather than on the first row, where both would land.
LEAD = {"side": "runs hot under load", "matters": "Its builds may slow while it cools."}
# The key of the row after kestrel's firmware look among every row the keys
# step through, or its own where it is the last.
AFTER_KESTREL = """() => {
  const rows = [...document.querySelectorAll('#looks a[data-row], #parts a[data-row]')];
  const at = rows.findIndex((row) => row.matches('a.look') && row.textContent.includes('firmware'));
  return rows[Math.min(at + 1, rows.length - 1)]?.dataset.row ?? null;
}"""


@pytest.mark.parametrize("size", SIZES, ids=[f"{width}px" for width, _ in SIZES])
def test_a_machines_look_says_why_it_matters_and_opens_it_in_the_launcher(
    browser: Browser, system_served: Served, system_daemon: StandIn, size: tuple[int, int]
) -> None:
    root, _ = system_served
    opened = Opened(browser, f"{root}system/", size, "dark")
    page = opened.page
    try:
        says_it_stands(page, "live")
        firmware = "booted 3 hours ago after a firmware error"
        matters = "What ran there may have ended without warning."
        system_daemon.tell(
            "machine",
            changed=[
                {
                    "key": "machine:kestrel",
                    "looks": [LEAD, {"side": firmware, "matters": matters}],
                }
            ],
        )
        looks = page.locator("#looks a.look")
        row = page.locator("#looks li").filter(has=page.locator("a.look", has_text="firmware"))
        why = row.locator(".why")
        expect(why, "the look didn't say why it matters").to_contain_text(matters, timeout=PACE_MS)
        expect(why, "the look's why was hidden").to_be_visible()
        launch = why.get_by_role("link", name="Open kestrel in the Launcher")
        expect(launch, "the look's act didn't open its launcher row").to_have_attribute(
            "href", "mycelium:machine/kestrel?from=system"
        )
        box = launch.bounding_box()
        assert box is not None, "the look's act wasn't drawn"
        assert box["height"] >= THUMB, f"the act is {box['height']} px tall, under a thumb's"
        words = row.locator(".words").bounding_box()
        drawn = why.bounding_box()
        assert words is not None, "the look's words weren't drawn"
        assert drawn is not None, "the look's why wasn't drawn"
        # Under the words, in their column, so the reason reads as this look's.
        assert abs(drawn["x"] - words["x"]) < 1, "the why stood apart from its look's words"
        assert drawn["y"] >= words["y"] + words["height"] - 1, "the why ran beside its words"
        # As drawn, not as named: a flex box drops the spaces between runs of
        # text, which the name read aloud keeps.
        assert launch.inner_text() == "Open kestrel in the Launcher", launch.inner_text()
        opened.passes_the_skills_checks()
        # The act is the look's, so a redraw keeps focus on it, and the keys
        # step on from it as from its look.
        launch.focus()
        system_daemon.tell(
            "machine",
            changed=[
                {"key": "machine:kestrel", "looks": [LEAD, {"side": firmware, "matters": ANEW}]}
            ],
        )
        expect(why, "the look's why wasn't redrawn").to_contain_text(ANEW, timeout=PACE_MS)
        expect(launch, "a redraw took focus off the look's act").to_be_focused()
        # Its words changed too, which changes the row's key: the act under the
        # row in its place keeps focus, so Return still opens the launcher.
        system_daemon.tell(
            "machine",
            changed=[
                {"key": "machine:kestrel", "looks": [LEAD, {"side": REWORDED, "matters": ANEW}]}
            ],
        )
        expect(row, "the look wasn't reworded").to_contain_text(
            REWORDED, ignore_case=True, timeout=PACE_MS
        )
        expect(launch, "rewording the look took focus off its act").to_be_focused()
        # A step back from the act lands on its own look, which it stands after,
        # and not on the look before it.
        expect(looks.first, "the lead look wasn't listed first").to_contain_text(
            LEAD["side"], ignore_case=True
        )
        page.keyboard.press("k")
        expect(row.locator("a.look"), "k from the act left its look").to_be_focused()
        launch.focus()
        after = page.evaluate(AFTER_KESTREL)
        page.keyboard.press("j")
        held = page.evaluate("document.activeElement?.dataset?.row ?? null")
        assert held == after, f"j from the act went to {held}, not {after}"
    finally:
        opened.close()


def test_a_machines_address_puts_the_cursor_on_it(browser: Browser, system_served: Served) -> None:
    root, _ = system_served
    opened = Opened(browser, f"{root}system/#machine=osprey", SIZES[-1], "dark")
    page = opened.page
    try:
        says_it_stands(page, "live")
        counted(page)
        expect(
            page.locator("#parts a[data-row='machine:osprey']"),
            "osprey's address didn't land on it",
        ).to_be_focused()
        # Answered, the address is cleared, so following it again is heard.
        expect(page, "the address answered wasn't cleared").to_have_url(f"{root}system/")
        page.evaluate("location.hash = '#machine=heron'")
        expect(page.locator("#note"), "an address for no machine here didn't say so").to_have_text(
            "No machine here is called heron."
        )
        expect(
            page.locator("#parts a[data-row='machine:osprey']"),
            "an address for no machine here moved the cursor",
        ).to_be_focused()
        page.evaluate("location.hash = '#machine=kestrel'")
        expect(
            page.locator("#parts a[data-row='machine:kestrel']"),
            "a new address on the open page didn't land on its machine",
        ).to_be_focused()
        expect(
            page.locator('#places a[data-place="machine-1"]'),
            "the place of the machine landed on wasn't marked",
        ).to_have_attribute("aria-current", "location")
        expect(
            page.locator("#note"), "a landing with nothing to say left the last note"
        ).to_have_text("")
        page.keyboard.press("2")
        expect(
            page.locator("#parts a[data-row='machine:osprey']"), "2 didn't move off kestrel"
        ).to_be_focused()
        page.evaluate("location.hash = '#machine=kestrel'")
        expect(
            page.locator("#parts a[data-row='machine:kestrel']"),
            "following the same address again didn't land",
        ).to_be_focused()
    finally:
        opened.close()


def test_an_address_for_a_machine_still_coming_lands_when_it_comes(
    system_page: Page, system_daemon: StandIn
) -> None:
    says_it_stands(system_page, "live")
    counted(system_page)
    # The part names wren a push before wren's own record comes, as a new
    # machine's can be, and puts osprey first, which says on the page that
    # the part has come.
    named: dict[str, object] = {
        "key": "part:machines",
        "things": ["machine:osprey", "machine:kestrel", "machine:wren"],
    }
    system_daemon.tell("part", changed=[named])
    first = system_page.locator("#parts a[data-row^='machine:']").first
    expect(first, "the part naming wren never came").to_have_attribute(
        "data-row", "machine:osprey", timeout=PACE_MS
    )
    system_page.keyboard.press("1")
    expect(first, "1 didn't put the cursor on the first machine").to_be_focused()
    heard(system_page, "#machine=wren")
    assert system_page.url.endswith("#machine=wren"), "a machine still coming wasn't waited for"
    expect(
        system_page.locator("#note"), "a machine still coming was said not to be here"
    ).to_have_text("")
    expect(first, "a machine still coming moved the cursor").to_be_focused()
    # The redraw wren's record brings puts the cursor back where it was
    # first, which mustn't count as Joel's own move.
    system_daemon.tell("machine", changed=[machine_called("wren")])
    expect(
        system_page.locator("#parts a[data-row='machine:wren']"),
        "the machine the address waited for didn't land when it came",
    ).to_be_focused(timeout=PACE_MS)


def test_a_wait_ends_on_joels_move_or_its_patience(
    system_page: Page, system_daemon: StandIn
) -> None:
    says_it_stands(system_page, "live")
    counted(system_page)
    first = system_page.locator("#parts a[data-row^='machine:']").first
    named: dict[str, object] = {
        "key": "part:machines",
        "things": ["machine:osprey", "machine:kestrel", "machine:wren"],
    }
    system_daemon.tell("part", changed=[named])
    expect(first, "the part naming wren never came").to_have_attribute(
        "data-row", "machine:osprey", timeout=PACE_MS
    )
    heard(system_page, "#machine=wren")
    system_page.keyboard.press("Shift")
    assert system_page.url.endswith("#machine=wren"), "a modifier held alone ended the wait"
    system_page.keyboard.press("j")
    expect(system_page, "Joel's move left the address waiting").to_have_url(
        re.compile(r"/system/$")
    )
    system_daemon.tell("machine", changed=[machine_called("wren")])
    wren = system_page.locator("#parts a[data-row='machine:wren']")
    expect(wren, "wren's record never came").to_be_visible(timeout=PACE_MS)
    expect(wren, "a wait Joel ended still landed").not_to_be_focused()
    # finch is named and never sent, so the page waits its patience, says
    # so, and still lands when finch comes, Joel not having moved.
    named["things"] = ["machine:kestrel", "machine:osprey", "machine:wren", "machine:finch"]
    system_daemon.tell("part", changed=[named])
    expect(first, "the part naming finch never came").to_have_attribute(
        "data-row", "machine:kestrel", timeout=PACE_MS
    )
    heard(system_page, "#machine=finch")
    expect(system_page.locator("#note"), "a wait past its patience said nothing").to_have_text(
        "No machine read here yet is called finch; the machines may not all be read.",
        timeout=2 * PACE_MS,
    )
    system_daemon.tell("machine", changed=[machine_called("finch")])
    expect(
        system_page.locator("#parts a[data-row='machine:finch']"),
        "a machine that came past the patience didn't land",
    ).to_be_focused(timeout=PACE_MS)
    expect(system_page.locator("#note"), "the late note outlived its machine").to_have_text("")


def test_a_machine_that_turns_is_counted(system_page: Page, system_daemon: StandIn) -> None:
    says_it_stands(system_page, "live")
    counted(system_page)
    turned: dict[str, object] = {
        "key": "machine:osprey",
        "health": "bad",
        "side": ["out of reach"],
    }
    system_daemon.tell("machine", changed=[turned])
    counted(system_page, "Two machines, one in trouble and one worth a look.")


def test_a_machine_out_of_reach_is_named_apart(system_page: Page, system_daemon: StandIn) -> None:
    """One the mycelium can't reach stands no pier and no section, and its one look goes to it."""
    says_it_stands(system_page, "live")
    counted(system_page)
    gone: dict[str, object] = {
        **machine_called("wren"),
        "health": "bad",
        "says": "is out of reach",
        "side": ["out of reach"],
        "ties": {
            "reached": False,
            "heard_at": "2026-11-16T11:00:00Z",
            "cells": {"busy": 1, "idle": 0, "ended": 0},
            "network": None,
            "code": None,
            "figures": None,
            "acts": None,
        },
    }
    system_daemon.tell("machine", changed=[gone])
    # The part names wren too, as the daemon's would once wren is a machine of it.
    things = ["machine:kestrel", "machine:osprey", "machine:wren"]
    named: dict[str, object] = {"key": "part:machines", "things": things}
    system_daemon.tell("part", changed=[named])
    counted(system_page, "Three machines, two worth a look and one out of reach.")
    row = system_page.locator("#away a[data-row='machine:wren']")
    expect(row, "wren wasn't named out of reach").to_be_visible(timeout=PACE_MS)
    expect(system_page.locator("#away .heard"), "its cells went unsaid").to_contain_text(
        "1 cell still on it"
    )
    expect(system_page.locator("section.machine"), "wren drew a section").to_have_count(2)
    expect(system_page.locator("#bridge"), "wren stood a pier").not_to_contain_text("wren")
    expect(system_page.locator('#places a[data-place="away"]'), "no place").to_have_count(1)
    system_page.keyboard.press("a")
    assert focused_row(system_page) == "look:away", "the look for those out of reach didn't lead"
    system_page.keyboard.press("Enter")
    assert section_of_the_cursor(system_page) == "away", "its look didn't go to them"
    system_page.set_viewport_size({"width": 390, "height": 844})
    spills = system_page.evaluate(
        "(() => { const away = document.querySelector('#away');"
        " return away.scrollWidth > away.clientWidth; })()"
    )
    assert spills is False, "the list out of reach spilled past a phone's width"


# A spot machine's name, as long as the provider's: sized to it, the names'
# column once took a phone's whole width and left the words a letter wide.
LONG_NAME = "gull-spot-provider-s99ia2-1795535220"
# The least of the list's width each look's words keep, whatever the names:
# the names' column takes two fifths at most, and the mark and gaps a little.
WORDS_KEEP = 0.4
# Where each look's words run to, against the list's width and the window's.
WORDS_ROOM = """(() => {
  const list = document.querySelector('#looks').getBoundingClientRect();
  const words = [...document.querySelectorAll('#looks .words')]
    .map((w) => w.getBoundingClientRect());
  return { least: Math.min(...words.map((w) => w.width)) / list.width,
    past: Math.max(...words.map((w) => w.right)) - innerWidth };
})()"""


def test_a_long_name_leaves_its_words_room_at_every_size(
    system_page: Page, system_daemon: StandIn
) -> None:
    says_it_stands(system_page, "live")
    counted(system_page)
    long: dict[str, object] = {
        **machine_called(LONG_NAME),
        "health": "bad",
        "says": "has a nearly full disk, 54 GB free",
        "side": ["nearly full disk, 54 GB free"],
    }
    system_daemon.tell("machine", changed=[long])
    things = ["machine:kestrel", "machine:osprey", f"machine:{LONG_NAME}"]
    system_daemon.tell("part", changed=[{"key": "part:machines", "things": things}])
    look = system_page.locator(f"#looks a[data-row^='look:machine:{LONG_NAME}:']")
    expect(look, "the long-named machine's look never came").to_be_visible(timeout=PACE_MS)
    for width, height in SIZES:
        system_page.set_viewport_size({"width": width, "height": height})
        room = system_page.evaluate(WORDS_ROOM)
        assert room["past"] <= 0, f"at {width} px a look's words ran {room['past']} px past"
        assert room["least"] >= WORDS_KEEP, f"at {width} px a look's words kept {room['least']}"


def test_a_builder_comes_on_its_own_watch(system_page: Page, system_daemon: StandIn) -> None:
    says_it_stands(system_page, "live")
    card = system_page.locator('#parts [data-part="part-builders"]')
    expect(card, "builders were drawn while the node named none").to_have_count(0)
    gull: dict[str, object] = {
        "key": "builder:gull",
        "kind": "builder",
        "name": "gull",
        "health": "watch",
        "says": "isn't answering (connection refused)",
        "side": ["not answering"],
        "ties": None,
    }
    system_daemon.tell("builder", changed=[gull])
    named = ["machine:kestrel", "machine:osprey", "builder:gull"]
    system_daemon.tell("part", changed=[{"key": "part:machines", "things": named}])
    expect(card, "gull's record never reached the page").to_contain_text(
        "gull (watch) isn't answering", timeout=PACE_MS
    )
    look = system_page.locator("#looks a[data-row^='look:builder:gull:']")
    expect(look, "gull's look wasn't listed").to_have_attribute("href", "../mycelium/#builder=gull")
    counted(system_page)


def test_a_machine_that_goes_leaves_its_part(system_page: Page, system_daemon: StandIn) -> None:
    says_it_stands(system_page, "live")
    row = system_page.locator('#parts a[data-row="machine:osprey"]')
    expect(row, "the page drew no row for osprey").to_be_visible()
    system_daemon.tell("machine", gone=["machine:osprey"])
    expect(row, "the gone machine was still drawn").to_have_count(0, timeout=PACE_MS)


def test_a_system_daemon_that_stops_is_said_broken_then_live(
    system_page: Page, system_daemon: StandIn
) -> None:
    says_it_stands(system_page, "live")
    system_daemon.leave()
    says_it_stands(system_page, "broken")
    system_daemon.start()
    says_it_stands(system_page, "live", RETRY_MS)
    counted(system_page)


def test_a_sites_server_that_goes_is_said_stale(
    system_opened: Opened, system_served: Served
) -> None:
    says_it_stands(system_opened.page, "live")
    _, server = system_served
    system_opened.loses(server)
    says_it_stands(system_opened.page, "stale")


def test_the_system_page_keeps_to_its_budget(browser: Browser, system_served: Served) -> None:
    root, _ = system_served
    opened = Opened(browser, f"{root}system/", SIZES[-1], "dark")
    try:
        says_it_stands(opened.page, "live")
        counted(opened.page)
        opened.keeps_to(BUDGET)
        says_it_stands(opened.page, "live")
    finally:
        opened.close()


@pytest.mark.parametrize("look", LOOKS)
@pytest.mark.parametrize("size", SIZES)
def test_the_system_page_passes_the_skills_page_checks(
    browser: Browser, system_served: Served, size: tuple[int, int], look: str
) -> None:
    root, _ = system_served
    opened = Opened(browser, f"{root}system/", size, look)
    says_it_stands(opened.page, "live")
    counted(opened.page)
    opened.passes_the_skills_checks()
    opened.close()
