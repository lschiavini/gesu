" Vim syntax file for Gesù. Works in Vim and Neovim.
if exists("b:current_syntax")
  finish
endif

syn case match

syn keyword gesuTodo TODO FIXME NOTE contained
syn match gesuComment "--.*$" contains=gesuTodo,@Spell

" Names: a capital starts a kind or a case, all capitals is a law.
syn match gesuCall "\<\h\w*\ze\s*("
syn match gesuKindName "\<\u\w*\>"
syn match gesuLaw "\<\u[A-Z0-9_]\+\>"
syn match gesuParableName "\%(\<parable\s\+\)\@<=\h\w*"

syn match gesuNumber "\<\d[0-9_]*\>"
syn match gesuDecimal "\<\d[0-9_]*\.\d\+\>"
syn match gesuArrow "->"

syn keyword gesuTruth Jesus False
syn keyword gesuHeresy True true TRUE
syn keyword gesuConditional if otherwise discern
syn keyword gesuRepeat while repeat times for each in break continue
syn keyword gesuException tempt forgive sin may
syn keyword gesuStatement let law give preach kind pagan baptize as
syn keyword gesuAmen amen
syn keyword gesuDefine parable
syn keyword gesuStorage mortal lend entrust
syn keyword gesuOperator deny and or is not
syn keyword gesuType Number Decimal Text Truth Maybe Pagan Nothing
syn keyword gesuConstant Found Lost
syn keyword gesuBuiltin length push pop get lookup remove keys copy range sort reverse
" `contains` is an option name to :syn keyword, so it is matched instead.
syn match gesuBuiltin "\<contains\>"
syn keyword gesuBuiltin split join trim upper lower replace chars text number decimal floor round
syn keyword gesuBuiltin abs min max read_file write_file read_line args parse_json to_json

syn region gesuString start=+"+ skip=+\\.+ end=+"+ contains=gesuInterp,gesuEscape,@Spell
syn region gesuInterp matchgroup=gesuInterpDelim start="{" end="}" contained contains=TOP
syn match gesuEscape +\\[nt"\\{}]+ contained

hi def link gesuTodo Todo
hi def link gesuComment Comment
hi def link gesuCall Function
hi def link gesuKindName Type
hi def link gesuLaw Constant
hi def link gesuParableName Function
hi def link gesuNumber Number
hi def link gesuDecimal Float
hi def link gesuArrow Operator
hi def link gesuTruth Boolean
hi def link gesuHeresy Error
hi def link gesuConditional Conditional
hi def link gesuRepeat Repeat
hi def link gesuException Exception
hi def link gesuStatement Statement
hi def link gesuAmen Keyword
hi def link gesuDefine Define
hi def link gesuStorage StorageClass
hi def link gesuOperator Operator
hi def link gesuType Type
hi def link gesuConstant Constant
hi def link gesuBuiltin Function
hi def link gesuString String
hi def link gesuInterpDelim Special
hi def link gesuEscape SpecialChar

let b:current_syntax = "gesu"
