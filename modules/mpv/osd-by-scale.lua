-- mpv draws its messages in the screen's own pixels once they don't grow
-- with the window, as mpv.conf has them, and never counts in logical
-- pixels, so this sets each of their sizes from the logical ones home.nix
-- gives it, times the screen's scale, whenever the scale changes.
local options = require('mp.options')

local given = { size = 0, margin_x = 0, margin_y = 0, outline = 0 }
options.read_options(given, 'osd-by-scale')

local function fit(_, scale)
  if not scale or scale <= 0 then
    scale = 1
  end
  mp.set_property_number('osd-font-size', given.size * scale)
  mp.set_property_number('osd-margin-x', math.floor(given.margin_x * scale + 0.5))
  mp.set_property_number('osd-margin-y', math.floor(given.margin_y * scale + 0.5))
  mp.set_property_number('osd-border-size', given.outline * scale)
end

mp.observe_property('display-hidpi-scale', 'number', fit)
