-- Load modes.nvim with error handling
local ok, modes = pcall(require, 'modes')
if not ok then
  vim.notify('Failed to load modes.nvim', vim.log.levels.ERROR)
  return
end

-- Get colors from current colorscheme highlights
-- This dynamically pulls colors from gruvbox highlight groups
local function get_theme_colors()
  -- Helper to extract color from highlight group
  local function get_hl_color(group, attr)
    local hl = vim.api.nvim_get_hl(0, { name = group })
    if attr == 'fg' then
      return hl.fg and string.format('#%06x', hl.fg) or nil
    elseif attr == 'bg' then
      return hl.bg and string.format('#%06x', hl.bg) or nil
    end
  end

  -- Get colors from gruvbox highlight groups
  -- These groups are defined by gruvbox.nvim
  return {
    yellow = get_hl_color('GruvboxYellow', 'fg') or '#fabd2f',
    red = get_hl_color('GruvboxRed', 'fg') or '#fb4934',
    aqua = get_hl_color('GruvboxAqua', 'fg') or '#8ec07c',
    purple = get_hl_color('GruvboxPurple', 'fg') or '#d3869b',
    blue = get_hl_color('GruvboxBlue', 'fg') or '#83a598',
    orange = get_hl_color('GruvboxOrange', 'fg') or '#fe8019',
    gray = get_hl_color('GruvboxGray', 'fg') or '#928374',
  }
end

-- Get the colors once at startup
local colors = get_theme_colors()

-- Configure modes.nvim
modes.setup({
  colors = {
    copy = colors.yellow, -- Yank operations
    delete = colors.red, -- Delete operations
    insert = colors.aqua, -- Insert mode
    visual = colors.purple, -- Visual mode
    replace = colors.blue, -- Replace mode
    change = colors.orange, -- Change operations (different from delete)
    format = colors.gray, -- Format operations (=, >, <, etc.)
  },
  ignore = {
    'NvimTree', -- Keep default
    'TelescopePrompt', -- Keep default
    'TelescopeResults', -- Keep default
    'lspinfo', -- Keep default
    'checkhealth', -- Keep default
    'help', -- Keep default
    'man', -- Keep default
    'oil', -- ADD: Oil.nvim file explorer
    'trouble', -- ADD: Trouble.nvim diagnostics
    'which-key', -- ADD: Which-key popups
    'fugitive', -- ADD: Git status windows
    'gitcommit', -- ADD: Git commit messages
    'octo', -- ADD: GitHub PR/issue buffers
  },
})
