-- y copies the frame in view to the clipboard as a picture, as swayimg's y
-- copies the picture in view: mpv's own clipboard takes only text, so the
-- frame is saved as a PNG, as shown, subtitles and all, in the runtime
-- folder and handed to wl-copy, which home.nix names in script-opts. s
-- still saves it to a file.
local options = { wl_copy = 'wl-copy' }
require('mp.options').read_options(options, 'copy-frame')

mp.add_key_binding(nil, 'copy', function()
  local folder = os.getenv('XDG_RUNTIME_DIR') or os.getenv('TMPDIR') or '/tmp'
  local file = folder .. '/mpv-copied-frame.png'
  local saved = mp.command_native({ name = 'screenshot-to-file', filename = file })
  local picture = saved ~= nil and io.open(file, 'rb')
  if not picture then
    mp.osd_message("Couldn't copy the frame")
    return
  end
  local png = picture:read('*a')
  picture:close()
  os.remove(file)
  local copied = mp.command_native({
    name = 'subprocess',
    args = { options.wl_copy, '--type', 'image/png' },
    stdin_data = png,
    playback_only = false,
  })
  if copied.status ~= 0 then
    mp.osd_message("Couldn't copy the frame")
    return
  end
  mp.osd_message('Copied')
end)
