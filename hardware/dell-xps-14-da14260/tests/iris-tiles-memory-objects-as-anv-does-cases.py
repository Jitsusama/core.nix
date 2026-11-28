"""Prove the iris tiling check on edited copies of the files it reads.

    iris-tiles-memory-objects-as-anv-does-cases BASH CHECK RESOURCE IMAGE OWN

Run with CHECK the check's script, RESOURCE and IMAGE the patched iris_resource.c and
anv_image.c, and OWN Mesa's own iris_resource.c. Each case edits a copy of one patched
file the way a later Mesa, or a patch carried badly, might, and says the words the
check's failure must hold. A case whose edit no longer changes its file fails by its
name, so a moved driver can't leave a case proving nothing. The files as they are pass
in the check this runs beside.
"""

import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

RESOURCE = "iris_resource.c"
IMAGE = "anv_image.c"
MEMORY_OBJECT = "   if (!res->mod_info && res->external_format != PIPE_FORMAT_NONE) {"
NO_AUX = "      usage |= ISL_SURF_USAGE_DISABLE_AUX_BIT;"
PREFER_4K = "      usage |= ISL_SURF_USAGE_PREFER_4K_ALIGNMENT;"
CONST_BW = "   else if (templ->bind & PIPE_BIND_CONST_BW)\n" + NO_AUX + "\n"
EXPORTED = "   if (image->vk.external_handle_types != 0 &&\n"
# The exported image's preference, after the flag that turns compression off, since
# an earlier one asks for 4K tiles only for a requested alignment.
ANV_NO_AUX = "      isl_extra_usage_flags |= ISL_SURF_USAGE_DISABLE_AUX_BIT;\n"
ANV_PREFER_4K = ANV_NO_AUX + "      isl_extra_usage_flags |= ISL_SURF_USAGE_PREFER_4K_ALIGNMENT;\n"
UNREADABLE = "can't tell how a memory object is tiled"

# Each case: its name, the file it edits, what it replaces and with what, and the words
# its failure must hold.
CASES: list[tuple[str, str, str, str, str]] = [
    (
        "the memory object's branch an else again",
        RESOURCE,
        MEMORY_OBJECT,
        "   else" + MEMORY_OBJECT[2:],
        "only when no branch before it matched",
    ),
    (
        "the memory object's preference commented out",
        RESOURCE,
        NO_AUX + "\n" + PREFER_4K,
        NO_AUX + "\n   // " + PREFER_4K.strip(),
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
        UNREADABLE,
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
]


def answer(bash: str, check: str, files: list[Path]) -> tuple[int, str]:
    """Run the check over the files, giving its status and its words."""
    # S603: bash and the check are the build's own, from Nix, and no shell reads the names.
    run = subprocess.run(  # noqa: S603
        [bash, check, *map(str, files)], capture_output=True, text=True, check=False
    )
    return run.returncode, run.stdout + run.stderr


def wrong(bash: str, check: str, resource: str, image: str, own: str) -> list[str]:
    """Give a line for each case the check answers wrongly."""
    lines = []
    for name, edits, before, after, expected in CASES:
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            copies = {RESOURCE: root / RESOURCE, IMAGE: root / IMAGE}
            shutil.copyfile(resource, copies[RESOURCE])
            shutil.copyfile(image, copies[IMAGE])
            edited = copies[edits]
            text = edited.read_text()
            if before not in text:
                lines.append(f"{name}: its edit no longer changes {edits}; move it with Mesa.")
                continue
            edited.write_text(text.replace(before, after, 1))
            status, said = answer(bash, check, [copies[RESOURCE], copies[IMAGE], Path(own)])
        if status == 0 or expected not in said:
            lines.append(f"{name}: should fail saying {expected!r}, and said: {said.strip()}")
    return lines


def main(argv: list[str]) -> None:
    """Fail, naming each case, unless the check answers every case rightly."""
    bash, check, resource, image, own = argv
    lines = wrong(bash, check, resource, image, own)
    if lines:
        sys.exit("The iris tiling check answers these edits wrongly:\n" + "\n".join(lines))


if __name__ == "__main__":
    main(sys.argv[1:])
