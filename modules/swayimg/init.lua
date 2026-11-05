-- swayimg's settings: a quick look at a picture, in the theme, that gets out
-- of the way. home.nix says what each part is for; this file is swayimg's
-- own Lua, and nix.lua beside it, which home.nix writes, holds what comes
-- from the theme and the programs the keys run.
local nix = require('nix')
local colour = nix.colour

-- swayimg sizes its text's face by the screen's scale but counts padding,
-- thumbnails and borders in the screen's own pixels, and doesn't say what
-- the scale is. niri does, for the screen it opens on.
local function screen_scale()
  local answer = io.popen('niri msg --json focused-output 2>/dev/null')
  if not answer then
    return 1
  end
  local told = answer:read('*a')
  answer:close()
  return tonumber(told:match('"scale":%s*([%d.]+)')) or 1
end

local scale = screen_scale()

local function px(logical)
  return math.floor(logical * scale + 0.5)
end

-- A path as one word for sh.
local function quoted(path)
  return "'" .. path:gsub("'", "'\\''") .. "'"
end

-- What to call the count of pictures: nothing when there's only one.
local function counted()
  if swayimg.imagelist.size > 1 then
    return { '{list.index} of {list.total}' }
  end
  return {}
end

-- swayimg makes its window's picture at the screen's scale when told the
-- scale, but doesn't tell niri the window's size back then, so a scale that
-- comes after niri sizes the window leaves it drawn twice as large as meant.
-- Giving swayimg its own size again as the size changes says it. Only as it
-- changes: giving it any sooner draws, and a window drawn before it has
-- answered niri's first configure breaks the protocol, and niri closes it.
-- Giving it is itself a change of size, which is let pass, or a scale that
-- rounds would shrink the window a pixel at a time.
local giving = false
swayimg.on_window_resize(function()
  if giving then
    giving = false
    return
  end
  giving = true
  local size = swayimg.get_window_size()
  swayimg.set_window_size(size.width, size.height)
end)

-- A picture opened alone brings its folder, so the arrows go on through
-- the rest, and the folder is watched, so one saved meanwhile joins them.
swayimg.imagelist.adjacent = true
swayimg.imagelist.fsmon = true

swayimg.text.font = nix.mono
swayimg.text.size = nix.size
swayimg.text.padding = px(nix.space.l)
swayimg.text.color = colour.strong
swayimg.text.shadow = colour.shadow
-- The name shows as a picture opens and goes, so nothing sits on it.
swayimg.text.timeout = 2
swayimg.text.status_timeout = 2

swayimg.viewer.set_window_background(colour.deepest)
swayimg.viewer.set_image_chessboard(px(nix.space.s), colour.sunken, colour.raised)
-- The next and previous pictures are ready before they're asked for.
swayimg.viewer.preload = 2
swayimg.viewer.history = 2
swayimg.viewer.mark_color = colour.strong

swayimg.gallery.thumb_size = px(nix.thumbnail)
swayimg.gallery.padding_size = px(nix.space.s)
-- The one selected has the accent's hairline, as a focused window does, and
-- rises into half the gap around it, since a hairline alone is lost on a
-- busy picture.
swayimg.gallery.border_size = px(nix.border)
swayimg.gallery.selected_scale = 1 + nix.space.s / nix.thumbnail
swayimg.gallery.window_color = colour.deepest
swayimg.gallery.unselected_color = colour.sunken
swayimg.gallery.selected_color = colour.raised
swayimg.gallery.border_color = colour.accent
swayimg.gallery.mark_color = colour.strong
-- Thumbnails are kept in the cache, so a folder seen once opens at once,
-- and made ahead of the scroll.
swayimg.gallery.pstore = true
swayimg.gallery.preload = true

-- A font's sample and a video's storyboard in the theme too.
swayimg.format_conf = {
  ttf = { color = colour.text, background = colour.deepest },
  video = { label = colour.muted },
}

-- Where each mode's keys work, in the words the list says it with.
local place = { [swayimg.viewer] = nix.places.viewer, [swayimg.gallery] = nix.places.gallery }

-- Whether a key works in a mode: everywhere, or there.
local function works(key, mode)
  return key.when == '' or key.when == nil or key.when == place[mode]
end

-- A key as the desktop draws it, by its chord.
local function face(chord)
  for _, key in ipairs(nix.keys) do
    if key.chord == chord then
      return key.face
    end
  end
  return chord
end

