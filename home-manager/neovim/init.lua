-- Core Vim settings
vim.opt.number = true
vim.opt.relativenumber = true
vim.opt.signcolumn = 'yes'
vim.opt.updatetime = 300 -- Faster cursor hold events (default is 4000ms)
vim.opt.scrolloff = 4 -- Lines of context when scrolling
vim.opt.sidescrolloff = 8 -- Columns of context

-- Terminal title settings
vim.opt.title = true
vim.opt.titlestring = '📝 %f'

-- Set leader key (needs to be in init.lua for other plugins to use)
vim.g.mapleader = ' '
vim.g.maplocalleader = ' '

-- UI Polish - Clean and sleek interface
vim.opt.pumblend = 10 -- Transparent completion popups
vim.opt.pumheight = 10 -- Maximum popup menu height
vim.opt.showmode = false -- Don't show mode in command line (shown in statusline)
vim.opt.laststatus = 3 -- Global statusline
vim.opt.cmdheight = 1 -- Command line height
vim.opt.ruler = false -- Don't show cursor position in command line

-- Better list characters
vim.opt.list = true
vim.opt.listchars = {
  tab = '→ ',
  trail = '·',
  extends = '❯',
  precedes = '❮',
  nbsp = '␣',
}

-- Search settings
vim.opt.ignorecase = true
vim.opt.smartcase = true -- Override ignorecase if search contains capitals
vim.opt.inccommand = 'split' -- Preview substitutions live
vim.opt.shortmess:append('S') -- Don't show search count in command line (we show it in lualine)
vim.opt.showcmdloc = 'statusline' -- Move partial commands to statusline instead of command line
vim.opt.showcmd = false -- Don't show partial commands at all (including selection size)

-- Indentation
vim.opt.expandtab = true
vim.opt.shiftwidth = 2
vim.opt.tabstop = 2
vim.opt.smartindent = true
vim.opt.shiftround = true -- Round indent to multiple of shiftwidth

-- Better splitting
vim.opt.splitbelow = true
vim.opt.splitright = true
vim.opt.splitkeep = 'screen' -- Keep the same relative cursor position

-- File handling
vim.opt.autowrite = true -- Auto save before commands like :next
vim.opt.confirm = true -- Confirm to save changes before exiting modified buffer
vim.opt.undofile = true -- Persistent undo
vim.opt.undolevels = 10000

-- Performance
vim.opt.timeoutlen = 300 -- Faster which-key popup
vim.opt.ttimeoutlen = 10 -- Faster key sequence completion

-- Clipboard
vim.opt.clipboard = vim.env.SSH_TTY and '' or 'unnamedplus' -- System clipboard unless in SSH

-- Better wrapping
vim.opt.wrap = false
vim.opt.linebreak = true -- Wrap at word boundaries when wrap is enabled
vim.opt.breakindent = true -- Preserve indentation in wrapped lines

-- Fold settings
vim.opt.foldlevel = 99
vim.opt.foldlevelstart = 99
vim.opt.foldenable = true

-- Session options
vim.opt.sessionoptions = { 'buffers', 'curdir', 'tabpages', 'winsize', 'help', 'globals', 'skiprtp', 'folds' }

-- Configure diagnostics appearance with better defaults
vim.diagnostic.config({
  float = {
    border = 'rounded',
    source = 'always',
    header = '',
    prefix = '',
  },
  severity_sort = true,
  signs = true,
  underline = true,
  update_in_insert = false,
  virtual_text = {
    spacing = 4,
    source = 'if_many',
    prefix = '●',
  },
})

-- Essential Autocmds
local function augroup(name)
  return vim.api.nvim_create_augroup('custom_' .. name, { clear = true })
end

-- Highlight on yank
vim.api.nvim_create_autocmd('TextYankPost', {
  group = augroup('highlight_yank'),
  callback = function()
    vim.highlight.on_yank({ timeout = 200 })
  end,
})

-- Restore cursor position
vim.api.nvim_create_autocmd('BufReadPost', {
  group = augroup('restore_cursor'),
  callback = function(event)
    local exclude = { 'gitcommit', 'gitrebase' }
    local buf = event.buf
    if vim.tbl_contains(exclude, vim.bo[buf].filetype) or vim.b[buf].cursor_restored then
      return
    end
    vim.b[buf].cursor_restored = true
    local mark = vim.api.nvim_buf_get_mark(buf, '"')
    local lcount = vim.api.nvim_buf_line_count(buf)
    if mark[1] > 0 and mark[1] <= lcount then
      pcall(vim.api.nvim_win_set_cursor, 0, mark)
    end
  end,
})

