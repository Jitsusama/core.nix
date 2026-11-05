-- Vim's keys in mpv, as Joel's Vim has them, each with one meaning: h and
-- l seek back and on 5 seconds, j on a minute and k back, as j goes on
-- through a file, and each arrow does what its letter does; 0 goes to the
-- start and $ to the end; gg plays the first in the list, G the last, and
-- a number before either that one; ]b and [b the next and the one before,
-- as Vim's go between buffers; yy copies the frame; ? and F1 open the
-- launcher's Shortcuts at the player's keys; ZZ closes it; and a number
-- before any of them does it that many times. q closes nothing, as in
-- all of Joel's programs, and says what does. mpv has no numbers before
-- keys, and does a key the moment it's pressed even when another binding
-- starts with it, so these are bound here, ahead of mpv's own. A key
-- that starts a pair waits 300 ms, his Neovim's timeoutlen, for its
-- second, and does what it does alone once none comes: [ and ] change
-- the speed, b smooths banding and Z delays the subtitles, as mpv has
-- them; y alone does nothing, as in Vim. Space leads nothing and pauses
-- at once, since waiting on a pause is felt every time, so mpv has no
-- Space w: a Space that paused and then closed on w would be half a
-- leader. Nothing shows what waits, as his Neovim has showcmd off.
local waiting = { count = '', first = nil, timer = nil }

local function stop()
  if waiting.timer then
    waiting.timer:kill()
    waiting.timer = nil
  end
  waiting.count, waiting.first = '', nil
end

local function times(count, command)
  for _ = 1, count or 1 do
    mp.command(command)
  end
end

-- To that one in the list, counted from one, as the list's numbers are.
local function nth(count)
  local last = mp.get_property_number('playlist-count', 1)
  mp.set_property_number('playlist-pos', math.min(count, last) - 1)
end

-- To the first or the last in the list, or with a number, that one.
local function end_or_nth(first)
  return function(count)
    if count then
      nth(count)
    elseif first then
      nth(1)
    else
      nth(mp.get_property_number('playlist-count', 1))
    end
  end
end

local function seek(seconds)
  return function(count)
    times(count, 'seek ' .. seconds)
  end
end

-- The launcher's Shortcuts, at the player's keys, as the picture and
-- document viewers' keys open it, rather than mpv's own page of every
-- binding, in mpv's words, which runs off the window.
local function shortcuts()
  mp.commandv('run', 'launcher-say', 'collection', 'shortcuts')
end

local function close()
  mp.command('quit')
end

-- What each key does, given the number before it, if any.
local alone = {
  h = seek(-5),
  LEFT = seek(-5),
  l = seek(5),
  RIGHT = seek(5),
  j = seek(60),
  DOWN = seek(60),
  k = seek(-60),
  UP = seek(-60),
  -- A 0 after a number is part of it, so 0 alone is only ever alone.
  ['0'] = function()
    mp.command('seek 0 absolute')
  end,
  -- keep-open holds the last frame there.
  ['$'] = function()
    mp.command('seek 100 absolute-percent')
  end,
  G = end_or_nth(false),
  SPACE = function()
    mp.command('cycle pause')
  end,
  ['['] = function(count)
    times(count, 'multiply speed 1/1.1')
  end,
  [']'] = function(count)
    times(count, 'multiply speed 1.1')
  end,
  b = function()
    mp.command('cycle deband')
  end,
  Z = function(count)
    times(count, 'add sub-delay +0.1')
  end,
  ['?'] = shortcuts,
  F1 = shortcuts,
  -- Nothing closes by a key pressed in passing; what does is said, over
  -- the picture, until it goes.
  q = function()
    mp.osd_message('q does nothing · ZZ closes')
  end,
  Q = function()
    mp.osd_message('Q does nothing · ZZ closes')
  end,
}

local paired = {
  g = { g = end_or_nth(true) },
  [']'] = {
    b = function(count)
      times(count, 'playlist-next')
    end,
  },
  ['['] = {
    b = function(count)
      times(count, 'playlist-prev')
    end,
  },
  y = {
    y = function()
      -- mpv names a script by its file, a dash made an underscore.
      mp.command('script-binding copy_frame/copy')
    end,
  },
  Z = { Z = close },
}

local pressed

-- Once nothing more comes, the key waiting does what it does alone.
local function wait()
  if waiting.timer then
    waiting.timer:kill()
  end
  waiting.timer = mp.add_timeout(0.3, function()
    local first, count = waiting.first, tonumber(waiting.count)
    stop()
    if first and alone[first] then
      alone[first](count)
    end
  end)
end

pressed = function(key)
  local count = tonumber(waiting.count)
  if key == 'ESC' and (count or waiting.first) then
    -- Escape cancels what waits, and does nothing more.
    stop()
  elseif waiting.first then
    local first = waiting.first
    local act = paired[first][key]
    stop()
    if act then
      act(count)
    else
      -- As in Vim: the first does what it does alone, and this key is
      -- heard on its own.
      if alone[first] then
        alone[first](count)
      end
      pressed(key)
    end
  elseif key:match('^%d$') and (count or key ~= '0') then
    waiting.count = waiting.count .. key
    wait()
  elseif paired[key] then
    waiting.first = key
    wait()
  elseif key == 'ESC' then
    -- At rest Escape closes nothing, and only takes the player out of
    -- full screen, as everywhere.
    mp.command('set fullscreen no')
  else
    stop()
    if alone[key] then
      alone[key](count)
    end
  end
end

-- Every key that acts alone, starts a pair or ends one, and the digits.
local bound = { ESC = true }
for key in pairs(alone) do
  bound[key] = true
end
for first, seconds in pairs(paired) do
  bound[first] = true
  for second in pairs(seconds) do
    bound[second] = true
  end
end
for digit = 0, 9 do
  bound[tostring(digit)] = true
end
-- A seek held down goes on seeking, as mpv's arrows did.
local held = {
  h = true,
  j = true,
  k = true,
  l = true,
  LEFT = true,
  RIGHT = true,
  UP = true,
  DOWN = true,
}
-- Bound ahead of mpv's own keys but not forced, so the input.conf beside
-- this script, which binds none of these, and any program that runs mpv
-- with keys of its own forced, as the camera's viewfinder takes Escape and
-- q, always win. Every mpv loads this script, and of two forced bindings
-- on one key the last loaded wins, which took Escape from the viewfinder.
for key in pairs(bound) do
  mp.add_key_binding(key, 'vi-' .. key, function()
    pressed(key)
  end, { repeatable = held[key] or false })
end

-- A time as the command line takes it, 12:30 or 750, in seconds.
local function seconds(time)
  local n = '[%d.]+'
  local whole = '^' .. n .. '$'
  if not (time:match(whole) or time:match('^' .. n .. ':' .. n .. '$')
    or time:match('^' .. n .. ':' .. n .. ':' .. n .. '$')) then
    return nil
  end
  local total = 0
  for part in time:gmatch('[^:]+') do
    total = total * 60 + (tonumber(part) or 0)
  end
  return total
end

-- :set as Vim's is: a name to turn it on, no before it to turn it off, !
-- after it to turn it over, and name=value, so :set pause and :set
-- volume=40 are mpv's own properties.
local function set(rest)
  local name, value = rest:match('^([%w-]+)=(.*)$')
  if rest == '' then
    mp.osd_message(':set needs a name')
  elseif name then
    mp.commandv('set', name, value)
  elseif rest:match('!$') then
    mp.commandv('cycle', rest:sub(1, -2))
  elseif rest:match('^no') and mp.get_property(rest) == nil then
    mp.commandv('set', rest:sub(3), 'no')
  else
    mp.commandv('set', rest, 'yes')
  end
end

-- What the launcher's command line asks of the video, by the command's
-- whole name, whichever short form was typed, and the rest of what was
-- typed: :set, a time to go to, as :12:30, :y and :q.
local commands = {
  set = set,
  ['goto'] = function(rest)
    local at = seconds(rest)
    if at then
      mp.commandv('seek', tostring(at), 'absolute')
    else
      mp.osd_message('No time ' .. rest)
    end
  end,
  yank = function()
    mp.command('script-binding copy_frame/copy')
  end,
  quit = close,
}

-- mpv-command (command.sh) carries each over mpv's socket, as a message
-- to this script.
mp.register_script_message('command', function(name, rest)
  if commands[name] then
    commands[name](rest or '')
  else
    mp.osd_message('No command ' .. name)
  end
end)

-- The command line reaches this mpv over its socket: the one it was given,
-- or one of its own, named for its process, so every mpv has one.
local runtime = os.getenv('XDG_RUNTIME_DIR')
if mp.get_property('input-ipc-server', '') == '' and runtime then
  local pid = require('mp.utils').getpid()
  mp.set_property('input-ipc-server', runtime .. '/mpv-' .. pid .. '.sock')
end
