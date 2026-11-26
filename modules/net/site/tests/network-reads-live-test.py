"""The network page reads live from net and the node, through the sites' server, in a browser.

The page is the one Nix builds, served by the real local-sites server beside a
stand-in net daemon and a stand-in node (modules/local-sites/tests/
stand_in_daemon.py), holding a made-up house as net tells it (net.json) and the
one machine mycelium runs on there with two cells (node.json), each feed
trimmed to the kinds home.nix declares (NET_KINDS, NODE_KINDS). It must say it's
live with every device drawn in its tree and on its map, a name anyone on the
Wi-Fi chose drawn as words, drop a device that goes with its link,
say in a machine's panel what runs there as the node moves and link it to its
place on the mycelium map, say broken when net stops and live again, its
devices read again, when it's back, say in its lede when net has never
answered, to a page opened later too, and count what stands once it does,
say stale when the server goes and keep what it drew, draw every
device and wire within a phone's width with its zoom buttons a finger's reach,
stack a bigger house with switches and a full floor
within a phone's width and a laptop's, draw the other way as a window narrows
to a phone's and widens back, its lede still counting, stack in a window just too narrow for the map
across once its floors' names are counted, keep to its budget,
and pass the web-design skill's page checks at a phone's width and a desk's in
both looks.
Anything the page logs fails it, but for the browser's own lines once the test
takes the server away. The server, the browser and the tab are
modules/local-sites/tests/live_page.py's.
"""

import json
import os
import re
from pathlib import Path
from typing import TYPE_CHECKING

import pytest
from live_page import LOOKS, SIZES, Budget, Opened, Served, serving
from playwright.sync_api import Browser, Locator, Page, expect
from stand_in_daemon import StandIn, standing_in

if TYPE_CHECKING:
    from collections.abc import Iterator

HERE = Path(__file__).parent
# The page draws on the frame after a record moves, so a change must show well inside this.
PACE_MS = 6000
# The server watches a gone daemon again every 2 seconds, so its return shows inside this.
BACK_MS = 15000
# A page that dropped what it drew on a change shows it within a redraw, and one that lost its
# server tries again a second on (feed.ts); waiting this long lets both happen first.
REDRAWN_MS = 2500
# Every device net.json holds, by the id its tree item and its place on the map carry.
DEVICES = [each["id"] for each in json.loads((HERE / "net.json").read_text())["kinds"]["device"]]
PRINTER = "mac:00:00:5e:00:53:10"
# The printer's one link, which goes when it does, as net keys it.
PRINTER_LINK = f"link:eero:2|{PRINTER}"
# A device anyone on the Wi-Fi may name, with markup in the name it chose.
TV = "mac:00:00:5e:00:53:30"
TV_NAME = "<b>Den</b> TV"
# The laptop mycelium runs on, by the id net gives it and the name both give it.
KESTREL = "mac:00:00:5e:00:53:20"
# What a first visit to the network page may cost, drawn from the stand-in's house. The kit's
# head.css, the head every live page shares, is one of the files and a few of the kilobytes,
# within the room the budget held before it.
BUDGET = Budget(files=36, kilobytes=505, elements=335)
# The lede once net has answered, counting what stands, and while it never has. No eero or no
# floor would be an empty net read as an all-clear, so a count of either starts at one.
COUNTED = re.compile(r"^\d+ devices? on [1-9]\d* eeros? across [1-9]\d* floors?, and .+\.$")
UNANSWERED = "Net hasn't answered, so no device is drawn."


@pytest.fixture
def net_daemon(tmp_path: Path) -> Iterator[StandIn]:
    yield from standing_in(tmp_path / "net.sock", "net", HERE / "net.json")


@pytest.fixture
def mycelium_node(tmp_path: Path) -> Iterator[StandIn]:
    yield from standing_in(tmp_path / "node.sock", "node", HERE / "node.json")


def feeds_of(net: StandIn, node: StandIn) -> dict[str, object]:
    """Give the server's feeds from `net` and `node`, trimmed to the kinds home.nix declares."""
    return {
        "net": {"socket": str(net.path), "kinds": json.loads(os.environ["NET_KINDS"])},
        "node": {"socket": str(node.path), "kinds": json.loads(os.environ["NODE_KINDS"])},
    }