-- Check if file changed outside of Neovim
vim.api.nvim_create_autocmd({ 'FocusGained', 'TermClose', 'TermLeave' }, {
  group = augroup('checktime'),
  callback = function()
    if vim.o.buftype ~= 'nofile' then
      vim.cmd('checktime')
    end
  end,
})

-- Resize splits when window is resized
vim.api.nvim_create_autocmd('VimResized', {
  group = augroup('resize_splits'),
  callback = function()
    local current_tab = vim.fn.tabpagenr()
    vim.cmd('tabdo wincmd =')
    vim.cmd('tabnext ' .. current_tab)
  end,
})

-- Close certain filetypes with q
vim.api.nvim_create_autocmd('FileType', {
  group = augroup('close_with_q'),
  pattern = {
    'help',
    'lspinfo',
    'man',
    'notify',
    'qf',
    'query',
    'startuptime',
    'tsplayground',
    'checkhealth',
  },
  callback = function(event)
    vim.bo[event.buf].buflisted = false
    vim.keymap.set('n', 'q', '<cmd>close<cr>', { buffer = event.buf, silent = true })
  end,
})

-- Auto create directories when saving
vim.api.nvim_create_autocmd('BufWritePre', {
  group = augroup('auto_create_dir'),
  callback = function(event)
    if event.match:match('^%w%w+:[\\/][\\/]') then
      return
    end
    local file = vim.uv.fs_realpath(event.match) or event.match
    vim.fn.mkdir(vim.fn.fnamemodify(file, ':p:h'), 'p')
  end,
})

-- Wrap and spell check in text filetypes
vim.api.nvim_create_autocmd('FileType', {
  group = augroup('wrap_spell'),
  pattern = { 'gitcommit', 'markdown', 'text', 'tex' },
  callback = function()
    vim.opt_local.wrap = true
    vim.opt_local.spell = true
  end,
})

-- Disable line numbers in terminal
vim.api.nvim_create_autocmd('TermOpen', {
  group = augroup('terminal_setup'),
  callback = function()
    vim.opt_local.number = false
    vim.opt_local.relativenumber = false
    vim.opt_local.signcolumn = 'no'
  end,
})

-- Window Navigation (Alt+Shift+Arrow)
vim.keymap.set('n', '<M-S-Left>', '<C-w>h', { desc = 'Go to left window' })
vim.keymap.set('t', '<M-S-Left>', '<C-\\><C-n><C-w>h', { desc = 'Go to left window from terminal' })
vim.keymap.set('n', '<M-S-Right>', '<C-w>l', { desc = 'Go to right window' })
vim.keymap.set('t', '<M-S-Right>', '<C-\\><C-n><C-w>l', { desc = 'Go to right window from terminal' })
vim.keymap.set('n', '<M-S-Up>', '<C-w>k', { desc = 'Go to upper window' })
vim.keymap.set('t', '<M-S-Up>', '<C-\\><C-n><C-w>k', { desc = 'Go to upper window from terminal' })
vim.keymap.set('n', '<M-S-Down>', '<C-w>j', { desc = 'Go to lower window' })
vim.keymap.set('t', '<M-S-Down>', '<C-\\><C-n><C-w>j', { desc = 'Go to lower window from terminal' })

-- Tab Navigation (Control+Arrow)
vim.keymap.set('n', '<C-Left>', 'gT', { desc = 'Previous tab' })
vim.keymap.set('t', '<C-Left>', '<C-\\><C-n>gT', { desc = 'Previous tab from terminal' })
vim.keymap.set('n', '<C-Right>', 'gt', { desc = 'Next tab' })
vim.keymap.set('t', '<C-Right>', '<C-\\><C-n>gt', { desc = 'Previous tab from terminal' })

-- Terminal mode mappings
vim.keymap.set('t', '<S-Esc>', '<C-\\><C-n>', { desc = 'Exit terminal mode' })

-- Quickfix list toggle
vim.keymap.set('n', '<C-q>', function()
  local qf_open = false
  for _, win in pairs(vim.fn.getwininfo()) do
    if win['quickfix'] == 1 then
      qf_open = true
      break
    end
  end
  if qf_open then
    vim.cmd('cclose')
  else
    vim.cmd('copen')
  end
end, { desc = 'Toggle quickfix' })

-- Window Resizing (Alt+Arrow for height only, width conflicts with Ghostty)
vim.keymap.set('n', '<A-Up>', '<cmd>resize +2<cr>', { desc = 'Increase window height' })
vim.keymap.set('n', '<A-Down>', '<cmd>resize -2<cr>', { desc = 'Decrease window height' })

