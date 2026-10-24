# The formatters and linters core holds its code to, at their strictest, for
# any repository built on it to hold its own code to the same: a repository's
# treefmt module imports this and adds only what it alone has. Each linter
# reads core's own settings, given by their place in the store, so a change
# to them reaches every repository at once rather than drifting in copies.
# Lines stop at 100 characters, the practical limit of the code style.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  # A VM test's script names the NixOS test driver's types, which mypy reads
  # from the driver as it's built; it's a program rather than a library, so
  # it's given as one.
  driver = pkgs.python3Packages.callPackage "${pkgs.path}/nixos/lib/test-driver" { };
  site = pkgs.python3.sitePackages;
  driverTypes = pkgs.python3Packages.toPythonModule (
    pkgs.runCommandLocal "test-driver-types" { } ''
      mkdir -p $out/${site}
      ln -s ${driver}/${site}/test_driver $out/${site}/
    ''
  );
  # The names the driver gives a script as globals, which it imports from
  # driver_names for type checkers alone, found on mypy's path wherever the
  # script lives.
  driverNames = lib.fileset.toSource {
    root = ../../tests;
    fileset = ../../tests/driver_names.pyi;
  };
  # What VM tests' scripts share, which a script imports as vm_helpers and
  # vm_pytest in any repository that takes these linters, as its driver does
  # (tests/lib).
  vmHelpers = lib.fileset.toSource {
    root = ../../tests/lib;
    fileset = lib.fileset.unions [
      ../../tests/lib/vm_helpers.py
      ../../tests/lib/vm_pytest.py
    ];
  };
  python = pkgs.python3.withPackages (
    python:
    [
      python.mypy
      driverTypes
      # Tests written for pytest, VM tests' among them, and vm_pytest itself.
      python.pytest
    ]
    ++ config.jitsusama.mypy.packages python
  );
  mypy = pkgs.writeShellApplication {
    name = "mypy";
    text = ''MYPYPATH=${driverNames}:${vmHelpers} exec ${python}/bin/mypy "$@"'';
  };
in
{
  options.jitsusama.mypy.packages = lib.mkOption {
    type = lib.types.functionTo (lib.types.listOf lib.types.package);
    default = _: [ ];
    description = ''
      The libraries a repository's Python imports, from the Python package
      set given, so that mypy reads their types too.
    '';
  };

  config = {
    projectRootFile = "flake.nix";

    # mypy reads every script at once, from a Python carrying the libraries
    # they import and the test driver's types, so their types are checked
    # too.
    settings.formatter.mypy = {
      command = lib.getExe mypy;
      options = [
        "--config-file"
        "${../../mypy.ini}"
      ];
      includes = [ "*.py" ];
    };

    # rumdl reads every Markdown file and fixes nothing.
    settings.formatter.rumdl = {
      command = lib.getExe pkgs.rumdl;
      options = [
        "check"
        "--config"
        "${../../.rumdl.toml}"
        "--config"
        "cache = false"
      ];
      includes = [ "*.md" ];
    };

    # zsh reads each of its files for what it can't parse, one at a time, as
    # it takes them; nothing lints zsh further.
    settings.formatter.zsh-parses = {
      command = lib.getExe pkgs.bash;
      options = [
        "-c"
        ''for file; do ${lib.getExe pkgs.zsh} --no-exec "$file" || exit; done''
        "zsh-parses"
      ];
      includes = [ "*.zsh" ];
    };

    # luacheck reads each Lua file as the program that runs it does, which
    # each repository says in a .luacheckrc of its own, since only it knows
    # which program runs which file.
    settings.formatter.luacheck = {
      command = "${pkgs.lua54Packages.luacheck}/bin/luacheck";
      options = [
        "--codes"
        "--no-color"
        "--quiet"
      ];
      includes = [ "*.lua" ];
    };

    # ruff reads core's settings rather than looking for a repository's own,
    # given after the command treefmt-nix starts its options with.
    settings.formatter.ruff-check.options = lib.mkAfter [
      "--config"
      "${../../ruff.toml}"
    ];
    settings.formatter.ruff-format.options = lib.mkAfter [
      "--config"
      "${../../ruff.toml}"
    ];

    programs = {
      deadnix.enable = true;
      nixfmt.enable = true;
      ruff-check.enable = true;
      ruff-format.enable = true;
      rustfmt = {
        enable = true;
        edition = lib.mkDefault "2024";
      };
      shellcheck.enable = true;
      statix.enable = true;
      stylua = {
        enable = true;
        settings = {
          column_width = 100;
          indent_type = "Spaces";
          indent_width = 2;
          quote_style = "AutoPreferSingle";
        };
      };
    };
  };
}
