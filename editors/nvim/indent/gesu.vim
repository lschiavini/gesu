" Indentation for Gesù: a block opens after its first line and closes at
" `amen`, `otherwise` or `forgive`.
if exists("b:did_indent")
  finish
endif
let b:did_indent = 1

setlocal indentexpr=GesuIndent()
setlocal indentkeys=o,O,0=amen,0=otherwise,0=forgive
setlocal autoindent

if exists("*GesuIndent")
  finish
endif

function! GesuIndent() abort
  let prev = prevnonblank(v:lnum - 1)
  if prev == 0
    return 0
  endif
  let ind = indent(prev)
  let before = substitute(getline(prev), '\s*--.*$', '', '')
  let opens = '^\s*\%(if\|otherwise\|while\|repeat\|for\|tempt\|forgive\|discern\|parable\|kind\)\>'
  let arm = '^\s*\u\w*\%(([^)]*)\)\=:\s*$'
  if before =~# opens || before =~# arm
    let ind += shiftwidth()
  endif
  if getline(v:lnum) =~# '^\s*\%(amen\|otherwise\|forgive\)\>'
    let ind -= shiftwidth()
  endif
  return ind < 0 ? 0 : ind
endfunction
