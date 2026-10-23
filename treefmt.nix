# How `nix fmt` formats and lints this repository, and what the formatting
# check holds it to. Lines stop at 100 characters, the practical limit of the
# code style. Every language's linter runs at its strictest, test code's
# included; ruff.toml and mypy.ini say how for Python.
{ pkgs, ... }:
{
  projectRootFile = "flake.nix";

  # mypy reads every script at once, from a Python carrying the libraries
  # they import, so their types are checked too.
  settings.formatter.mypy = {
    command = "${
      pkgs.python3.withPackages (python: [
        python.dbus-fast
        python.fido2
        python.mypy
      ])
    }/bin/mypy";
    includes = [ "*.py" ];
  };

  programs = {
    actionlint.enable = true;
    deadnix.enable = true;
    nixfmt.enable = true;
    ruff-check.enable = true;
    ruff-format.enable = true;
    rustfmt = {
      enable = true;
      edition = "2024";
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
}
