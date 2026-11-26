-- Settings for Gesù buffers, and the language server when `gesu` is installed.
vim.bo.commentstring = "-- %s"
vim.bo.comments = ":--"
vim.bo.expandtab = true
vim.bo.shiftwidth = 4
vim.bo.softtabstop = 4
vim.bo.tabstop = 4

-- With the compiled parser (see build-parser.sh), tree-sitter colors the
-- buffer. Without it, the syntax file does.
pcall(vim.treesitter.start, 0, "gesu")

local command = vim.g.gesu_command or "gesu"
if vim.fn.executable(command) == 1 then
  vim.lsp.start({
    name = "gesu",
    cmd = { command, "lsp" },
    root_dir = vim.fs.dirname(vim.api.nvim_buf_get_name(0)),
  })

  -- `gesu fmt` on save. To turn it off: vim.g.gesu_format_on_save = false
  vim.api.nvim_create_autocmd("BufWritePre", {
    buffer = 0,
    group = vim.api.nvim_create_augroup("gesu_format_" .. vim.api.nvim_get_current_buf(), { clear = true }),
    callback = function(event)
      if vim.g.gesu_format_on_save ~= false then
        vim.lsp.buf.format({ bufnr = event.buf, name = "gesu", timeout_ms = 2000 })
      end
    end,
  })
end
