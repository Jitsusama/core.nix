# The check that rust_code.py, beside this, tells a crate's own code from what
# only its tests read as rustc finds modules, not by their names, since
# commands-agree reads a command's source through it. It plants a crate with
# each way a module comes to be read by tests alone, and real files whose
# names or doc lines say tests, and holds the reader to the list of those
# tests read, and to what it leaves of the crate's own code. Then it sees the
# reader refuse crates it would otherwise misread, each for its own reason.
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
      #[cfg(unix)]
      #[cfg(test)]
      mod second;
      #[cfg(unix)]
      #[cfg(not(test))]
      mod neither;
      #[cfg(any(test, all(feature = "x", test)))]
      mod any_test;
      #[cfg(all(
          feature = "slow",
          test
      ))]
      mod wrapped;
      #[allow(dead_code)] #[cfg(test)]
      mod shared_line;
      /// Read from its own file: #[path = "documented.rs"]
      #[cfg(test)]
      mod doc_path;

      fn own() {}

      #[cfg(test)]
      mod inline {
          mod nested;
      }

      fn after() {}

      #[cfg(test)]
      mod empty {}

      fn last() {}

      // A test module's braces, strings and comments, which hold braces
      // that close nothing, and a test module in it, cut with it.
      #[cfg(test)]
      mod literals {
          const RAW: &str = r##"
      "# }"##;
          const BRACE: char = '}';
          const BYTE: u8 = b'}';
          const BYTES: &[u8] = br#"}"#;
          // }
          /* /* */ } */
          fn l<'a>(x: &'a ()) -> char { let _: &'a () = x; '{' }
          fn t<'a>(x: &'a str) -> &'a str {
              let _ = "}\"}";
              x
          }
          #[cfg(test)]
          mod deeper {
              fn d() {}
          }
          fn after_deeper() {}
      }

      fn tail() {}
    '';
    # Read by tests alone, though a cfg beside holds outside them, or the
    # cfg runs over lines or shares one, or holds only under test by `any`.
    "second.rs" = "";
    "any_test.rs" = "";
    "wrapped.rs" = "";
    "shared_line.rs" = "";
    # Read by tests alone, though a doc line above it reads as a path.
    "doc_path.rs" = "";
    # Real code, under two cfgs neither of which is test's.
    "neither.rs" = "";
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

      // Not a module, though its text reads as one.
      const FAKE: &str = "
      #[cfg(test)]
      mod fake {
      ";
      // Nor a path, which would have checks.rs find its modules beside it.
      const FAKE_PATH: &str = r#"#[path = "checks.rs"]"#;

      fn still_real() {}
    '';
    "real_tests/inner/deeper.rs" = "";
    # A module a test module declares is read by tests alone too, and one it
    # gives a path is read from beside it, not from its folder.
    "checks.rs" = ''
      mod helper;
      #[path = "shared.rs"]
      mod shared;
      /*
      mod ghost;
      */
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
    any_test.rs
    bin/t.rs
    checks.rs
    checks/helper.rs
    deep/golden.rs
    doc_path.rs
    elsewhere/linux.rs
    elsewhere/probe.rs
    fixtures/mod.rs
    inline/nested.rs
    real_tests/inner/deeper.rs
    second.rs
    shared.rs
    shared_line.rs
    slow.rs
    spaced.rs
    wrapped.rs
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
    #[cfg(unix)]
    #[cfg(test)]
    mod second;
    #[cfg(unix)]
    #[cfg(not(test))]
    mod neither;
    #[cfg(any(test, all(feature = "x", test)))]
    mod any_test;
    #[cfg(all(
        feature = "slow",
        test
    ))]
    mod wrapped;
    #[allow(dead_code)] #[cfg(test)]
    mod shared_line;
    /// Read from its own file: #[path = "documented.rs"]
    #[cfg(test)]
    mod doc_path;

    fn own() {}



    fn after() {}



    fn last() {}

    // A test module's braces, strings and comments, which hold braces
    // that close nothing, and a test module in it, cut with it.


    fn tail() {}
    == neither.rs
    == real_tests.rs
    fn real() {}

    mod outer {


        fn kept() {}
    }



    // Not a module, though its text reads as one.
    const FAKE: &str = "
    #[cfg(test)]
    mod fake {
    ";
    // Nor a path, which would have checks.rs find its modules beside it.
    const FAKE_PATH: &str = r#"#[path = "checks.rs"]"#;

    fn still_real() {}
  '';
  # Crates the reader refuses, each a main.rs, and what its refusal says.
  refusals = {
    # A module tests always build, whose file the reader looked for in the
    # wrong place, rather than taken as real code.
    missing = {
      main = ''
        #[cfg(test)]
        mod gone;
      '';
      says = "mod gone is built under test with no file";
    };
    unclosed = {
      main = ''
        #[cfg(test)]
        mod tests {
            fn t() {}
      '';
      says = "a test module's braces never close";
    };
    unread = {
      main = ''
        #[cfg(all(feature = "x", test)]
        mod odd;
      '';
      # Said of its file, and as written, its string unmasked.
      says = ''main.rs: a cfg the reader can't read: cfg(all(feature = "x", test)'';
    };
  };
  refused = lib.concatStrings (
    lib.mapAttrsToList (name: refusal: ''
      mkdir -p refused/${name}/src
      cp ${writeText "refused" refusal.main} refused/${name}/src/main.rs
      if ${lib.getExe python3} ${rule}/rust_code.py --own refused/${name}/src > /dev/null 2> said; then
        echo "the reader took the ${name} crate" >&2
        exit 1
      fi
      grep -qF ${lib.escapeShellArg refusal.says} said || { cat said >&2; exit 1; }
    '') refusals
  );
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
  ${refused}
  touch $out
''
