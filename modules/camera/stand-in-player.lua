-- A stand-in for the player's own scripts (modules/mpv), as vi-keys.lua
-- binds Z in every mpv to start its ZZ: it takes Z once the camera plays,
-- after camera.lua has bound it, so a viewfinder that loads it always loses
-- the key, where vi-keys.lua loses or wins it by which script loaded last.
-- It says so to the check (keeps-its-keys.py) once it has, and Z here only
-- says it was taken.
mp.register_event('file-loaded', function()
  mp.add_forced_key_binding('Z', 'player-z', function()
    mp.commandv('script-message', 'player-took-z')
  end)
  mp.commandv('script-message', 'player-bound')
end)
