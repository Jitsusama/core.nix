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

swayimg.text.font = nix.sans
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

-- A key as the desktop draws it, by its xkb name, and a letter as itself.
local function face(name)
  return nix.keys[name] or name
end
local arrows = face('Left') .. ' ' .. face('Right')
local ends = 'Home End'

-- Every key each mode answers that's worth knowing, as drawn and what it
-- does, which ? lists; and the few that move on, which a quiet line names
-- as a picture opens.
local keys = {
  [swayimg.viewer] = {
    listed = {
      { arrows, 'previous, next' },
      { ends, 'first, last' },
      { face('Return'), 'every picture' },
      { '+ -', 'closer, further' },
      { '[ ]', 'turn' },
      { 'F', 'full screen' },
      { 'E', 'edit' },
      { 'Y', 'copy' },
      { 'Delete', 'to the trash' },
      { face('Escape'), 'close' },
    },
    hint = arrows .. '  pictures     ' .. face('Return') .. '  all     ?  keys',
  },
  [swayimg.gallery] = {
    listed = {
      { arrows .. ' ' .. face('Up') .. ' ' .. face('Down'), 'choose' },
      { ends, 'first, last' },
      { face('Return'), 'open' },
      { '+ -', 'larger, smaller' },
      { 'E', 'edit' },
      { 'Y', 'copy' },
      { 'Delete', 'to the trash' },
      { face('Escape'), 'close' },
    },
    hint = face('Return') .. '  open     ?  keys',
  },
}

-- Whether ? has the keys listed, which stays until it's pressed again.
local listing = false

local function shown(mode)
  local bottom = {}
  if listing then
    for _, row in ipairs(keys[mode].listed) do
      table.insert(bottom, row[1] .. '\t' .. row[2])
    end
  elseif mode ~= swayimg.viewer or swayimg.imagelist.size > 1 then
    bottom = { keys[mode].hint }
  end
  mode.text =
    { topleft = { '{name}' }, topright = counted(), bottomleft = bottom, bottomright = {} }
end

-- The list shows until ? is pressed again, and then goes with the rest of
-- the words, two seconds on.
local function list(mode)
  listing = not listing
  swayimg.text.timeout = listing and 0 or 2
  shown(mode)
  swayimg.text.visible = true
end

for _, mode in ipairs({ swayimg.viewer, swayimg.gallery }) do
  mode.on_image_change(function()
    shown(mode)
  end)
end

-- The arrows go that way, to the previous and the next picture, as Page Up
-- and Page Down do too, and Home and End to the first and the last: the
-- arrows' own default only moves a picture larger than the window.
for key, where in pairs({ Left = 'prev', Right = 'next', Home = 'first', End = 'last' }) do
  swayimg.viewer.on_key(key, function()
    swayimg.viewer.open(where)
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
  swayimg.exit(0)
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
  swayimg.imagelist.remove(image.path)
  swayimg.text.status = 'Moved to the trash'
end

for _, mode in ipairs({ swayimg.viewer, swayimg.gallery }) do
  mode.on_key('e', function()
    edit(mode)
  end)
  mode.on_key('y', function()
    copy(mode)
  end)
  mode.on_key('Delete', function()
    trash(mode)
  end)
  -- q as well as Escape, as in mpv.
  mode.on_key('q', function()
    swayimg.exit(0)
  end)
  -- ? says what's here: swayimg hears it as the key it's shifted from.
  mode.on_key('Shift-slash', function()
    list(mode)
  end)
end
