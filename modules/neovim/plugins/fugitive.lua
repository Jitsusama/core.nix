-- Fugitive runs git for Space g's keys (keys.nix), and its status window
-- gets the status keys there too.
local keys = require('keys')

local function ask(prompt, command)
  return function()
    local name = vim.fn.input(prompt)
    if name ~= '' then
      vim.cmd(command .. name)
    end
  end
end

keys.act('git.tag', ask('Tag name: ', 'Git tag '))
keys.act('git.untag', ask('Delete tag: ', 'Git tag -d '))

vim.api.nvim_create_autocmd('FileType', {
  pattern = 'fugitive',
  callback = function(args)
    keys.attach('status', args.buf)
  end,
})
