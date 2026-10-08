# How `nix fmt` formats and lints this repository, and what the formatting
# check holds it to. Lines stop at 100 characters, the practical limit of the
# code style.
{
  projectRootFile = "flake.nix";

  programs = {
    actionlint.enable = true;
    deadnix.enable = true;
    nixfmt.enable = true;
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