@pytest.fixture
def network_served(tmp_path: Path, net_daemon: StandIn, mycelium_node: StandIn) -> Iterator[Served]:
    feeds = feeds_of(net_daemon, mycelium_node)
    with serving(tmp_path, {"network": os.environ["NETWORK_SITE"]}, feeds) as served:
        yield served


@pytest.fixture
def network_opened(browser: Browser, network_served: Served) -> Iterator[Opened]:
    root, _ = network_served
    opened = Opened(browser, f"{root}network/", SIZES[-1], "dark")
    yield opened
    opened.close()


@pytest.fixture
def network_page(network_opened: Opened) -> Page:
    return network_opened.page


def says_it_stands(page: Page, state: str, timeout: int = PACE_MS) -> None:
    expect(page.locator("#asof"), f"the network page never said it was {state}").to_have_attribute(
        "data-state", state, timeout=timeout
    )


def device(page: Page, key: str) -> Locator:
    return page.locator(f'#tree [role=treeitem][data-id="{key}"]')


def placed(page: Page, key: str) -> Locator:
    return page.locator(f'#map-draw [data-level=device][data-id="{key}"]')


def every_device_drawn(page: Page) -> None:
    for key in DEVICES:
        expect(device(page, key), f"the tree drew no {key}").to_have_count(1, timeout=PACE_MS)
        expect(placed(page, key), f"the map drew no {key}").to_have_count(1, timeout=PACE_MS)


def opened_on(page: Page, key: str) -> Locator:
    expect(device(page, key), f"the tree drew no {key} to open").to_have_count(1, timeout=PACE_MS)
    device(page, key).locator(".ti").click()
    return page.locator("#panel-body")


def test_the_network_page_draws_every_device_live(network_page: Page) -> None:
    says_it_stands(network_page, "live")
    every_device_drawn(network_page)


def test_a_name_from_the_wifi_is_drawn_as_words(network_page: Page) -> None:
    says_it_stands(network_page, "live")
    expect(device(network_page, TV), "the TV's name wasn't drawn as words").to_contain_text(
        TV_NAME, timeout=PACE_MS
    )
    expect(placed(network_page, TV), "the TV's name wasn't on the map as words").to_contain_text(
        TV_NAME
    )
    expect(network_page.locator("main b"), "the TV's name was read as markup").to_have_count(0)


def test_a_device_that_goes_leaves_the_tree(network_page: Page, net_daemon: StandIn) -> None:
    says_it_stands(network_page, "live")
    every_device_drawn(network_page)
    # net drops a device with its link, as one reading of the house.
    net_daemon.tell("link", gone=[PRINTER_LINK])
    net_daemon.tell("device", gone=[f"device:{PRINTER}"])
    expect(device(network_page, PRINTER), "the gone printer was still drawn").to_have_count(
        0, timeout=PACE_MS
    )
    expect(placed(network_page, PRINTER), "the gone printer was still on the map").to_have_count(0)
    expect(device(network_page, "eero:2"), "the printer's eero went too").to_have_count(1)
    expect(placed(network_page, "eero:2"), "the printer's eero left the map").to_have_count(1)


def test_a_machine_mycelium_runs_on_links_to_its_place_on_the_map(
    network_page: Page, mycelium_node: StandIn
) -> None:
    says_it_stands(network_page, "live")
    panel = opened_on(network_page, KESTREL)
    expect(panel, "kestrel's panel didn't say what runs there").to_contain_text(
        "2: 1 waiting on you, 1 moving", timeout=PACE_MS
    )
    home = panel.locator('a[href="../mycelium/#machine=kestrel"]')
    expect(home, "kestrel wasn't linked to its place on the map").to_have_text(
        "kestrel on the mycelium map"
    )
    mycelium_node.tell("cell", gone=["cell:TERN01"])
    expect(panel, "kestrel's panel didn't follow the node").to_contain_text(
        "1: 1 moving", timeout=PACE_MS
    )


