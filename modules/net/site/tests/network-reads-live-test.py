"""The network page reads live from net and the node, through the sites' server, in a browser.

The page is the one Nix builds, served by the real local-sites server beside a
stand-in net daemon and a stand-in node (modules/local-sites/tests/
stand_in_daemon.py), holding a made-up house as net tells it (net.json) and the
one machine mycelium runs on there with two cells (node.json), each feed
trimmed to the kinds home.nix declares (NET_KINDS, NODE_KINDS). It must say it's
live with every device drawn in its tree, drop a device that goes, say in a
machine's panel what runs there and link it to its place on the mycelium map,
say broken when net stops and live again, its devices read again, when it's
back, say stale when the server goes, keep to its budget, and pass the
web-design skill's page checks at a phone's width and a desk's in both looks.
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
# The browser's own line for a feed that couldn't connect, once the server is gone.
UNREACHED = "WebSocket connection to"
# Every device net.json holds, by the id its tree item carries.
DEVICES = ["modem", "eero:1", "eero:2", "mac:00:00:5e:00:53:10", "mac:00:00:5e:00:53:20"]
PRINTER = "mac:00:00:5e:00:53:10"
# The laptop mycelium runs on, by the id net gives it and the name both give it.
KESTREL = "mac:00:00:5e:00:53:20"
# What a first visit to the network page may cost, drawn from the stand-in's house.
BUDGET = Budget(files=36, kilobytes=545, elements=335)


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


def every_device_drawn(page: Page) -> None:
    for key in DEVICES:
        expect(device(page, key), f"the tree drew no {key}").to_have_count(1, timeout=PACE_MS)


def test_the_network_page_draws_every_device_live(network_page: Page) -> None:
    says_it_stands(network_page, "live")
    every_device_drawn(network_page)
    expect(
        network_page.locator("#map-draw g, #map-draw rect").first,
        "the map drew nothing",
    ).to_be_attached()


def test_a_device_that_goes_leaves_the_tree(network_page: Page, net_daemon: StandIn) -> None:
    says_it_stands(network_page, "live")
    every_device_drawn(network_page)
    net_daemon.tell("device", gone=[f"device:{PRINTER}"])
    expect(device(network_page, PRINTER), "the gone printer was still drawn").to_have_count(
        0, timeout=PACE_MS
    )
    expect(device(network_page, "eero:2"), "the printer's eero went too").to_have_count(1)


def test_a_machine_mycelium_runs_on_links_to_its_place_on_the_map(network_page: Page) -> None:
    says_it_stands(network_page, "live")
    device(network_page, KESTREL).locator(".ti").click()
    panel = network_page.locator("#panel-body")
    expect(panel, "kestrel's panel didn't say what runs there").to_contain_text(
        "2: 1 waiting on you, 1 moving", timeout=PACE_MS
    )
    home = panel.locator('a[href="../mycelium/#machine=kestrel"]')
    expect(home, "kestrel wasn't linked to its place on the map").to_have_text(
        "kestrel on the mycelium map"
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
    expect(device(network_page, "modem"), "the modem wasn't drawn again").to_have_count(1)


def test_a_sites_server_that_goes_is_said_stale(
    network_opened: Opened, network_served: Served
) -> None:
    network_page = network_opened.page
    says_it_stands(network_page, "live")
    # Once the server's gone, the browser logs each feed's next try as a line of its own.
    network_opened.expecting(UNREACHED)
    _, server = network_served
    server.kill()
    says_it_stands(network_page, "stale")
    every_device_drawn(network_page)


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
