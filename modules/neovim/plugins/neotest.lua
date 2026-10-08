local neotest = require('neotest')

neotest.setup({
  adapters = {
    require('neotest-minitest')({}),
  },
  quickfix = {
    open = true,
  },
})

local function map(mode, keys, func, desc)
  vim.keymap.set(mode, keys, func, { desc = desc, silent = true })
end

-- Primary test operations
map('n', '<leader>tt', function()
  neotest.run.run()
end, 'Test nearest')
map('n', '<leader>tf', function()
  neotest.run.run(vim.fn.expand('%'))
end, 'Test file')
map('n', '<leader>ts', function()
  neotest.run.run(vim.fn.getcwd())
end, 'Test suite (all)')
map('n', '<leader>tl', function()
  neotest.run.run_last()
end, 'Test last')

-- Test output and results
map('n', '<leader>to', function()
  neotest.output.open({ enter = true, auto_close = true })
end, 'Test output')
map('n', '<leader>tO', function()
  neotest.output_panel.open()
end, 'Test Output panel')
map('n', '<leader>tS', function()
  neotest.summary.open()
end, 'Test Summary')

-- Test control
map('n', '<leader>tx', function()
  neotest.run.stop()
end, 'Test stop')
map('n', '<leader>tw', function()
  neotest.watch.toggle(vim.fn.expand('%'))
end, 'Test watch file')
map('n', '<leader>tW', function()
  neotest.watch.toggle(vim.fn.getcwd())
end, 'Test Watch suite')

-- Test debugging (future DAP integration placeholder)
map('n', '<leader>td', function()
  vim.notify('Test debugging requires DAP setup - not yet implemented', vim.log.levels.INFO)
end, 'Test debug (placeholder)')

-- Quick test navigation (non-leader)
map('n', ']t', function()
  neotest.jump.next()
end, 'Next test')
map('n', '[t', function()
  neotest.jump.prev()
end, 'Previous test')

-- Register with which-key if available
local wk_ok, which_key = pcall(require, 'which-key')
if wk_ok then
  which_key.add({
    -- Test domain
    { '<leader>tt', desc = 'Test nearest' },
    { '<leader>tf', desc = 'Test file' },
    { '<leader>ts', desc = 'Test suite (all)' },
    { '<leader>tl', desc = 'Test last' },
    { '<leader>to', desc = 'Test output' },
    { '<leader>tO', desc = 'Test Output panel' },
    { '<leader>tS', desc = 'Test Summary' },
    { '<leader>tx', desc = 'Test stop' },
    { '<leader>tw', desc = 'Test watch file' },
    { '<leader>tW', desc = 'Test Watch suite' },
    { '<leader>td', desc = 'Test debug (placeholder)' },

    -- Non-leader navigation
    { ']t', desc = 'Next test' },
    { '[t', desc = 'Previous test' },
  })
end
