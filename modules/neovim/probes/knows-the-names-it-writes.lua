local line = vim.api.nvim_buf_get_lines(0, 0, 1, false)[1]
local flagged = {}
for _, bad in ipairs(vim.spell.check(line)) do
  table.insert(flagged, bad[1])
end
local said = table.concat(flagged, ' ')
io.stdout:write(
  vim.wo.spell and said == 'speling' and 'as designed'
    or ('spell ' .. tostring(vim.wo.spell) .. ', flagged: ' .. said)
)
vim.cmd('qa!')
