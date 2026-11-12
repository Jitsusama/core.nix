# The checks of lid-hinge, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  onLinux,
  pkgs,
  ...
}:
onLinux {
  # The program believes a shut switch once the hinge came down past
  # nearly shut, or once nobody has used the laptop's keys or touchpad a
  # while and a light sensor lit at the switch reads dark, or none was
  # lit; it doubts it otherwise, and again once those are used. It reads
  # the hinge's angle from its buffer and the presses from the interrupts.
  the-lid-shuts-when-the-hinge-agrees =
    pkgs.runCommandCC "the-lid-shuts-when-the-hinge-agrees" { nativeBuildInputs = [ pkgs.rustc ]; }
      ''
        # Its tests sit beside it, where its `#[path]` looks for them.
        cp ${./lid-hinge.rs} lid-hinge.rs
        cp ${./lid-hinge_tests.rs} lid-hinge_tests.rs
        rustc --edition 2024 --test -D warnings -o tests lid-hinge.rs
        ./tests
        touch $out
      '';
}
