"""What a live page's budget says, failing or passing, as a run of pytest shows it.

Each case runs a test of its own in pytest beside this one (pytester),
loading live_page as every page's test does, on a tab that stands in for
Chromium's: it hands keeps_to the files a first visit asked for, each with
the body the server sent, then answers every file of the second visit as
unchanged. So what keeps_to says when a visit is past its budget, and what
the run's "first visits" summary says either way, is read as a reader of the
run reads it, without a browser or a page.
"""

import re
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    import pytest

# A test run beside this one, holding a tab to a budget as a page's own test
# does, on the files and bodies its case hands it.
VISITED = """
from live_page import Budget, Opened


class Answer:
    def __init__(self, status):
        self.status = status


class Asked:
    def __init__(self, url, body, status=200):
        self.url, self.body, self.status = url, body, status

    def sizes(self):
        return {{"responseBodySize": self.body}}

    def response(self):
        return Answer(self.status)


class Drawn:
    url = "http://sites.localhost:8080/web/"

    def __init__(self, tab):
        self.tab = tab

    def wait_for_load_state(self, state):
        pass

    def reload(self, wait_until):
        self.tab.asked.extend(Asked(each.url, 0, 304) for each in self.first)

    def evaluate(self, script):
        return 40 if "length" in script else [["div", 30]]


def visited(files):
    tab = Opened.__new__(Opened)
    tab.address, tab.sockets = "http://sites.localhost:8080/web/", []
    tab.asked = [Asked(f"http://sites.localhost:8080/web/{{name}}", body) for name, body in files]
    tab.page = Drawn(tab)
    tab.page.first = list(tab.asked)
    return tab


def test_a_visit():
    visited({files}).keeps_to(Budget(files=2, kilobytes=1, elements=50))
"""


def said(ran: pytest.RunResult, lines: list[str]) -> None:
    """Find `lines` in what `ran` printed, in order, each as written anywhere in a line of it."""
    ran.stdout.re_match_lines([".*" + re.escape(line) for line in lines])


def run(pytester: pytest.Pytester, files: list[tuple[str, int]]) -> pytest.RunResult:
    """Run a test holding a first visit of `files` to a budget of 2 files and 1 KB."""
    pytester.makepyfile(test_visit=VISITED.format(files=files))
    return pytester.runpytest_subprocess("-p", "live_page", "-p", "no:cacheprovider")


def test_a_visit_within_its_budget_passes_and_says_what_it_cost(
    pytester: pytest.Pytester,
) -> None:
    ran = run(pytester, [("index.html", 600), ("site.css", 300)])
    ran.assert_outcomes(passed=1)
    said(
        ran,
        [
            "- first visits -",
            (
                "test_visit.py::test_a_visit: /web/ cost a first visit 2 files, 0.9 KB (900 bytes) "
                "and 40 elements, against a budget of 2 files, 1 KB (1024 bytes) and 50 elements"
            ),
        ],
    )


def test_a_visit_past_its_budget_names_each_measure_past_it_and_is_still_summed_up(
    pytester: pytest.Pytester,
) -> None:
    ran = run(pytester, [("index.html", 600), ("site.css", 300), ("site.js", 200)])
    ran.assert_outcomes(failed=1)
    cost = (
        "/web/ cost a first visit 3 files, 1.1 KB (1100 bytes) and 40 elements, against a "
        "budget of 2 files, 1 KB (1024 bytes) and 50 elements"
    )
    said(
        ran,
        [
            (
                f"AssertionError: {cost}: past its budget in files, asking for "
                "['http://sites.localhost:8080/web/index.html', "
                "'http://sites.localhost:8080/web/site.css', "
                "'http://sites.localhost:8080/web/site.js']; and in kilobytes"
            ),
            "- first visits -",
            f"test_visit.py::test_a_visit: {cost}",
        ],
    )


def test_a_visit_asking_for_a_file_twice_is_refused_naming_it(
    pytester: pytest.Pytester,
) -> None:
    ran = run(pytester, [("index.html", 600), ("index.html", 0)])
    ran.assert_outcomes(failed=1)
    said(
        ran,
        [
            (
                "past its budget in files asked for more than once, "
                "['http://sites.localhost:8080/web/index.html']"
            ),
        ],
    )
