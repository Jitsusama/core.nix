# The checks of inlyne, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  graphical,
  lib,
  onLinux,
  pkgs,
  ...
}:
onLinux {
  # inlyne draws a sheet with the account's settings, on a screen of
  # nobody's, with the theme's fonts and a software GPU: the page on the
  # theme's background, its heading in the strong text, and none of
  # inlyne's own colours. The keys page its settings name, which ? opens,
  # draws in the theme too. Markdown files open in inlyne.
  inlyne-reads-in-the-theme =
    let
      home = account graphical;
      inherit (home.jitsusama.theme) colours;
      opens = graphical.config.xdg.mime.defaultApplications."text/markdown" or "";
      inlyne = lib.findFirst (package: lib.getName package == "inlyne") null home.home.packages;
      fonts = pkgs.makeFontsConf { fontDirectories = graphical.config.fonts.packages; };
      # The colour as ImageMagick names a pixel's.
      srgb =
        colour:
        let
          hex = lib.removePrefix "#" colour;
          channel = index: toString (lib.fromHexString (builtins.substring (index * 2) 2 hex));
        in
        "srgb(${channel 0},${channel 1},${channel 2})";
    in
    assert opens == "inlyne.desktop";
    pkgs.runCommandLocal "inlyne-reads-in-the-theme"
      {
        nativeBuildInputs = [
          inlyne
          pkgs.bash
          pkgs.cage
          pkgs.grim
          pkgs.imagemagick
          pkgs.inotify-tools
        ];
      }
      ''
        export HOME=$PWD XDG_CONFIG_HOME=$PWD/config XDG_RUNTIME_DIR=$PWD/run
        export FONTCONFIG_FILE=${fonts}
        export WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1
        export LIBGL_ALWAYS_SOFTWARE=1
        export __EGL_VENDOR_LIBRARY_DIRS=${pkgs.mesa}/share/glvnd/egl_vendor.d
        export VK_ICD_FILENAMES=${pkgs.mesa}/share/vulkan/icd.d/lvp_icd.x86_64.json
        mkdir -p config/inlyne run attachments
        chmod 700 run
        # The account's settings, with a line logged for every frame drawn,
        # which the script waits on.
        cat ${home.xdg.configFile."inlyne/inlyne.toml".source} > config/inlyne/inlyne.toml
        printf '\n[debug]\nmetrics = "log"\n' >> config/inlyne/inlyne.toml
        cp ${pkgs.nixos-icons}/share/icons/hicolor/64x64/apps/nix-snowflake.png attachments/chart.png
        cat > sheet.md <<'EOF'
        # The Look and the Apps

        Every app wears the theme, and [the plan](plans/plan.md) says how.

        - [x] a ticked box
        - [ ] one still open

        ```rust
        fn main() {}
        ```

        ![the chart](attachments/chart.png)
        EOF
        keys=$(sed -n 's/^keys-page = "\(.*\)"$/\1/p' config/inlyne/inlyne.toml)
        [ -s "$keys" ] || { echo "inlyne's settings name no keys page"; exit 1; }
        for read in sheet.md:page.png "$keys:keys.png"; do
          timeout 60 cage -- bash ${./tests/inlyne-reads-in-the-theme.sh} "''${read%%:*}" "''${read##*:}" || true
          if ! [ -e "''${read##*:}.read" ]; then cat said; exit 1; fi
          # inlyne goes on with its own settings when it can't read the
          # account's, saying so only as a warning.
          if grep -a 'Failed reading config file' said; then exit 1; fi
        done

        # The colour of one pixel, and how many pixels are one colour.
        at() { magick "$1" -format "%[pixel:p{$2}]" info:; }
        many() {
          magick "$1" -fill black +opaque "$2" -fill white -opaque "$2" \
            -format '%[fx:round(mean*w*h)]' info:
        }
        fail() { echo "$1"; exit 1; }
        for page in page.png keys.png; do
          [ "$(at $page 2,2)" = "${srgb colours.background}" ] \
            || fail "inlyne's $page is $(at $page 2,2), not the theme's background"
          [ "$(many $page '${srgb colours.strong}')" -gt 100 ] \
            || fail "inlyne's heading in $page isn't drawn in the theme's strong text"
          # inlyne's own dark text and background, which the theme replaces.
          for own in 'srgb(157,172,187)' 'srgb(26,29,34)'; do
            [ "$(many $page "$own")" -eq 0 ] || fail "inlyne drew its own colour $own in $page"
          done
        done
        # The photographs stay, to look at.
        mkdir $out
        cp page.png keys.png $out
      '';
}