-- Better arrow key movement (respects wrapped lines)
vim.keymap.set(
  { 'n', 'x' },
  '<Down>',
  "v:count == 0 ? 'gj' : 'j'",
  { expr = true, silent = true, desc = 'Down (wrapped)' }
)
vim.keymap.set({ 'n', 'x' }, '<Up>', "v:count == 0 ? 'gk' : 'k'", { expr = true, silent = true, desc = 'Up (wrapped)' })

-- ===============================================
-- DOCUMENTED UI/SETTINGS DOMAIN (<leader>u)
-- Per KEYBINDINGS.md - UI appearance and editor settings
-- ===============================================

-- Toggle background between light and dark
vim.keymap.set('n', '<leader>ub', function()
  if vim.o.background == 'dark' then
    vim.o.background = 'light'
    vim.notify('Background: light', vim.log.levels.INFO)
  else
    vim.o.background = 'dark'
    vim.notify('Background: dark', vim.log.levels.INFO)
  end
end, { desc = 'UI background' })

-- Toggle line numbers (cycle through: none -> relative -> absolute)
vim.keymap.set('n', '<leader>ul', function()
  if not vim.wo.number and not vim.wo.relativenumber then
    -- Currently no numbers -> enable relative
    vim.wo.number = true
    vim.wo.relativenumber = true
    vim.notify('Line numbers: relative', vim.log.levels.INFO)
  elseif vim.wo.relativenumber then
    -- Currently relative -> switch to absolute
    vim.wo.relativenumber = false
    vim.notify('Line numbers: absolute', vim.log.levels.INFO)
  else
    -- Currently absolute -> disable all
    vim.wo.number = false
    vim.notify('Line numbers: disabled', vim.log.levels.INFO)
  end
end, { desc = 'UI line numbers' })

-- Toggle line wrapping
vim.keymap.set('n', '<leader>uw', function()
  vim.wo.wrap = not vim.wo.wrap
  if vim.wo.wrap then
    vim.notify('Line wrap: enabled', vim.log.levels.INFO)
  else
    vim.notify('Line wrap: disabled', vim.log.levels.INFO)
  end
end, { desc = 'UI wrap' })

-- ===============================================
-- DOCUMENTED SEQUENTIAL NAVIGATION ([]/])
-- Per KEYBINDINGS.md - Core Neovim functionality
-- ===============================================

-- Buffer navigation [b]/]b
vim.keymap.set('n', '[b', '<cmd>bprevious<cr>', { desc = 'Previous buffer' })
vim.keymap.set('n', ']b', '<cmd>bnext<cr>', { desc = 'Next buffer' })

-- Tab navigation [T]/]T
vim.keymap.set('n', '[T', '<cmd>tabprevious<cr>', { desc = 'Previous tab' })
vim.keymap.set('n', ']T', '<cmd>tabnext<cr>', { desc = 'Next tab' })

-- Window navigation [w]/]w
vim.keymap.set('n', '[w', '<C-w>W', { desc = 'Previous window' })
vim.keymap.set('n', ']w', '<C-w>w', { desc = 'Next window' })

-- Quickfix navigation [q]/]q
vim.keymap.set('n', '[q', function()
  pcall(vim.cmd, 'cprevious')
end, { desc = 'Previous quickfix' })

vim.keymap.set('n', ']q', function()
  pcall(vim.cmd, 'cnext')
end, { desc = 'Next quickfix' })

-- Location list navigation [l]/]l
vim.keymap.set('n', '[l', function()
  pcall(vim.cmd, 'lprevious')
end, { desc = 'Previous location' })

vim.keymap.set('n', ']l', function()
  pcall(vim.cmd, 'lnext')
end, { desc = 'Next location' })

-- ===============================================
-- DOCUMENTED TAB DOMAIN (<leader>T)
-- Per KEYBINDINGS.md - Tab management operations
-- ===============================================

-- Tab picker (double-T pattern)
vim.keymap.set('n', '<leader>TT', function()
  local tabs = {}
  for i = 1, vim.fn.tabpagenr('$') do
    local tabid = vim.fn.tabpagebuflist(i)[1]
    local name = vim.fn.bufname(tabid)
    if name == '' then
      name = '[No Name]'
    else
      name = vim.fn.fnamemodify(name, ':t') -- Just filename
    end
    table.insert(tabs, string.format('%d: %s', i, name))
  end

  vim.ui.select(tabs, {
    prompt = 'Select tab:',
    format_item = function(item)
      return item
    end,
  }, function(choice)
    if choice then
      local tab_num = choice:match('^(%d+):')
      if tab_num then
        vim.cmd('tabnext ' .. tab_num)
      end
    end
  end)
end, { desc = 'Tab picker' })

