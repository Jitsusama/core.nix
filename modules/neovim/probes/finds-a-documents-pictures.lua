-- The markdown file open, a plan in a quest's plans, names a picture in
-- the quest's attachments by a path relative to the plan; Snacks' images,
-- switched on, find it there as the file's one picture,
-- and a picture other than PNG has ImageMagick to turn it into one. A
-- terminal draws it, which a headless Neovim has none of.
require('later').load_all()
local wrong = {}
local function say(line)
  table.insert(wrong, line)
end

if not Snacks.config.image.enabled then
  say("Snacks' images are off")
end
if vim.fn.executable('magick') ~= 1 then
  say('no ImageMagick turns a JPEG into a PNG')
end

local wanted = vim.uv.fs_realpath('attachments/chart.png')
Snacks.image.doc.find(0, function(pictures)
  if #pictures ~= 1 then
    say('the file has ' .. #pictures .. ' pictures, not one')
  elseif pictures[1].src ~= wanted then
    say('its picture is ' .. vim.inspect(pictures[1].src) .. ', not ' .. wanted)
  end
  io.stdout:write(#wrong == 0 and 'as designed' or table.concat(wrong, '\n'))
  vim.cmd('qa!')
end)
