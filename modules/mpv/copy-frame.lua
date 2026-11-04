-- yy copies the frame in view to the clipboard as a picture, as swayimg's
-- yy copies the picture in view (vi-keys.lua binds it): mpv's own
-- clipboard takes only text, so the frame is saved as a PNG, as shown,
-- subtitles and all, in the runtime folder and handed to wl-copy, which
-- home.nix names in script-opts. s still saves it to a file.
local options = { wl_copy = 'wl-copy' }
require('mp.options').read_options(options, 'copy-frame')

mp.add_key_binding(nil, 'copy', function()
  local folder = os.getenv('XDG_RUNTIME_DIR') or os.getenv('TMPDIR') or '/tmp'
  local file = folder .. '/mpv-copied-frame.png'
  -- A command that answers nothing answers nil when it works, so only an
  -- error says it didn't.
  local _, failed = mp.command_native({ name = 'screenshot-to-file', filename = file })
  if failed then
    mp.osd_message("Couldn't copy the frame")
    return
  end
  -- The PNG goes to wl-copy as its input from the file, since mpv takes
  -- only text as what a subprocess reads.
  local copied = mp.command_native({
    name = 'subprocess',
    args = { 'sh', '-c', 'exec "$0" --type image/png < "$1"', options.wl_copy, file },
    playback_only = false,
  })
  os.remove(file)
  if copied.status ~= 0 then
    mp.osd_message("Couldn't copy the frame")
    return
  end
  mp.osd_message('Copied')
end)
