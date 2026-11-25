# The check that rust_code.py, beside this, tells a crate's own code from what
# only its tests read as rustc finds modules, not by their names, since
# commands-agree reads a command's source through it. It plants a crate with
# each way a module comes to be read by tests alone, and real files whose
# names or doc lines say tests, and holds the reader to the list of those
# tests read, and to what it leaves of the crate's own code.
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
      /// Real code, unlike what #[cfg(test)] rules in.
      mod documented;
      #[cfg(test)]
      mod checks;
      #[cfg(test)]
      mod fixtures;
      #[cfg(all(test, target_os = "linux"))]
      #[path = "elsewhere/linux.rs"]
      mod linux;
      #[ cfg ( test ) ]
      mod spaced;
      #[cfg(all(feature = "slow", test))]
      mod slow;
      #[cfg(any(test, feature = "probe"))]
      mod either;
      #[cfg(not(test))]
      mod live;
      #[cfg(all(test, target_os = "redox"))]
      mod redox;

      fn own() {}

      #[cfg(test)]
      mod inline {
          mod nested;
      }

      fn after() {}

      #[cfg(test)]
      mod empty {}

      fn last() {}
    '';
    # Real code, though its name or doc line says tests, or it's read
    # outside them too, or only outside them.
    "documented.rs" = "";
    "either.rs" = "";
    "live.rs" = "";
    # Read by tests alone, however their cfg is written.
    "spaced.rs" = "";
    "slow.rs" = "";
    # Real code, though its name says tests, with a test module inline in a
    # file no root, whose modules are read from the file's folder.
    "real_tests.rs" = ''
      fn real() {}

      mod outer {
          #[cfg(test)]
          mod tests {
              fn t() {}
          }

          fn kept() {}
      }

      #[cfg(test)]
      mod inner {
          mod deeper;
      }
    '';
    "real_tests/inner/deeper.rs" = "";
    # A module a test module declares is read by tests alone too, and one it
    # gives a path is read from beside it, not from its folder.
    "checks.rs" = ''
      mod helper;
      #[path = "shared.rs"]
      mod shared;
    '';
    "checks/helper.rs" = "";
    "shared.rs" = "";
    # A module with no file of its name is read from its folder's mod.rs.
    "fixtures/mod.rs" = "";
    # A file a path names finds its modules beside it, not in its folder,
    # which holds a file of the name nothing reads.
    "elsewhere/linux.rs" = ''
      mod probe;
    '';
    "elsewhere/probe.rs" = "";
    "elsewhere/linux/probe.rs" = "";
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
    elsewhere/probe.rs
    fixtures/mod.rs
    inline/nested.rs
    real_tests/inner/deeper.rs
    shared.rs
    slow.rs
    spaced.rs
  '';
  # The crate's own code, each file named as `--own` names it and followed
  # by what's left of it once its inline test modules are cut.
  own = writeText "own-code" ''
    == bin/tool_tests.rs
    #[cfg(test)]
    mod t;
    == deep/mod.rs
    #[cfg(test)]
    mod golden;
    == documented.rs
    == either.rs
    == elsewhere/linux/probe.rs
    == live.rs
    == main.rs
    mod deep;
    mod real_tests;
    /// Real code, unlike what #[cfg(test)] rules in.
    mod documented;
    #[cfg(test)]
    mod checks;
    #[cfg(test)]
    mod fixtures;
    #[cfg(all(test, target_os = "linux"))]
    #[path = "elsewhere/linux.rs"]
    mod linux;
    #[ cfg ( test ) ]
    mod spaced;
    #[cfg(all(feature = "slow", test))]
    mod slow;
    #[cfg(any(test, feature = "probe"))]
    mod either;
    #[cfg(not(test))]
    mod live;
    #[cfg(all(test, target_os = "redox"))]
    mod redox;

    fn own() {}



    fn after() {}



    fn last() {}
    == real_tests.rs
    fn real() {}

    mod outer {


        fn kept() {}
    }


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
  ${lib.getExe python3} ${rule}/rust_code.py --own src > own
  diff -u ${own} own
  touch $out
''
