"""The fixtures every test of run-subtests is given: the tool, and stand-ins.

The tool is the file RUN_SUBTESTS names, loaded as a module of no package,
and the stand-ins are stand_ins.py's, laid out afresh for each test.
"""

import importlib.machinery
import importlib.util
import os
import shutil
import tempfile
from pathlib import Path
from typing import TYPE_CHECKING, cast

import pytest
from stand_ins import StandIn, Tool

if TYPE_CHECKING:
    from collections.abc import Iterator


@pytest.fixture(scope="session", name="tool")
def loaded_tool() -> Tool:
    """Load the tool from its path, as a module of no package."""
    path = os.environ["RUN_SUBTESTS"]
    loader = importlib.machinery.SourceFileLoader("run_subtests", path)
    spec = importlib.util.spec_from_loader(loader.name, loader)
    if spec is None:
        pytest.fail(f"{path} can't be loaded")
    module = importlib.util.module_from_spec(spec)
    loader.exec_module(module)
    return cast("Tool", module)


@pytest.fixture(name="stand_in")
def a_stand_in(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Iterator[StandIn]:
    """Give a check's driver and nix, as stand-ins, for one test."""
    runtime = Path(tempfile.mkdtemp(prefix="rt."))
    yield StandIn(tmp_path, runtime, monkeypatch)
    shutil.rmtree(runtime)
