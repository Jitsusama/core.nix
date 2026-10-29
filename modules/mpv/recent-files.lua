-- mpv says each file it plays to the desktop's list of files opened lately
-- (open-with's recent-files), so mpv's Recent in the launcher lists it, as
-- Neovim says each file it edits. It says what mpv was given as it was
-- given: recent-files passes over a stream or a web address, which isn't a
-- file. Where the desktop has no recent-files, nothing is said. A script
-- runs on a thread of its own, so playing never waits on the call.
mp.register_event('file-loaded', function()
  local path = mp.get_property('path')
  if path == nil then
    return
  end
  mp.command_native({
    name = 'subprocess',
    args = { 'recent-files', 'add', 'mpv', path },
    playback_only = false,
    capture_stderr = true,
  })
end)
