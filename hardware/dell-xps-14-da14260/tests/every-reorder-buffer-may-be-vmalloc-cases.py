"""Prove the reorder-buffer check on edited copies of the files it reads.

    every-reorder-buffer-may-be-vmalloc-cases BASH CHECK FILE...

Run from the patched source, with CHECK the check's script and FILE each iwlmld file it
reads, the session's allocation in the first. Each case edits a copy of the files the
way a later kernel might, and says what the check must answer: a pass, or a failure
whose words hold those given. A case whose edit no longer changes its file fails by its
name, so a moved driver can't leave a case proving nothing. The files as they are pass
in the check this runs beside.
"""

import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

AGG = "agg.c"
MLD = "mld.c"
ALLOCATED = "baid_data = kvzalloc("
FREED = "\tkvfree(baid_data);"
READ = "\tba_data = rcu_dereference(mld->fw_id_to_ba[baid]);"
DECLARED = "\tstruct iwl_mld_baid_data *ba_data;"
TAKEN = "\t\tdata = wiphy_dereference(mld->wiphy, mld->fw_id_to_ba[baid]);"
OTHER = "\tkfree(mld->nvm_data);"
KMALLOC = "takes only kmalloc's memory"

# Each case: its name, the file it edits by its name's end, what it replaces and with
# what, and the words its failure must hold, or None where it must pass.
CASES: list[tuple[str, str, str, str, str | None]] = [
    ("the allocation back to kzalloc", AGG, ALLOCATED, "baid_data = kzalloc(", "order-5"),
    ("baid_data freed by kfree", AGG, FREED, "\tkfree(baid_data);", KMALLOC),
    ("ba_data freed by kfree", AGG, READ, READ + "\n\tkfree(ba_data);", KMALLOC),
    ("data freed by kfree", AGG, TAKEN, TAKEN + "\n\t\tkfree(data);", KMALLOC),
    (
        "a second declarator freed",
        AGG,
        DECLARED,
        "\tstruct iwl_mld_baid_data *ba_data, *spare = NULL;\n\tkfree(spare);",
        KMALLOC,
    ),
    (
        "freed as it leaves its scope",
        AGG,
        DECLARED,
        "\tstruct iwl_mld_baid_data *ba_data __free(kfree) = NULL;",
        KMALLOC,
    ),
    (
        "the table's entry freed",
        AGG,
        READ,
        READ + "\n\tkfree(rcu_access_pointer(mld->fw_id_to_ba[baid]));",
        KMALLOC,
    ),
    ("another type's data freed", MLD, OTHER, OTHER + "\n\tkfree(data);", None),
]


def answer(bash: str, check: str, files: list[str], root: Path) -> tuple[int, str]:
    """Run the check over the files under a folder, giving its status and its words."""
    # S603: bash and the check are the build's own, from Nix, and no shell reads the names.
    run = subprocess.run(  # noqa: S603
        [bash, check, *files], cwd=root, capture_output=True, text=True, check=False
    )
    return run.returncode, run.stdout + run.stderr


def wrong(bash: str, check: str, files: list[str]) -> list[str]:
    """Give a line for each case the check answers wrongly."""
    lines = []
    for name, ending, before, after, expected in CASES:
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            for file in files:
                (root / file).parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(file, root / file)
                (root / file).chmod(0o644)
            edited = root / next(file for file in files if file.endswith(ending))
            text = edited.read_text()
            if before not in text:
                lines.append(
                    f"{name}: its edit no longer changes {ending}; move it with the driver."
                )
                continue
            edited.write_text(text.replace(before, after, 1))
            status, said = answer(bash, check, files, root)
        if expected is None and status != 0:
            lines.append(f"{name}: should pass, and failed: {said.strip()}")
        elif expected is not None and (status == 0 or expected not in said):
            lines.append(f"{name}: should fail saying {expected!r}, and said: {said.strip()}")
    return lines


def main(argv: list[str]) -> None:
    """Fail, naming each case, unless the check answers every case rightly."""
    bash, check, *files = argv
    lines = wrong(bash, check, files)
    if lines:
        sys.exit("The reorder-buffer check answers these edits wrongly:\n" + "\n".join(lines))


if __name__ == "__main__":
    main(sys.argv[1:])