-- Tab operations
vim.keymap.set('n', '<leader>Tn', '<cmd>tabnew<cr>', { desc = 'Tab new' })
vim.keymap.set('n', '<leader>Tc', '<cmd>tabclose<cr>', { desc = 'Tab close' })
vim.keymap.set('n', '<leader>To', '<cmd>tabonly<cr>', { desc = 'Tab only' })
vim.keymap.set('n', '<leader>Tf', '<cmd>tabfirst<cr>', { desc = 'Tab first' })
vim.keymap.set('n', '<leader>Tl', '<cmd>tablast<cr>', { desc = 'Tab last' })

-- ===============================================
-- DOCUMENTED WINDOW DOMAIN (<leader>w)
-- Per KEYBINDINGS.md - Window management operations
-- ===============================================

-- Window picker (double-w pattern)
vim.keymap.set('n', '<leader>ww', function()
  -- Simple window selection - cycle through windows
  vim.cmd('wincmd w')
end, { desc = 'Window picker' })

-- Window operations
vim.keymap.set('n', '<leader>ws', '<cmd>split<cr>', { desc = 'Window split' })
vim.keymap.set('n', '<leader>wv', '<cmd>vsplit<cr>', { desc = 'Window vertical' })
vim.keymap.set('n', '<leader>wc', '<cmd>close<cr>', { desc = 'Window close' })
vim.keymap.set('n', '<leader>wo', '<cmd>only<cr>', { desc = 'Window only' })
vim.keymap.set('n', '<leader>w=', '<C-w>=', { desc = 'Window equalize' })

-- Window resizing
vim.keymap.set('n', '<leader>w+', '<cmd>resize +2<cr>', { desc = 'Window height+' })
vim.keymap.set('n', '<leader>w-', '<cmd>resize -2<cr>', { desc = 'Window height-' })
vim.keymap.set('n', '<leader>w>', '<cmd>vertical resize +2<cr>', { desc = 'Window width+' })
vim.keymap.set('n', '<leader>w<', '<cmd>vertical resize -2<cr>', { desc = 'Window width-' })

-- GitHub operations requiring system commands
-- PR operations that need system commands
vim.keymap.set({ 'n', 'v' }, '<leader>ghpy', function()
  local result = vim.fn.system('gh pr view --json url -q .url 2>/dev/null')
  if vim.v.shell_error == 0 then
    local url = result:gsub('\n', '')
    vim.fn.setreg('+', url)
    print('Copied PR URL: ' .. url)
  else
    print('No PR found for current branch')
  end
end, { desc = 'PR yank url' })

vim.keymap.set({ 'n', 'v' }, '<leader>ghpo', function()
  vim.cmd('!gh pr view --web 2>/dev/null || echo "No PR found for current branch"')
end, { desc = 'PR open browser' })

-- Repository operations that need system commands
vim.keymap.set({ 'n', 'v' }, '<leader>ghry', function()
  local url = vim.fn.system('gh repo view --json url -q .url'):gsub('\n', '')
  vim.fn.setreg('+', url)
  print('Copied repo URL: ' .. url)
end, { desc = 'Repo yank url' })

-- Gist operations that need system commands
vim.keymap.set('n', '<leader>ghgn', function()
  local filename = vim.fn.expand('%:t')
  if filename == '' then
    filename = 'untitled'
  end
  vim.cmd('write !gh gist create -f ' .. vim.fn.shellescape(filename))
end, { desc = 'Gist new (from buffer)' })

vim.keymap.set('v', '<leader>ghgn', function()
  vim.cmd("'<,'>write !gh gist create")
end, { desc = 'Gist new (from selection)' })

vim.keymap.set('n', '<leader>Q', '<cmd>confirm qall<cr>', { desc = 'Quit all' })

vim.keymap.set('n', '<leader>sc', function()
  vim.cmd('nohlsearch')
end, { desc = 'Search clear' })

-- Keep search/replace functionality
vim.keymap.set('n', '<leader>sr', function()
  -- Simple search and replace using built-in substitution
  local search_term = vim.fn.input('Search for: ')
  if search_term ~= '' then
    local replace_term = vim.fn.input('Replace with: ')
    local confirm = vim.fn.input('Replace all occurrences? (y/n): ')
    if confirm:lower() == 'y' then
      vim.cmd(string.format('%%s/%s/%s/g', vim.fn.escape(search_term, '/'), vim.fn.escape(replace_term, '/')))
    else
      vim.cmd(string.format('%%s/%s/%s/gc', vim.fn.escape(search_term, '/'), vim.fn.escape(replace_term, '/')))
    end
  end
end, { desc = 'Search replace' })
