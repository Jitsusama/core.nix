-- vim-easy-align: Simple text alignment plugin

-- Visual mode: Select text then <leader>cn to start alignment
vim.keymap.set('x', '<leader>cn', '<Plug>(EasyAlign)', { desc = 'Code align (interactive)' })
-- Normal mode: <leader>cn{motion} to align (e.g., <leader>cnip for inner paragraph)
vim.keymap.set('n', '<leader>cn', '<Plug>(EasyAlign)', { desc = 'Code align for motion' })
