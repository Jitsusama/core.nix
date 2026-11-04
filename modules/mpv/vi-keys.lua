-- Vim's keys in mpv, as Joel's Vim has them: h and l seek back and on 5
-- seconds, j on a minute and k back, as j goes on through a file; 0 goes
-- to the start and $ to the end; gg plays the first in the list, G the
-- last, and a number before either that one; ]b and [b the next and the
-- one before, as Vim's go between buffers; yy copies the frame; Space ?
-- lists the keys, as ? does; and a number before any of them does it that
-- many times. The arrows stay mpv's own. mpv has no numbers before keys,
-- and does a key the moment it's pressed even when another binding starts
-- with it, so these are bound here, ahead of mpv's own. A key that starts
-- a pair waits 300 ms, his Neovim's timeoutlen, for its second, and does
-- what it does alone once none comes: Space pauses, [ and ] change the
-- speed, and b smooths banding, as mpv has them; y alone does nothing, as
-- in Vim. What waits shows at the foot's right, muted, as Neovim's
-- showcmd.
local options = { muted = '808080' }
require('mp.options').read_options(options, 'vi-keys')

local waiting = { count = '', first = nil, timer = nil }
local shown = mp.create_osd_overlay('ass-events')

local function show()
  local typed = waiting.count .. (waiting.first or '')
  if typed == '' then
    shown:remove()
    return
  end
  shown.data = '{\\an3}{\\1c&H' .. options.muted .. '&}' .. typed
  shown:update()
end

local function stop()
  if waiting.timer then
    waiting.timer:kill()
    waiting.timer = nil
  end
  waiting.count, waiting.first = '', nil
  show()
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

-- What each key does, given the number before it, if any.
local alone = {
  h = function(count)
    times(count, 'seek -5')
  end,
  l = function(count)
    times(count, 'seek 5')
  end,
  j = function(count)
    times(count, 'seek 60')
  end,
  k = function(count)
    times(count, 'seek -60')
  end,
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
  -- mpv's own, bound here since Space ? ends on it.
  ['?'] = function()
    mp.command('script-binding stats/display-page-4-toggle')
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
  SPACE = { ['?'] = alone['?'] },
}

local pressed

-- Once nothing more comes, the key waiting does what it does alone.
local function wait()
  show()
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
-- A seek held down goes on seeking, as mpv's arrows do.
local held = { h = true, j = true, k = true, l = true }
for key in pairs(bound) do
  mp.add_forced_key_binding(key, 'vi-' .. key, function()
    pressed(key)
  end, { repeatable = held[key] or false })
end
