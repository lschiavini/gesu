; Highlights for Gesù in Zed. Made from tree-sitter-gesu/queries/highlights.scm
; with Zed's capture names. Later patterns win.

(identifier) @variable

; A capital starts a kind or a case; all capitals is a law.
((identifier) @type
  (#match? @type "^[A-Z]"))
((identifier) @constant
  (#match? @constant "^[A-Z][A-Z0-9_]+$"))

(comment) @comment

(string) @string
(escape) @string.escape
(interpolation) @embedded

(number) @number
(decimal) @number
(truth) @boolean

["let" "law" "preach" "give" "kind" "pagan" "baptize" "as"] @keyword
(amen) @keyword
"parable" @keyword
["if" "otherwise" "discern"] @keyword
["while" "repeat" "times" "for" "each" "in" "break" "continue"] @keyword
["tempt" "forgive" "sin" "may" "repent"] @keyword
["mortal" "lend" "entrust"] @keyword
["deny" "and" "or" "is" "not"] @keyword

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
(kind name: (identifier) @type)

; Parables and calls
(parable name: (identifier) @function)
(parameter name: (identifier) @variable.parameter)
(call parable: (identifier) @function)
(call parable: (member field: (identifier) @function.method))
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
(pagan_statement name: (identifier) @variable.special)

; There is only one Truth.
((identifier) @hint
  (#any-of? @hint "True" "true" "TRUE"))
