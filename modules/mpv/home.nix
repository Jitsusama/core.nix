# mpv plays video, set for the picture first and then for smooth motion:
#
# - It draws through libplacebo (gpu-next), on Vulkan where there is one,
#   and decodes on the GPU wherever that's known to be safe, so a 4K file
#   plays without the CPU.
# - The high-quality profile scales with a sharpened EWA Lanczos, and
#   banding in flat colour is smoothed out, as web video is full of it.
# - Frames are timed to the display rather than to the sound, with the
#   sound resampled by a fraction of a percent to keep in step, so no frame
#   is shown twice or dropped to catch up. A frame rate that doesn't divide
#   the display's is blended between frames rather than juddered. The sound
#   stays in sync over Bluetooth too: PipeWire tells mpv how long the
#   headphones hold it, and mpv shows each frame that much later.
# - A file stays on its last frame when it ends, so a short recording can
#   be looked at rather than vanishing. Sound alone opens no window.
#   Quitting remembers where a file was, and opening it again goes back.
#
# What mpv draws over the picture is in the theme: its messages and
# subtitles in the proportional face, and the controller's bar in the
# deepest colour with the text's colour on it. The keys are mpv's own: Space
# pauses, the arrows seek, f fills the screen and q quits. It reports what
# it plays over MPRIS, so the media keys and the shell's player reach it.
# umpv, the package's second entry, which queues files into one window,
# stays out of the launcher, where it would sit beside mpv as a second mpv.
{ config, pkgs, ... }:
let
  inherit (config.jitsusama.theme) roles font;
in
{
  imports = [
    ../theme/home.nix
    ../unlisted/home.nix
  ];

  jitsusama.unlisted = [ "umpv" ];

  programs.mpv = {
    enable = true;
    scripts = [ pkgs.mpvScripts.mpris ];
    config = {
      profile = "high-quality";
      vo = "gpu-next";
      hwdec = "auto-safe";
      deband = true;

      video-sync = "display-resample";
      interpolation = true;
      tscale = "oversample";
      ao = "pipewire";

      keep-open = true;
      save-position-on-quit = true;
      title = "\${media-title}";

      osd-font = font.sans.family;
      osd-font-size = 24;
      osd-color = roles.strong;
      osd-border-color = roles.shadow;
      osd-border-size = 2;
      sub-font = font.sans.family;
      sub-color = roles.strong;
      sub-border-color = roles.shadow;

      screenshot-directory = "${config.home.homeDirectory}/Pictures/Screenshots";
      screenshot-format = "png";
      screenshot-template = "%F %wH-%wM-%wS";
    };
    scriptOpts.osc = {
      background_color = roles.deepest;
      timecode_color = roles.text;
      title_color = roles.strong;
      time_pos_color = roles.text;
      buttons_color = roles.text;
      small_buttonsL_color = roles.text;
      small_buttonsR_color = roles.text;
      top_buttons_color = roles.text;
      held_element_color = roles.muted;
      time_pos_outline_color = roles.shadow;
    };
  };
}
