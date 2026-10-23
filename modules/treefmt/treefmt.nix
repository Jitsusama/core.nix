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
    # they import, so their types are checked too.
    settings.formatter.mypy = {
      command = "${
        pkgs.python3.withPackages (python: [ python.mypy ] ++ config.jitsusama.mypy.packages python)
      }/bin/mypy";
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