def test_a_net_that_stops_is_said_broken_then_live(network_page: Page, net_daemon: StandIn) -> None:
    says_it_stands(network_page, "live")
    every_device_drawn(network_page)
    net_daemon.leave()
    says_it_stands(network_page, "broken")
    # What net holds moves while it's away, so only reading it again shows it.
    net_daemon.tell("device", gone=[f"device:{PRINTER}"])
    net_daemon.start()
    says_it_stands(network_page, "live", BACK_MS)
    expect(device(network_page, PRINTER), "the page didn't read net again").to_have_count(
        0, timeout=PACE_MS
    )
    expect(device(network_page, "modem"), "the modem went with the printer").to_have_count(1)


def test_a_net_that_never_answers_says_so_in_the_lede(
    browser: Browser, tmp_path: Path, net_daemon: StandIn, mycelium_node: StandIn
) -> None:
    net_daemon.leave()
    feeds = feeds_of(net_daemon, mycelium_node)
    with serving(tmp_path, {"network": os.environ["NETWORK_SITE"]}, feeds) as (root, _):
        first = Opened(browser, f"{root}network/", SIZES[-1], "dark")
        later = None
        try:
            lede = first.page.locator("#lede")
            expect(lede, "the lede still said net was being reached").to_have_text(
                UNANSWERED, timeout=BACK_MS
            )
            # A page joining the watch the server holds as broken is told
            # every kind empty, which mustn't read as a calm, empty house.
            later = Opened(browser, f"{root}network/", SIZES[-1], "dark")
            expect(later.page.locator("#lede"), "a later page's lede didn't say so").to_have_text(
                UNANSWERED, timeout=BACK_MS
            )
            net_daemon.start()
            for page in (first.page, later.page):
                expect(page.locator("#lede"), "the lede didn't count net once back").to_have_text(
                    COUNTED, timeout=BACK_MS
                )
        finally:
            first.close()
            if later is not None:
                later.close()


def test_a_sites_server_that_goes_is_said_stale(
    network_opened: Opened, network_served: Served
) -> None:
    network_page = network_opened.page
    says_it_stands(network_page, "live")
    _, server = network_served
    network_opened.loses(server)
    says_it_stands(network_page, "stale")
    network_page.wait_for_timeout(REDRAWN_MS)
    every_device_drawn(network_page)
    says_it_stands(network_page, "stale")


# How far right of the map's room, short of any scrollbar's gutter, each device's box reaches in
# pixels, how far the room scrolls sideways, and how far left of the drawing's edge any wire runs
# in its own units: all nothing once each floor stacks.
PAST_THE_ROOM = """() => {
  const view = document.querySelector("#map-view");
  const right = view.getBoundingClientRect().left + view.clientLeft + view.clientWidth;
  const boxes = [...document.querySelectorAll("#map-draw [data-level=device]")];
  const reach = Math.max(...boxes.map((box) => box.getBoundingClientRect().right));
  const wires = [...document.querySelectorAll("#map-draw .n-link")];
  return {
    boxes: reach - right,
    scroll: view.scrollWidth - view.clientWidth,
    wires: -Math.min(0, ...wires.map((wire) => wire.getBBox().x)),
  };
}"""

# Each zoom button's width and height, and how far it reaches past the buttons' fieldset.
ZOOM_BUTTONS = """() => {
  const zoom = document.querySelector(".zoom").getBoundingClientRect();
  return [...document.querySelectorAll(".zoom button")].map((button) => {
    const at = button.getBoundingClientRect();
    const past = Math.max(at.right - zoom.right, zoom.left - at.left);
    return { wide: at.width, tall: at.height, past };
  });
}"""

# A finger's reach, which each zoom button keeps.
REACH_PX = 44

# Whether the map is drawn across, the kestrel's row beside its eero rather than under it; null
# while a redraw has either missing.
DRAWN_ACROSS = """(kestrel) => {
  const eero = document.querySelector('#map-draw [data-id="eero:1"]');
  const row = document.querySelector(`#map-draw [data-id="${kestrel}"]`);
  if (eero === null || row === null) {
    return null;
  }
  return row.getBBox().x >= eero.getBBox().x + eero.getBBox().width;
}"""