-- The few keys that move on, which a quiet line names as a picture opens,
-- by their letters, as every hint of Joel's names them.
local hint = {
  [swayimg.viewer] = face('h')
    .. ' '
    .. face('l')
    .. '  pictures     '
    .. face('Return')
    .. '  all     '
    .. face('space Shift+slash')
    .. '  keys',
  [swayimg.gallery] = face('Return') .. '  open     ' .. face('space Shift+slash') .. '  keys',
}

-- A number, or the first of a pair of keys, waiting for what comes after
-- it. Nothing shows it, as Joel's Neovim has showcmd off.
local waiting = { count = '', first = nil, turn = 0 }

local function shown(mode)
  local bottom = {}
  if mode ~= swayimg.viewer or swayimg.imagelist.size > 1 then
    bottom = { hint[mode] }
  end
  mode.text =
    { topleft = { '{name}' }, topright = counted(), bottomleft = bottom, bottomright = {} }
end

-- Nothing waits any more.
local function stop_waiting()
  waiting.count, waiting.first = '', nil
  waiting.turn = waiting.turn + 1
end

-- What's waiting goes 300 ms on with nothing more, as Joel's Neovim's
-- timeoutlen has it, doing what it does alone, if anything.
local function wait(timed_out)
  waiting.turn = waiting.turn + 1
  local turn = waiting.turn
  swayimg.defer(0.3, function()
    if waiting.turn == turn then
      local count = tonumber(waiting.count)
      stop_waiting()
      if timed_out then
        timed_out(count)
      end
    end
  end)
end

-- The keys are listed where every program's are, in the launcher's
-- Shortcuts, which opens at swayimg's own, as zathura's Space ? does. A
-- list drawn in the window ran past its foot, as a window fits fewer rows
-- than swayimg has keys.
local function list()
  os.execute('launcher-say collection shortcuts >/dev/null 2>&1 &')
end

-- The picture looked at is kept for the launcher, which leads swayimg's
-- Recent with it: a line saying this swayimg's process and the picture's
-- whole path, in nix.viewing, written aside and renamed over so it's never
-- read half written, and removed as swayimg closes. What one killed left
-- is cleared by the next to start. Keeping it is a courtesy, so a failure
-- is passed over.
local stat = io.open('/proc/self/stat')
local pid = stat and stat:read('*n')
if stat then
  stat:close()
end
local kept = pid and (nix.viewing .. '/' .. pid)

if pid then
  os.execute(
    '{ mkdir -p '
      .. quoted(nix.viewing)
      .. ' && cd '
      .. quoted(nix.viewing)
      .. ' && for kept in [0-9]* .[0-9]*; do'
      .. ' [ -e "$kept" ] && [ ! -e "/proc/${kept#.}" ] && rm -f -- "$kept"; done; } 2>/dev/null'
  )
end

local function keep(mode)
  local image = mode.get_image()
  if not kept or not image then
    return
  end
  local writing = nix.viewing .. '/.' .. pid
  local file = io.open(writing, 'w')
  if file then
    file:write(pid .. '\t' .. image.path .. '\n')
    file:close()
    os.rename(writing, kept)
  end
end

local function quit()
  if kept then
    os.remove(kept)
  end
  swayimg.exit(0)
end

