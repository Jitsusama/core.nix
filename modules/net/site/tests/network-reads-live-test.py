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
devices read again, when it's back, say stale when the server goes and keep
what it drew, keep to its budget, and pass the web-design skill's page checks
at a phone's width and a desk's in both looks.
Anything the page logs fails it, but for the browser's own lines once the test
takes the server away. The server, the browser and the tab are
modules/local-sites/tests/live_page.py's.
"""

import json
import os
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
# What a first visit to the network page may cost, drawn from the stand-in's house.
BUDGET = Budget(files=36, kilobytes=514, elements=335)


@pytest.fixture
def net_daemon(tmp_path: Path) -> Iterator[StandIn]:
    yield from standing_in(tmp_path / "net.sock", "net", HERE / "net.json")


@pytest.fixture
def mycelium_node(tmp_path: Path) -> Iterator[StandIn]:
    yield from standing_in(tmp_path / "node.sock", "node", HERE / "node.json")


@pytest.fixture
def network_served(tmp_path: Path, net_daemon: StandIn, mycelium_node: StandIn) -> Iterator[Served]:
    feeds = {
        "net": {"socket": str(net_daemon.path), "kinds": json.loads(os.environ["NET_KINDS"])},
        "node": {"socket": str(mycelium_node.path), "kinds": json.loads(os.environ["NODE_KINDS"])},
    }
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
