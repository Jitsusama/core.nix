local separators = {
  arrow_left = vim.fn.nr2char(0xe0b0),
  arrow_right = vim.fn.nr2char(0xe0b2),
  thin_arrow_left = vim.fn.nr2char(0xe0b1),
  thin_arrow_right = vim.fn.nr2char(0xe0b3),
  round_left = vim.fn.nr2char(0xe0b4),
  round_right = vim.fn.nr2char(0xe0b6),
}

local icons = {
  chevron_right = vim.fn.nr2char(0xe0b1),
  claude_asterisk = vim.fn.nr2char(0xf069),
  exclamation_triangle = vim.fn.nr2char(0xf071),
  folder = vim.fn.nr2char(0xf07b),
  fork = vim.fn.nr2char(0xe0a0),
  info_circle = vim.fn.nr2char(0xf05a),
  lightbulb = vim.fn.nr2char(0xf0eb),
  magnifying_glass = vim.fn.nr2char(0xf002),
  overlapping_squares = vim.fn.nr2char(0xf0c5),
  padlock = vim.fn.nr2char(0xf023),
  pencil = vim.fn.nr2char(0xf044),
  stacked_bars = vim.fn.nr2char(0xf233),
  stacked_lines = vim.fn.nr2char(0xf0c9),
  times_circle = vim.fn.nr2char(0xf00d),
}

-- Helper function to get the full root directory path
local function get_root_path()
  local root = vim.fn.getcwd()
  -- Try to get git root
  local git_root = vim.fn.system('git rev-parse --show-toplevel 2>/dev/null'):gsub('\n', '')
  if vim.v.shell_error == 0 and git_root ~= '' then
    root = git_root
  end
  return root
end

-- Helper function to get the root directory basename
local function get_root()
  return vim.fn.fnamemodify(get_root_path(), ':t')
end

-- Helper function to check if LSP is active
local function has_lsp()
  local clients = vim.lsp.get_clients({ bufnr = 0 })
  return #clients > 0
end

require('lualine').setup({
  options = {
    icons_enabled = true,
    theme = 'gruvbox',
    component_separators = { left = separators.thin_arrow_left, right = separators.thin_arrow_right },
    section_separators = { left = separators.arrow_left, right = separators.arrow_right },
    disabled_filetypes = {
      statusline = { 'dashboard', 'alpha', 'ministarter' },
    },
    always_divide_middle = true,
    globalstatus = true,
  },
  sections = {
    lualine_a = {
      { 'mode' },
    },
    lualine_b = {
      {
        function()
          return icons.stacked_bars
        end,
        cond = has_lsp,
        padding = { left = 1, right = 2 },
      },
      {
        function()
          local ok, claudecode = pcall(require, 'claudecode')
          if ok and claudecode.is_claude_connected and claudecode.is_claude_connected() then
            return icons.claude_asterisk
          end
          return ''
        end,
        color = { fg = '#FF8C42' }, -- Claude's orange color
      },
      {
        'diagnostics',
        symbols = {
          error = icons.times_circle .. ' ',
          warn = icons.exclamation_triangle .. ' ',
          hint = icons.lightbulb .. ' ',
          info = icons.info_circle .. ' ',
        },
      },
    },
    lualine_c = {
      {
        'filetype',
        icon_only = true,
        separator = '',
        padding = { left = 1, right = 0 },
      },
      {
        'filename',
        path = 1, -- Show relative path
        symbols = {
          modified = icons.pencil,
          readonly = icons.padlock,
          unnamed = ' [No Name]',
          newfile = ' [New]',
        },
      },
    },
    lualine_x = {},
    lualine_y = {
      {
        'searchcount',
        maxcount = 999,
        timeout = 500,
        icon = icons.magnifying_glass,
        fmt = function(str)
          if str == '' then
            return ''
          end
          return str:gsub('%[', ''):gsub('%]', '')
        end,
      },
      {
        'selectioncount',
        icon = icons.stacked_lines,
        fmt = function(str)
          if str == '' then
            return ''
          end
          return str
        end,
      },
    },
    lualine_z = {
      { 'location' },
    },
  },
  inactive_sections = {
    lualine_a = {},
    lualine_b = {},
    lualine_c = { 'filename' },
    lualine_x = { 'location' },
    lualine_y = {},
    lualine_z = {},
  },
  tabline = {
    lualine_a = {
      { get_root, icon = icons.folder },
    },
    lualine_b = {
      { 'branch', icon = icons.fork },
      {
        'diff',
        symbols = {
          added = '+',
          modified = '~',
          removed = '-',
        },
      },
    },
    lualine_c = {
      {
        'aerial',
        sep = ' ' .. icons.chevron_right .. ' ',
        sep_highlight = 'Normal',
        padding = { left = 1, right = 1 },
      },
    },
    lualine_x = {},
    lualine_y = {},
    lualine_z = {},
  },
  extensions = {},
})