-- The picture it was opened with is said to the desktop's list of files
-- opened lately (open-with's recent-files, where the desktop has it), so
-- swayimg's Recent in the launcher lists it. The pictures stepped through
-- after it aren't, as GNOME's viewers leave them, or a folder looked
-- through would fill Recent.
local opened_said = false

local function say_opened()
  local image = swayimg.viewer.get_image()
  if opened_said or not image then
    return
  end
  opened_said = true
  local said = 'recent-files add swayimg ' .. quoted(image.path)
  os.execute('command -v recent-files >/dev/null && ' .. said .. ' &')
end

for _, mode in ipairs({ swayimg.viewer, swayimg.gallery }) do
  mode.on_image_change(function()
    shown(mode)
    keep(mode)
    if mode == swayimg.viewer then
      say_opened()
    end
  end)
end

-- What each key acts on: the picture in view, or the one selected.
local function current(mode)
  return mode.get_image()
end

-- Editing is GIMP's, and a drawing's Inkscape's, each opened by niri so
-- it's the desktop's and stays when the viewer closes, which it does, since
-- the look is over.
local function edit(mode)
  local image = current(mode)
  if not image then
    return
  end
  local editor = image.path:lower():match('%.svgz?$') and nix.inkscape or nix.gimp
  local found = os.execute('command -v ' .. editor.command .. ' >/dev/null')
  if found ~= 0 and found ~= true then
    swayimg.text.status = editor.name .. " isn't here to edit it"
    return
  end
  os.execute('niri msg action spawn -- ' .. editor.command .. ' ' .. quoted(image.path))
  quit()
end

-- The picture as it's shown, turned or flipped, goes to the clipboard as a
-- PNG, which every program takes; a thumbnail's file goes as it is.
local function copy(mode)
  local image = current(mode)
  if not image then
    return
  end
  local file = image.path
  if mode == swayimg.viewer then
    file = os.getenv('XDG_RUNTIME_DIR') .. '/swayimg-copied.png'
    swayimg.viewer.export(file)
  end
  os.execute(nix.wl_copy .. ' < ' .. quoted(file))
  swayimg.text.status = 'Copied'
end

-- The pictures moved to the trash, the last one last, for u to bring back.
local trashed = {}

-- To the trash, where it can be had back, and out of the list.
local function trash(mode)
  local image = current(mode)
  if not image then
    return
  end
  local moved = os.execute(nix.gio .. ' trash ' .. quoted(image.path))
  if moved ~= 0 and moved ~= true then
    swayimg.text.status = "Couldn't move it to the trash"
    return
  end
  table.insert(trashed, image.path)
  swayimg.imagelist.remove(image.path)
  swayimg.text.status = 'Moved to the trash'
end

-- The trash in the home, as the freedesktop.org trash has it: each
-- picture in files, and beside it in info where it came from and when.
local function home_trash()
  local data = os.getenv('XDG_DATA_HOME')
  if not data or data == '' then
    data = os.getenv('HOME') .. '/.local/share'
  end
  return data .. '/Trash'
end

local function unescaped(text)
  return (text:gsub('%%(%x%x)', function(hex)
    return string.char(tonumber(hex, 16))
  end))
end

-- The newest of what the trash keeps from that path, by its name there.
local function kept_from(bin, path)
  local found, newest = nil, ''
  local names = io.popen('ls -A ' .. quoted(bin .. '/info') .. ' 2>/dev/null')
  if not names then
    return nil
  end
  for name in names:lines() do
    local file = name:match('%.trashinfo$') and io.open(bin .. '/info/' .. name)
    if file then
      local info = file:read('*a')
      file:close()
      local from = info:match('\nPath=([^\n]*)')
      local when = info:match('\nDeletionDate=([^\n]*)') or ''
      if from and unescaped(from) == path and when >= newest then
        found, newest = name:gsub('%.trashinfo$', ''), when
      end
    end
  end
  names:close()
  return found
end

-- The last picture moved to the trash comes back to where it was, and into
-- its place in the list, in view. gio can't take it back without GVfs's
-- trash, which the desktop doesn't run, so it's moved back as the trash
-- keeps it.
local function restore(mode)
  local path = trashed[#trashed]
  if not path then
    return
  end
  local bin = home_trash()
  local name = kept_from(bin, path)
  local there = io.open(path)
  if there then
    there:close()
  end
  if not name or there or not os.rename(bin .. '/files/' .. name, path) then
    swayimg.text.status = "Couldn't bring it back"
    return
  end
  table.remove(trashed)
  os.remove(bin .. '/info/' .. name .. '.trashinfo')
  swayimg.imagelist.add(path)
  if mode == swayimg.viewer then
    swayimg.viewer.open_path(path)
  else
    swayimg.gallery.select_path(path)
  end
  swayimg.text.status = 'Brought back'
end

-- The picture at that place in the list, counted as its name's count is.
local function nth(mode, number)
  local entry = swayimg.imagelist.get()[number]
  if not entry then
    return
  end
  if mode == swayimg.viewer then
    swayimg.viewer.open_path(entry.path)
  else
    swayimg.gallery.select_path(entry.path)
  end
end

-- A picture larger than the window moves a tenth of the window, as
-- swayimg's own arrows moved it, and one that fits stays.
local function pan(rows)
  local at = swayimg.viewer.get_position()
  local size = swayimg.get_window_size()
  swayimg.viewer.set_abs_position(at.x, at.y + rows * math.floor(size.height / 10))
end

-- Each move, as many times as the number before it says.
local function times(count, fn)
  for _ = 1, count or 1 do
    fn()
  end
end

-- A move one way: on a picture, the previous or the next, or the picture
-- moved up or down; in the gallery, the picture chosen that way.
local function way(viewer, gallery)
  return function(mode, count)
    times(count, function()
      if mode == swayimg.viewer then
        viewer()
      else
        swayimg.gallery.select(gallery)
      end
    end)
  end
end

-- To the first or the last, or with a number, that picture.
local function end_or_nth(dir)
  return function(mode, count)
    if count then
      nth(mode, count)
    elseif mode == swayimg.viewer then
      swayimg.viewer.open(dir)
    else
      swayimg.gallery.select(dir)
    end
  end
end

-- Escape leaves full screen, and at rest does nothing: a look is put away
-- by ZZ or Space w, as every window of Joel's is, never by a key pressed
-- in passing.
local function escape()
  if swayimg.fullscreen then
    swayimg.fullscreen = false
  end
end

-- q closes nothing, as in all his programs, and says what does.
local function inert()
  swayimg.text.status = 'q does nothing · ZZ closes'
end

-- What each key with an act does. The arrows go that way, to the previous
-- and the next picture, and Home and End to the first and the last: the
-- arrows' own default only moves a picture larger than the window, which
-- Up and Down still do.
local acts = {
  left = way(function()
    swayimg.viewer.open('prev')
  end, 'left'),
  right = way(function()
    swayimg.viewer.open('next')
  end, 'right'),
  up = way(function()
    pan(1)
  end, 'up'),
  down = way(function()
    pan(-1)
  end, 'down'),
  first = end_or_nth('first'),
  last = end_or_nth('last'),
  edit = edit,
  copy = copy,
  trash = trash,
  restore = restore,
  escape = escape,
  turn_left = function()
    swayimg.viewer.rotate(270)
  end,
  turn_right = function()
    swayimg.viewer.rotate(90)
  end,
  close = quit,
  inert = inert,
  keys = list,
}

-- Each key with an act, by the key that starts it, where it works: alone,
-- or as the first of a pair and the key after it. swayimg spells a held
-- key with a dash, and a letter by its own name, Shift-g, but a symbol by
-- what Shift makes of it: ? comes as Shift-question, never as the slash the
-- table names it by.
local shifted = { ['Shift+slash'] = 'Shift+question' }
local function heard(chord)
  return ((shifted[chord] or chord):gsub('%+', '-'))
end
local alone, paired = {}, {}
for _, mode in ipairs({ swayimg.viewer, swayimg.gallery }) do
  alone[mode], paired[mode] = {}, {}
  for _, key in ipairs(nix.keys) do
    if key.act and works(key, mode) then
      local first, second = key.chord:match('^(%S+) (%S+)$')
      if first then
        first, second = heard(first), heard(second)
        paired[mode][first] = paired[mode][first] or {}
        paired[mode][first][second] = key.act
      else
        alone[mode][heard(key.chord)] = key.act
      end
    end
  end
  -- Unlisted, since it does nothing, but bound, to say so.
  alone[mode].q = 'inert'
end

-- The slideshow, swayimg's own, which s starts, closed on Escape as the
-- viewer did; there Escape goes back to the picture, as s does, and q
-- says what closes.
swayimg.slideshow.on_key('Escape', function()
  swayimg.mode = 'viewer'
end)
swayimg.slideshow.on_key('q', inert)

-- A key pressed: the second of a pair waiting finishes it; a digit adds to
-- the number, a pair's first key waits for its second, and a key alone
-- does its act, that many times.
local function pressed(mode, key)
  local count = tonumber(waiting.count)
  if waiting.first then
    local first = waiting.first
    local act = paired[mode][first][key]
    stop_waiting()
    if act then
      acts[act](mode, count)
    elseif alone[mode][key] == 'escape' then
      -- Escape only takes back what waited, as in Vim.
      return
    else
      -- As in Vim: the first does what it does alone, and this key is
      -- heard on its own.
      if alone[mode][first] then
        acts[alone[mode][first]](mode, count)
      end
      pressed(mode, key)
    end
    return
  end
  local act = alone[mode][key]
  if key:match('^%d$') and not act and (count or key ~= '0') then
    waiting.count = waiting.count .. key
    wait()
  elseif paired[mode][key] then
    -- A key that starts a pair waits for its second, and alone, as y and
    -- [ are too, does what it does once none comes.
    waiting.first = key
    wait(act and function(times_over)
      acts[act](mode, times_over)
    end)
  elseif act == 'escape' and count then
    stop_waiting()
  else
    stop_waiting()
    if act then
      acts[act](mode, count)
    end
  end
end

-- Every key that acts, starts a pair or ends one, and the digits.
for _, mode in ipairs({ swayimg.viewer, swayimg.gallery }) do
  local bound = {}
  for key in pairs(alone[mode]) do
    bound[key] = true
  end
  for first, seconds in pairs(paired[mode]) do
    bound[first] = true
    for second in pairs(seconds) do
      bound[second] = true
    end
  end
  for digit = 0, 9 do
    bound[tostring(digit)] = true
  end
  for key in pairs(bound) do
    mode.on_key(key, function()
      pressed(mode, key)
    end)
  end
end
