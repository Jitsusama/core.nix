# How `nix fmt` formats and lints this repository, and what the formatting
# check holds it to. Lines stop at 100 characters, the practical limit of the
# code style. Every language's linter runs at its strictest, test code's
# included; ruff.toml and mypy.ini say how for Python, .luacheckrc for Lua.
{ pkgs, ... }:
let
  driver = pkgs.python3Packages.callPackage "${pkgs.path}/nixos/lib/test-driver" { };
  site = pkgs.python3.sitePackages;
  driverTypes = pkgs.python3Packages.toPythonModule (
    pkgs.runCommandLocal "test-driver-types" { } ''
      mkdir -p $out/${site}
      ln -s ${driver}/${site}/test_driver $out/${site}/
    ''
  );
in
{
  projectRootFile = "flake.nix";

  # mypy reads every script at once, from a Python carrying the libraries
  # they import, so their types are checked too. A VM test's script names the
  # test driver's types, which mypy reads from the driver as it's built; it's
  # a program rather than a library, so it's given as one.
  settings.formatter.mypy = {
    command = "${
      pkgs.python3.withPackages (python: [
        python.dbus-fast
        python.fido2
        python.mypy
        driverTypes
      ])
    }/bin/mypy";
    includes = [ "*.py" ];
  };

  # luacheck reads each Lua file as the program that runs it does, as
  # .luacheckrc says.
  settings.formatter.luacheck = {
    command = "${pkgs.lua54Packages.luacheck}/bin/luacheck";
    options = [
      "--codes"
      "--no-color"
      "--quiet"
    ];
    includes = [ "*.lua" ];
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
