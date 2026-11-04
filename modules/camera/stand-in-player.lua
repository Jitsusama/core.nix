-- A stand-in for the player's own scripts (modules/mpv), as vi-keys.lua
-- binds Escape in every mpv: it takes Escape once the camera plays, after
-- camera.lua has bound it, so a viewfinder that loads it always loses the
-- key, where vi-keys.lua loses or wins it by which script loaded last. It
-- says so to the check (keeps-its-keys.py) once it has, and Escape here
-- only says it was taken.
mp.register_event('file-loaded', function()
  mp.add_forced_key_binding('ESC', 'player-escape', function()
    mp.commandv('script-message', 'player-took-escape')
  end)
  mp.commandv('script-message', 'player-bound')
end)
