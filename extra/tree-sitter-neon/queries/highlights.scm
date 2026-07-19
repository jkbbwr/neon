; Syntax highlighting for Neon.
;
; Later patterns win over earlier ones, so this file goes general -> specific:
; identifiers are captured broadly first, then narrowed by position.
;
; Capture names are the common subset shared by Neon-supporting editors. See
; README.md for the two places Neovim and Zed diverge.

; ---- comments -------------------------------------------------------------

(line_comment) @comment
(block_comment) @comment
(doc_comment) @comment.documentation

; ---- the broad fallback ---------------------------------------------------
;
; Every later pattern narrows this one. It has to come first: Neovim and Zed
; both let a later pattern override an earlier one for the same node.

(identifier) @variable

; ---- literals -------------------------------------------------------------

(integer) @number
(float) @number.float
(rune) @character
(boolean) @constant.builtin
(null) @constant.builtin

; An atom is a value that is its own name: `:ok`, `:not_found`.
(atom) @constant

(string) @string
(escape_sequence) @string.escape

; `"#{...}"` -- the delimiters are punctuation, the hole is ordinary Neon.
(interpolation
  "#{" @punctuation.special
  "}" @punctuation.special)

; ---- keywords -------------------------------------------------------------

[
  "record"
  "opaque"
  "newtype"
  "type"
  "mu"
  "protocol"
  "marker"
  "impl"
  "where"
  "const"
  "internal"
  "orphan"
] @keyword

[
  "let"
] @keyword

"fn" @keyword.function

[
  "mod"
  "use"
  "as"
] @keyword.import

[
  "if"
  "else"
  "match"
] @keyword.conditional

[
  "loop"
  "while"
  "for"
  "in"
  "break"
] @keyword.repeat

; `continue` is the whole node -- it carries no operand, so there is no
; separate anonymous token to match.
(continue_expression) @keyword.repeat

"return" @keyword.return

[
  "throws"
  "throw"
  "try"
  "catch"
] @keyword.exception

[
  "test"
  "bench"
  "assert"
  "assert_eq"
  "assert_ne"
  "assert_throws"
] @keyword

; `is` tests a type, `as` casts to one -- both are operators spelled as words.
"is" @keyword.operator

(as_expression
  "as" @keyword.operator)

; ---- operators ------------------------------------------------------------

[
  "and"
  "or"
  "orelse"
  "band"
  "bor"
  "bxor"
  "bnot"
  "bsl"
  "bsr"
] @keyword.operator

[
  "+"
  "-"
  "*"
  "/"
  "%"
  "=="
  "!="
  "<"
  "<="
  ">"
  ">="
  "="
  "!"
  "|>"
  "->"
  "=>"
  ".."
  "&"
  "|"
  "?"
] @operator

; ---- punctuation ----------------------------------------------------------

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

[
  ","
  ":"
  "::"
  ";"
  "."
] @punctuation.delimiter

; ---- annotations ----------------------------------------------------------

(annotation
  "@" @attribute
  name: (identifier) @attribute)

; ---- types ----------------------------------------------------------------

; A bare name in type position. The narrower patterns below re-capture the
; parts that are really something else (a module path, a call).
(any_type) @type.builtin

(generic_type
  name: (identifier) @type)

(generic_type
  name: (path (identifier) @type .))

(parameter
  type: (identifier) @type)

(field_declaration
  type: (identifier) @type)

(function_declaration
  return_type: (identifier) @type)

(where_bound
  bound: (identifier) @type)

(is_expression
  type: (identifier) @type)

(as_expression
  type: (identifier) @type)

(is_pattern
  type: (identifier) @type)

(union_type (identifier) @type)
(intersection_type (identifier) @type)
(negated_type (identifier) @type)
(function_type_parameters (identifier) @type)
(type_arguments (identifier) @type)
(turbofish_arguments (identifier) @type)
(tuple_type (identifier) @type)
(parenthesized_type (identifier) @type)

(type_parameters (identifier) @type.parameter)
(where_bound parameter: (identifier) @type.parameter)
(protocol_subject name: (identifier) @type)

; Declaration sites.
(record_declaration name: (identifier) @type.definition)
(type_alias_declaration name: (identifier) @type.definition)
(mu_type_declaration name: (identifier) @type.definition)
(newtype_declaration name: (identifier) @type.definition)
(marker_declaration name: (identifier) @type.definition)
(protocol_declaration name: (identifier) @type.definition)
(impl_declaration protocol: (identifier) @type)
(impl_declaration protocol: (path (identifier) @type .))
(impl_declaration target: (identifier) @type)

; `Point { x: 1 }` and the matching pattern -- the path names a record type.
(record_literal path: (identifier) @type)
(record_literal path: (path (identifier) @type .))
(record_pattern path: (identifier) @type)
(record_pattern path: (path (identifier) @type .))

; ---- functions ------------------------------------------------------------

(function_declaration
  name: (identifier) @function)

(call_expression
  function: (identifier) @function.call)

(call_expression
  function: (field_expression
    field: (identifier) @function.method.call))

; `a::b::c(x)` -- only the last segment is the function.
(call_expression
  function: (path
    (identifier) @function.call .))

; ---- variables and members ------------------------------------------------

(parameter
  name: (identifier) @variable.parameter)

(lambda_parameter
  name: (identifier) @variable.parameter)

(catch_clause
  binding: (identifier) @variable.parameter)

(field_declaration
  name: (identifier) @variable.member)

(field_initializer
  name: (identifier) @variable.member)

(field_pattern
  name: (identifier) @variable.member)

(field_expression
  field: (identifier) @variable.member)

; A module path's leading segments: `std::io::println`.
(path
  (identifier) @module
  (identifier))

(use_tree
  prefix: (identifier) @module)

(module_declaration
  name: (identifier) @module)

(wildcard_pattern) @variable.builtin
