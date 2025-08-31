-- nvim-autopairs: Automatic insertion of closing brackets, quotes, etc.

local autopairs = require('nvim-autopairs')
local cmp_autopairs = require('nvim-autopairs.completion.cmp')
local cmp = require('cmp')

autopairs.setup({
  disable_filetype = { 'TelescopePrompt', 'oil' },
  check_ts = true,
})

-- Integration with nvim-cmp
cmp.event:on('confirm_done', cmp_autopairs.on_confirm_done())
