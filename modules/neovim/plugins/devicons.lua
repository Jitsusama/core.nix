-- The marks files and folders wear in trouble, oil and the rest: each kind
-- keeps its own mark, but every mark takes the text's colour, as the
-- finder's do, rather than one each type brings: the name is the star, and
-- the mark only says what kind of thing it names.
local devicons = require('nvim-web-devicons')
devicons.setup({
  color_icons = false,
  override = {
    default_icon = vim.tbl_extend('force', devicons.get_default_icon(), {
      color = require('theme').text,
    }),
  },
})