def stands_within_the_room(page: Page, where: str) -> None:
    past = page.evaluate(PAST_THE_ROOM)
    assert past["boxes"] <= 0, f"a device reaches {past['boxes']} px past {where}'s map"
    assert past["scroll"] <= 0, f"{where}'s map scrolls {past['scroll']} px sideways"
    assert past["wires"] <= 0, f"a wire runs {past['wires']} units off {where}'s map"


def test_a_phone_sees_every_device_without_scrolling_sideways(
    browser: Browser, network_served: Served
) -> None:
    root, _ = network_served
    opened = Opened(browser, f"{root}network/", SIZES[0], "dark")
    try:
        says_it_stands(opened.page, "live")
        every_device_drawn(opened.page)
        stands_within_the_room(opened.page, "a phone")
        for button in opened.page.evaluate(ZOOM_BUTTONS):
            assert button["wide"] >= REACH_PX, f"a zoom button is {button['wide']} px wide"
            assert button["tall"] >= REACH_PX, f"a zoom button is {button['tall']} px tall"
            assert button["past"] <= 0, f"a zoom button spills {button['past']} px past its set"
    finally:
        opened.close()


def test_narrowing_a_desk_to_a_phone_draws_the_map_the_other_way(
    browser: Browser, network_served: Served
) -> None:
    root, _ = network_served
    desk, phone = SIZES[-1], SIZES[0]
    opened = Opened(browser, f"{root}network/", desk, "dark")
    try:
        page = opened.page
        says_it_stands(page, "live")
        every_device_drawn(page)
        lede = page.locator("#lede")
        expect(lede, "a desk's lede didn't count what stands").to_have_text(COUNTED)
        assert page.evaluate(DRAWN_ACROSS, KESTREL), "a desk's map wasn't drawn across"
        page.set_viewport_size({"width": phone[0], "height": phone[1]})
        stacked = f"(kestrel) => ({DRAWN_ACROSS})(kestrel) === false"
        page.wait_for_function(stacked, arg=KESTREL, timeout=PACE_MS)
        every_device_drawn(page)
        expect(lede, "the lede lost its count as the map stacked").to_have_text(COUNTED)
        stands_within_the_room(page, "a narrowed window")
        page.set_viewport_size({"width": desk[0], "height": desk[1]})
        across = f"(kestrel) => ({DRAWN_ACROSS})(kestrel) === true"
        page.wait_for_function(across, arg=KESTREL, timeout=PACE_MS)
    finally:
        opened.close()


# The room across takes at the smallest zoom, in units (grid.ts's ACROSS_LEAST, and the same
# less its floors' names' column), and that zoom (view.ts's ZOOMS[0]). A window whose map's room
# falls between the two must stack, or the names' column is left out of the measure again.
ACROSS_LEAST = 924
ACROSS_UNNAMED = 884
SMALLEST_ZOOM = 12 / 13
# A narrow window with no panel beside the map, whose room falls between those two.
JUST_SHORT = (864, 700)
ROOM = """() => document.querySelector("#map-view").clientWidth"""


def test_a_window_just_short_of_the_map_across_stacks_it(
    browser: Browser, network_served: Served
) -> None:
    root, _ = network_served
    opened = Opened(browser, f"{root}network/", JUST_SHORT, "dark")
    try:
        page = opened.page
        says_it_stands(page, "live")
        every_device_drawn(page)
        room = page.evaluate(ROOM)
        least, unnamed = ACROSS_LEAST * SMALLEST_ZOOM, ACROSS_UNNAMED * SMALLEST_ZOOM
        assert unnamed <= room < least, f"the map's room is {room} px, not in [{unnamed}, {least})"
        assert page.evaluate(DRAWN_ACROSS, KESTREL) is False, "the map was drawn across"
        stands_within_the_room(page, f"a {JUST_SHORT[0]} px window")
    finally:
        opened.close()


