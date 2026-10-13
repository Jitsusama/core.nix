# zathura, for reading a document: a PDF, an EPUB, an XPS, a DjVu scan or
# PostScript. It's light, quick and kept to the pages: no bars at rest, the
# command line only while one is typed, so a document opened to glance at
# shows nothing but itself.
#
# Every colour it draws is from the theme: the desk behind the pages is the
# deepest colour, and its bar, command line, contents and notices are drawn
# as the rest of the desktop's are, in the proportional face. The pages
# themselves are recoloured into the theme, paper to the background and ink
# to the text, keeping each colour's hue so a chart still reads, and leaving
# photographs as they are; Ctrl+R shows a page as printed and back.
#
# Search hits are washed in a light neutral and the current one in the
# accent, the screen's one focal point, so the words stay legible under
# both. Text selected goes to the
# clipboard, where a paste looks for it, rather than only the primary
# selection. zathura remembers where each document was left and opens it
# there again.
#
# Its keys are its own, Vim's: j and k scroll, J and K turn pages, / and ?
# search, Tab opens the contents, Ctrl+N shows the bar, q quits.
#
# Comic book archives are left out: that plugin also claims folders and
# zip, tar and 7z archives, which would open in zathura.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.theme)
    roles
    font
    type
    space
    ;
  rgba = import ../theme/rgba.nix { inherit lib; };
  plugins = pkgs.zathuraPkgs;
in
{
  imports = [ ../theme/home.nix ];

  programs.zathura = {
    enable = true;
    package = pkgs.zathura.override {
      plugins = [
        plugins.zathura_pdf_mupdf
        plugins.zathura_djvu
        plugins.zathura_ps
      ];
    };
    options = {
      font = "${font.sans.family} ${type.sans.body}";
      guioptions = "";

      default-bg = roles.deepest;
      default-fg = roles.text;
      page-v-padding = space.s;
      page-h-padding = space.s;
      render-loading-bg = roles.background;
      render-loading-fg = roles.muted;

      recolor = true;
      recolor-keephue = true;
      recolor-reverse-video = true;
      recolor-lightcolor = roles.background;
      recolor-darkcolor = roles.text;

      statusbar-bg = roles.sunken;
      statusbar-fg = roles.text;
      statusbar-h-padding = space.m;
      statusbar-v-padding = space.s;
      statusbar-home-tilde = true;
      inputbar-bg = roles.sunken;
      inputbar-fg = roles.strong;

      completion-bg = roles.sunken;
      completion-fg = roles.text;
      completion-group-bg = roles.deepest;
      completion-group-fg = roles.muted;
      completion-highlight-bg = roles.selection;
      completion-highlight-fg = roles.strong;

      index-bg = roles.background;
      index-fg = roles.text;
      index-active-bg = roles.selection;
      index-active-fg = roles.strong;

      notification-bg = roles.raised;
      notification-fg = roles.text;
      notification-warning-bg = roles.warning;
      notification-warning-fg = roles.on_accent;
      notification-error-bg = roles.alert;
      notification-error-fg = roles.on_accent;

      # Washed over the words, which are lighter than the page: a wash as dark
      # as the selection would dull them, so a lighter one lifts the page to
      # about the selection's colour and leaves the words legible.
      highlight-color = rgba roles.muted "0.35";
      highlight-active-color = rgba roles.accent "0.4";
      highlight-fg = roles.strong;

      signature-success-color = rgba roles.success "0.9";
      signature-warning-color = rgba roles.warning "0.9";
      signature-error-color = rgba roles.alert "0.9";

      selection-clipboard = "clipboard";
      window-title-basename = true;
    };
  };
}
