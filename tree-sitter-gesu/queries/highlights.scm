; Highlights for Gesù. General patterns come first: where two match the
; same node, the later one wins (Neovim, Zed, and Helix 25.07 or newer).

(identifier) @variable

; A capital starts a kind or a case; all capitals is a law.
((identifier) @type
  (#match? @type "^[A-Z]"))
((identifier) @constant
  (#match? @constant "^[A-Z][A-Z0-9_]+$"))

(comment) @comment @spell

(string) @string
(escape) @string.escape
(interpolation) @none

(number) @number
(decimal) @number.float
(truth) @boolean

["let" "law" "preach" "give" "kind" "pagan" "baptize" "as"] @keyword
(amen) @keyword
"parable" @keyword.function
["if" "otherwise" "discern"] @keyword.conditional
["while" "repeat" "times" "for" "each" "in" "break" "continue"] @keyword.repeat
["tempt" "forgive" "sin" "may"] @keyword.exception
["mortal" "lend" "entrust"] @keyword.modifier
["deny" "and" "or" "is" "not"] @keyword.operator

["+" "-" "*" "/" "%" "<" ">" "<=" ">=" "=" "->"] @operator
["(" ")" "[" "]" "{" "}"] @punctuation.bracket
["," ":" "."] @punctuation.delimiter
(interpolation
  "{" @punctuation.special
  "}" @punctuation.special)

; Types
(type_name (identifier) @type)
((type_name (identifier) @type.builtin)
  (#any-of? @type.builtin "Number" "Decimal" "Text" "Truth" "Pagan" "Nothing"))
"Maybe" @type.builtin
(kind name: (identifier) @type.definition)

; Parables and calls
(parable name: (identifier) @function)
(parameter name: (identifier) @variable.parameter)
(call parable: (identifier) @function.call)
(call parable: (member field: (identifier) @function.method.call))
((call parable: (identifier) @constructor)
  (#match? @constructor "^[A-Z]"))
((call parable: (identifier) @function.builtin)
  (#any-of? @function.builtin
    "length" "push" "pop" "get" "lookup" "contains" "remove" "keys" "copy" "range" "sort" "reverse"
    "split" "join" "trim" "upper" "lower" "replace" "chars" "text" "number" "decimal" "floor" "round"
    "abs" "min" "max" "read_file" "write_file" "read_line" "args" "parse_json" "to_json"))

; Fields and cases
(member field: (identifier) @property)
(field name: (identifier) @property)
(argument name: (identifier) @property)
(case name: (identifier) @constructor)
(pattern case: (identifier) @constructor)
((identifier) @constant.builtin
  (#any-of? @constant.builtin "Found" "Lost"))

(law_statement name: (identifier) @constant)
(pagan_statement name: (identifier) @module)

; There is only one Truth.
((identifier) @comment.error
  (#any-of? @comment.error "True" "true" "TRUE"))
