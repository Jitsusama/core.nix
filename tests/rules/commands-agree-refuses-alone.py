"""Hold commands-agree to failing alone a crate the reader refuses.

rust-tests-are-read-as-declared.nix runs this beside the crates it plants:
it judges two commands, one whose crate the reader refuses and one it reads,
and the first must be said to be unreadable, by name and with the reader's
reason, while the second is still judged, so no refusal hides the rest.

usage: commands-agree-refuses-alone.py <rules folder> <empty folder>
"""

import importlib.util
import sys
from pathlib import Path

# The reader names files resolved, so the tree is too.
rules, tree = Path(sys.argv[1]), Path(sys.argv[2]).resolve()
sys.path.insert(0, str(rules))
spec = importlib.util.spec_from_file_location("commands_agree", rules / "commands-agree.py")
if spec is None or spec.loader is None:
    sys.exit(f"no commands-agree.py in {rules}")
agree = importlib.util.module_from_spec(spec)
spec.loader.exec_module(agree)

for name, main in (("refused", "#[cfg(test)]\nmod gone;\n"), ("read", "fn main() {}\n")):
    (tree / name / "src").mkdir(parents=True)
    (tree / name / "src/main.rs").write_text(main)
    (tree / name / "Cargo.toml").write_text(
        f'[package]\nname = "{name}"\n[dependencies]\nterminal-kit = {{}}\n'
    )
found = [agree.Command(name, tree / name / "Cargo.toml") for name in ("refused", "read")]
said, _ = agree.judged(found, {}, {}, tree)
reason = f"{tree / 'refused/src/main.rs'}: mod gone is built under test with no file at gone.rs"
expected = [
    f"refused (refused) can't be read apart from its tests: {reason} or gone/mod.rs.",
    "read (read) breaks helps",
]
if len(said) != len(expected) or [said[0], said[1][: len(expected[1])]] != expected:
    print("commands-agree didn't fail the refused crate alone, with its reason:", file=sys.stderr)
    print("\n".join(said), file=sys.stderr)
    sys.exit(1)
