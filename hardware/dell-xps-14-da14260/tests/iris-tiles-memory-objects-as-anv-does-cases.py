"""Prove the iris tiling check on edited copies of the files it reads.

    iris-tiles-memory-objects-as-anv-does-cases BASH CHECK RESOURCE IMAGE OWN

Run with CHECK the check's script, RESOURCE and IMAGE the patched iris_resource.c and
anv_image.c, and OWN Mesa's own iris_resource.c. Each case edits a copy of one of them
the way a later Mesa, or a patch carried badly, might, and says the words the check's
failure must hold. Each edit lands only in the function the check reads from that
file, and a case whose edit no longer changes that function is named apart, as one to
move with Mesa here, so a moved driver can't leave a case proving nothing. The files as
they are pass in the check this runs beside.
"""

import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

RESOURCE = "iris_resource.c"
IMAGE = "anv_image.c"
OWN = "Mesa's own iris_resource.c"
# The function the check reads from each file, by how its definition starts.
FUNCTIONS = {
    RESOURCE: "iris_resource_configure_main(",
    IMAGE: "anv_image_init(",
    OWN: "iris_resource_configure_main(",
}
MEMORY_OBJECT = "   if (!res->mod_info && res->external_format != PIPE_FORMAT_NONE) {"
NO_AUX = "      usage |= ISL_SURF_USAGE_DISABLE_AUX_BIT;"
PREFER_4K = "      usage |= ISL_SURF_USAGE_PREFER_4K_ALIGNMENT;"
CONST_BW = "   else if (templ->bind & PIPE_BIND_CONST_BW)\n" + NO_AUX + "\n"
EXPORTED = "   if (image->vk.external_handle_types != 0 &&\n"
# The exported image's preference, after the flag that turns compression off, since
# an earlier one asks for 4K tiles only for a requested alignment.
ANV_NO_AUX = "      isl_extra_usage_flags |= ISL_SURF_USAGE_DISABLE_AUX_BIT;\n"
ANV_PREFER_4K = ANV_NO_AUX + "      isl_extra_usage_flags |= ISL_SURF_USAGE_PREFER_4K_ALIGNMENT;\n"
SHADOWED = "only when no branch before it matched"
# Named whole, since in the build this file runs from the store under another name.
STALE = (
    "These cases no longer find what they edit, which says nothing yet about the check:"
    " move them with Mesa in hardware/dell-xps-14-da14260/tests/"
    "iris-tiles-memory-objects-as-anv-does-cases.py."
)

# Each case: its name, the file it edits, what it replaces and with what, and the words
# its failure must hold.
CASES: list[tuple[str, str, str, str, str]] = [
    (
        "the memory object's branch an else again",
        RESOURCE,
        MEMORY_OBJECT,
        "   else" + MEMORY_OBJECT[2:],
        SHADOWED,
    ),
    (
        "the memory object's branch an else of the line before",
        RESOURCE,
        MEMORY_OBJECT,
        "   else\n" + MEMORY_OBJECT,
        SHADOWED,
    ),
    (
        "the memory object's branch an else before the comment over it",
        RESOURCE,
        "   /* A memory object carries no modifier",
        "   else\n   /* A memory object carries no modifier",
        SHADOWED,
    ),
    (
        "the memory object's preference commented out",
        RESOURCE,
        NO_AUX + "\n" + PREFER_4K,
        NO_AUX + "\n   // " + PREFER_4K.strip(),
        "doesn't prefer 4K tiles for a memory object",
    ),
    (
        "the memory object's preference moved to the branch after it",
        RESOURCE,
        NO_AUX + "\n" + PREFER_4K + "\n   }",
        NO_AUX + "\n   } else if (templ->bind & PIPE_BIND_SCANOUT) {\n" + PREFER_4K + "\n   }",
        "doesn't prefer 4K tiles for a memory object",
    ),
    (
        "the memory object compressed",
        RESOURCE,
        MEMORY_OBJECT + "\n" + NO_AUX,
        MEMORY_OBJECT,
        "compresses a memory object anv wrote uncompressed",
    ),
    (
        "the memory object's branch renamed",
        RESOURCE,
        MEMORY_OBJECT,
        MEMORY_OBJECT.replace("external_format", "imported_format"),
        "has no branch for a memory object as it was written",
    ),
    ("the constant-bandwidth branch lost", RESOURCE, CONST_BW, "", "constant-bandwidth"),
    ("anv's preference dropped", IMAGE, ANV_PREFER_4K, ANV_NO_AUX, "anv no longer prefers 4K"),
    (
        "anv's condition narrowed by a third line",
        IMAGE,
        EXPORTED,
        EXPORTED + "       image->vk.usage == 0 &&\n",
        "no longer asks only for no modifier",
    ),
    (
        "Mesa's own iris fixed",
        OWN,
        "   else" + MEMORY_OBJECT[2:],
        MEMORY_OBJECT,
        "Mesa's own iris passes too",
    ),
    (
        "Mesa's own iris fixed without the preference",
        OWN,
        "   else" + MEMORY_OBJECT[2:] + "\n" + NO_AUX + "\n" + PREFER_4K,
        MEMORY_OBJECT + "\n" + NO_AUX,
        "failed for another reason",
    ),
    (
        "Mesa's own iris unreadable",
        OWN,
        "\niris_resource_configure_main(",
        "\niris_resource_configure_surface(",
        "failed for another reason",
    ),
]


def answer(bash: str, check: str, files: list[Path]) -> tuple[int, str]:
    """Run the check over the files, giving its status and its words."""
    # S603: bash and the check are the build's own, from Nix, and no shell reads the names.
    run = subprocess.run(  # noqa: S603
        [bash, check, *map(str, files)], capture_output=True, text=True, check=False
    )
    return run.returncode, run.stdout + run.stderr


def function(text: str, starts: str) -> tuple[int, int] | None:
    """Give where the function whose definition starts so begins and ends in the text."""
    start = text.find("\n" + starts)
    end = text.find("\n}", start)
    return None if start < 0 or end < 0 else (start, end + 2)


def judge(bash: str, check: str, sources: dict[str, str]) -> tuple[list[str], list[str]]:
    """Give the cases the check answers wrongly, and those whose edit no longer applies."""
    wrong, stale = [], []
    for name, edits, before, after, expected in CASES:
        with tempfile.TemporaryDirectory() as folder:
            copies = {key: Path(folder) / str(index) for index, key in enumerate(sources)}
            for key, path in sources.items():
                shutil.copyfile(path, copies[key])
            text = copies[edits].read_text()
            span = function(text, FUNCTIONS[edits])
            if span is None or before not in text[span[0] : span[1]]:
                stale.append(f"{name}: its edit no longer changes {FUNCTIONS[edits]}) in {edits}.")
                continue
            start, end = span
            edited = text[:start] + text[start:end].replace(before, after, 1) + text[end:]
            copies[edits].write_text(edited)
            status, said = answer(bash, check, list(copies.values()))
        if status == 0 or expected not in said:
            wrong.append(f"{name}: should fail saying {expected!r}, and said: {said.strip()}")
    return wrong, stale


def main(argv: list[str]) -> None:
    """Fail, naming each case, unless the check answers every case rightly."""
    bash, check, resource, image, own = argv
    wrong, stale = judge(bash, check, {RESOURCE: resource, IMAGE: image, OWN: own})
    lines = []
    if wrong:
        lines += ["The iris tiling check answers these edits wrongly:", *wrong]
    if stale:
        lines += [STALE, *stale]
    if lines:
        sys.exit("\n".join(lines))


if __name__ == "__main__":
    main(sys.argv[1:])
