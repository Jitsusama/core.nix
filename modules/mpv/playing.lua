-- mpv says the file it plays now, so mpv's Recent in the launcher leads
-- with it and goes to its window. Each mpv keeps one file in the folder
-- the desktop makes for it at sign-in, in the state home, named by mpv's process id and
-- holding that id and the file's whole path, a tab between, the line the
-- launcher reads (mpv's `opened` in home.nix). It's written as each file
-- loads, to a hidden name and then renamed over, so it's never read half
-- written, and removed as the file ends and as mpv quits. A stream isn't
-- a file, so none is kept while one plays. Where there's no folder,
-- nothing is kept. Starting, it clears what an mpv that was killed left.
local utils = require('mp.utils')

local home = os.getenv('HOME')
local state = os.getenv('XDG_STATE_HOME') or (home and home .. '/.local/state')
if state == nil then
  return
end
local folder = state .. '/mpv/playing'
local pid = tostring(utils.getpid())
local kept = folder .. '/' .. pid
local writing = folder .. '/.' .. pid

for _, name in ipairs(utils.readdir(folder, 'files') or {}) do
  local other = name:match('^%.?(%d+)$')
  if other ~= nil and other ~= pid and utils.file_info('/proc/' .. other) == nil then
    os.remove(folder .. '/' .. name)
  end
end

local function forget()
  os.remove(kept)
end

mp.register_event('file-loaded', function()
  local path = mp.get_property('path')
  if path == nil or path:find('://', 1, true) ~= nil then
    forget()
    return
  end
  local whole = utils.join_path(mp.get_property('working-directory') or '', path)
  local file = io.open(writing, 'w')
  if file == nil then
    return
  end
  file:write(pid, '\t', whole, '\n')
  file:close()
  os.rename(writing, kept)
end)

mp.register_event('end-file', forget)
mp.register_event('shutdown', forget)
