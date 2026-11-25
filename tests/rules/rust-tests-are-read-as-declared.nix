# The check that rust_code.py, beside this, tells a crate's own code from what
# only its tests read as rustc finds modules, not by their names, since
# commands-agree reads a command's source through it. It plants a crate with
# each way a module comes to be read by tests alone, and two real files whose
# names say tests, and holds the reader to the list of those tests read.
{
  lib,
  python3,
  runCommandLocal,
  writeText,
}:
let
  # The crate, each file under src as it's named.
  crate = {
    "main.rs" = ''
      mod deep;
      mod real_tests;
      #[cfg(test)]
      mod checks;
      #[cfg(all(test, target_os = "linux"))]
      #[path = "elsewhere/linux.rs"]
      mod linux;
      #[cfg(test)]
      mod inline {
          mod nested;
      }
    '';
    # Real code, though its name says tests.
    "real_tests.rs" = "";
    # A module a test module declares is read by tests alone too.
    "checks.rs" = "mod helper;";
    "checks/helper.rs" = "";
    "elsewhere/linux.rs" = "";
    "inline/nested.rs" = "";
    # A mod.rs finds its modules beside it.
    "deep/mod.rs" = ''
      #[cfg(test)]
      mod golden;
    '';
    "deep/golden.rs" = "";
    # A binary's root, real code though its name says tests, finds its
    # modules beside it.
    "bin/tool_tests.rs" = ''
      #[cfg(test)]
      mod t;
    '';
    "bin/t.rs" = "";
  };
  read = writeText "read-by-tests-alone" ''
    bin/t.rs
    checks.rs
    checks/helper.rs
    deep/golden.rs
    elsewhere/linux.rs
    inline/nested.rs
  '';
  planted = lib.concatStrings (
    lib.mapAttrsToList (file: text: ''
      mkdir -p "src/$(dirname ${file})"
      cp ${writeText "planted" text} src/${file}
    '') crate
  );
  rule = lib.fileset.toSource {
    root = ./.;
    fileset = ./rust_code.py;
  };
in
runCommandLocal "rust-tests-are-read-as-declared" { } ''
  ${planted}
  ${lib.getExe python3} ${rule}/rust_code.py src > read
  diff -u ${read} read
  touch $out
''
