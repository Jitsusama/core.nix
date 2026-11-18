# The checks of treefmt, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  onLinux,
  pkgs,
  self,
  treefmt-nix,
  ...
}:
onLinux {
  # jitsusama.mypy.paths takes a folder or a module or stub named alone, and
  # refuses a place that isn't there, one that's none of those and one given
  # as a string, as Nix reads it, so the sandboxed mypy never runs without
  # what it was told to read. Each is read as the wrapper nix fmt runs is,
  # and tryEval sees the refusals; the same wrapper taking a folder and a
  # module is what shows each refusal is for the place alone.
  the-mypy-path-refuses-what-it-cant-read =
    let
      pathed =
        paths:
        builtins.tryEval
          (treefmt-nix.lib.evalModule pkgs {
            imports = [ self.treefmtModules.treefmt ];
            jitsusama.mypy.paths = paths;
          }).config.build.wrapper.drvPath;
      inherit (pkgs.lib) optional;
      wrong =
        optional (!(pathed [ ../../tests/lib ]).success) "refused a folder"
        ++ optional (!(pathed [ ../../tests/lib/vm_helpers.py ]).success) "refused a module"
        ++ optional (!(pathed [ ../../tests/lib/driver_names.pyi ]).success) "refused a stub"
        ++ optional (pathed [ ./missing.py ]).success "took a module that isn't there"
        ++ optional (pathed [ ./mypy.sh ]).success "took a shell script"
        ++ optional (pathed [ (toString ../../tests/lib) ]).success "took a string";
      said = "jitsusama.mypy.paths ${pkgs.lib.concatStringsSep ", " wrong}";
    in
    pkgs.runCommandLocal "the-mypy-path-refuses-what-it-cant-read" { } (
      if wrong == [ ] then
        "touch $out"
      else
        ''
          echo ${pkgs.lib.escapeShellArg said}
          exit 1
        ''
    );

  # A folder named on jitsusama.mypy.paths reaches mypy with its stubs and
  # its Python alone: the wrapper nix fmt runs, given tests/lib, puts last on
  # its path a folder holding that folder's stub and module, not its README.
  the-mypy-path-keeps-a-folders-stubs =
    let
      inherit
        ((treefmt-nix.lib.evalModule pkgs {
          imports = [ self.treefmtModules.treefmt ];
          jitsusama.mypy.paths = [ ../../tests/lib ];
        }).config.settings.formatter.mypy
        )
        command
        ;
    in
    # Kept in Nix because what it reads is the wrapper Nix builds from the
    # option given above, named only as Nix interpolates it.
    pkgs.runCommandLocal "the-mypy-path-keeps-a-folders-stubs" { } ''
      path=$(sed -n 's/^MYPYPATH="\([^"]*\)".*/\1/p' ${command})
      named=''${path##*:}
      if [ -z "$path" ]; then
        echo "${command} sets no MYPYPATH this check can read" >&2
        exit 1
      fi
      if [ ! -f "$named/driver_names.pyi" ] || [ ! -f "$named/vm_helpers.py" ] \
        || [ -e "$named/README.md" ]; then
        echo "given tests/lib, mypy's path ends in $named, not its stub and Python alone" >&2
        exit 1
      fi
      touch $out
    '';

  # Two files no formatter reads, listed first in a repository of thousands,
  # fail the run inside a minute and are named, where released treefmt waits
  # on git for good (package.nix).
  a-file-no-linter-reads-fails-at-once =
    pkgs.runCommandLocal "a-file-no-linter-reads-fails-at-once"
      {
        nativeBuildInputs = [
          (pkgs.callPackage ./package.nix { })
          pkgs.git
        ];
      }
      ''
        export HOME=$TMPDIR
        cd "$(mktemp -d)"
        git init -q .
        mkdir a
        echo unread > a/first.unread
        echo unread > a/second.unread
        for i in $(seq 3000); do echo read > "file-$i.sh"; done
        cat > treefmt.toml <<'EOF'
        on-unmatched = "fatal"
        excludes = ["treefmt.toml"]
        [formatter.nothing]
        command = "true"
        includes = ["*.sh"]
        EOF
        status=0
        timeout -s KILL 60 treefmt --no-cache > said 2>&1 || status=$?
        cat said
        if [ "$status" -eq 137 ]; then
          echo "treefmt was still waiting after a minute rather than failing" >&2
          exit 1
        fi
        if [ "$status" -eq 0 ] || ! grep -q 'no formatter for path: a/' said; then
          echo "treefmt exited $status without naming the unread file" >&2
          exit 1
        fi
        touch $out
      '';
}