# A bigger house than net.json's: an attic eero, a switch on each of two floors, and the basement
# full past one column, so stacked it takes four lanes of wires, run closer to keep the map's
# width, and across a second column.
ATTIC = "eero:3"
SWITCHES = {"switch:1": ["eero:1", "modem"], "switch:2": ["eero:2", "eero:1", "modem"]}
CROWD = [f"mac:00:00:5e:00:53:4{n}" for n in range(6)]
# A laptop's width, where the panel beside the map leaves it too little room to draw across.
LAPTOP = (1024, 800)


def a_bigger_house(net_daemon: StandIn) -> None:
    def held(key: str, name: str, roles: list[str], through: list[str]) -> dict[str, object]:
        named = {"key": f"device:{key}", "kind": "device", "id": key, "name": name}
        read = {"roles": roles, "addresses": [], "macs": [], "source": "eero"}
        return {**named, **read, "path": {"through": through}}

    def tie(key: str, up: str, **how: object) -> dict[str, object]:
        ends = {"a": {"device": key}, "b": {"device": up}}
        return {"key": f"link:{up}|{key}", "kind": "link", **ends, "source": "eero", **how}

    wifi = {"medium": "wifi", "speed_mbps": 144, "signal_dbm": -60, "crossings": 0}
    cable = {"medium": "ethernet", "speed_mbps": 1000, "crossings": 0}
    devices = [held(ATTIC, "Attic", ["access-point"], ["eero:2", "eero:1", "modem"])]
    links = [tie(ATTIC, "eero:2", **wifi)]
    for n, (switch, through) in enumerate(SWITCHES.items()):
        devices.append(held(switch, f"Switch {n + 1}", ["switch"], through))
        devices[-1]["ports"] = [{"name": "1", "kind": "ethernet", "link": "up"}]
        links.append(tie(switch, through[0], **cable))
    for n, phone in enumerate(CROWD):
        devices.append(held(phone, f"Phone {n + 1}", ["client"], ["eero:1", "modem"]))
        links.append(tie(phone, "eero:1", **wifi))
    net_daemon.tell("device", devices)
    net_daemon.tell("link", links)


@pytest.mark.parametrize("size", [SIZES[0], LAPTOP])
def test_a_bigger_house_stacks_within_the_room(
    browser: Browser, network_served: Served, net_daemon: StandIn, size: tuple[int, int]
) -> None:
    root, _ = network_served
    # Told before the page opens, so its first drawing holds the whole house.
    a_bigger_house(net_daemon)
    opened = Opened(browser, f"{root}network/", size, "dark")
    try:
        says_it_stands(opened.page, "live")
        every_device_drawn(opened.page)
        for key in [ATTIC, *SWITCHES, *CROWD]:
            expect(placed(opened.page, key), f"the map drew no {key}").to_have_count(1)
        # Links come over their own watch, so a house drawn before they arrive takes no lanes;
        # measured once each switch and the attic is wired, the lanes are all there.
        for key in [ATTIC, *SWITCHES]:
            ends = f'#map-draw .n-link[data-from="{key}"], #map-draw .n-link[data-to="{key}"]'
            expect(opened.page.locator(ends), f"the map drew no wire to {key}").not_to_have_count(0)
        stands_within_the_room(opened.page, f"a {size[0]} px window")
    finally:
        opened.close()


def test_the_network_page_keeps_to_its_budget(network_opened: Opened) -> None:
    says_it_stands(network_opened.page, "live")
    # Counted once every device the feeds hold is drawn, so the count is the page's whole and
    # not how far it got.
    every_device_drawn(network_opened.page)
    network_opened.keeps_to(BUDGET)
    says_it_stands(network_opened.page, "live")


@pytest.mark.parametrize("look", LOOKS)
@pytest.mark.parametrize("size", SIZES)
def test_the_network_page_passes_the_skills_page_checks(
    browser: Browser, network_served: Served, size: tuple[int, int], look: str
) -> None:
    root, _ = network_served
    opened = Opened(browser, f"{root}network/", size, look)
    try:
        says_it_stands(opened.page, "live")
        every_device_drawn(opened.page)
        opened.passes_the_skills_checks()
    finally:
        # Closed whatever failed, so what the page logged is said too.
        opened.close()
