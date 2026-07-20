; Syntax highlighting for Neon.
;
; Later patterns win over earlier ones, so this file goes general -> specific:
; identifiers are captured broadly first, then narrowed by position. Nothing
; here may be reordered casually -- the whole file is one long argument about
; precedence, and a pattern moved above the one that used to override it changes
; the colour of something with no other visible sign.
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
;
; The name set is closed. `expand.rs`'s `lookup()` maps exactly these six and
; `run()` reports anything else as "unknown annotation" -- an error, not a
; no-op, so the file will not compile. Colouring an unrecognised `@name` as an
; error is therefore not a guess; it is the same answer the compiler gives, just
; sooner.
;
; KEEP THIS LIST IN STEP WITH `lookup()`. That instruction was already here and
; was still not enough: `inline` was added to the compiler, nothing here noticed,
; and the five `@inline` uses in `stdlib/std/collections/list.neon` rendered as
; errors -- correct-looking, confidently red, and wrong. The failure is silent in
; both directions, because a stale list neither fails to compile nor fails a test.

(annotation
  "@" @attribute
  name: (identifier) @attribute
  (#any-of? @attribute "native" "cfg" "doc" "runtime" "pure" "inline"))

(annotation
  name: (identifier) @error
  (#not-any-of? @error "native" "cfg" "doc" "runtime" "pure" "inline"))

; ---- types ----------------------------------------------------------------
;
; Two patterns per context, because a type is spelled either as a bare name or
; as a path: `Reader` and `std::io::Reader`. For the path form only the final
; segment is the type -- the anchor `.` after the captured child is what pins it
; to last -- and the leading segments are picked up as `@module` further down.
; Without the path half, every qualified type name in the file stayed
; `@variable`, which is how `x: std::io::Reader` came out looking like an
; expression.

(any_type) @type.builtin

; `List[i64]`, `std::collections::Map[str, i64]`.
(generic_type
  name: (identifier) @type)
(generic_type
  name: (path (identifier) @type .))

; Binding and declaration sites that carry an explicit type.
(parameter
  type: (identifier) @type)
(parameter
  type: (path (identifier) @type .))

(lambda_parameter
  type: (identifier) @type)
(lambda_parameter
  type: (path (identifier) @type .))

(field_declaration
  type: (identifier) @type)
(field_declaration
  type: (path (identifier) @type .))

(let_statement
  type: (identifier) @type)
(let_statement
  type: (path (identifier) @type .))

(const_declaration
  type: (identifier) @type)
(const_declaration
  type: (path (identifier) @type .))

(function_declaration
  return_type: (identifier) @type)
(function_declaration
  return_type: (path (identifier) @type .))

; The `-> C` of a function *type*, which is a different node from the `-> C` of
; a function *declaration* and was going uncoloured because of it.
(function_type
  return_type: (identifier) @type)
(function_type
  return_type: (path (identifier) @type .))

(where_bound
  bound: (identifier) @type)
(where_bound
  bound: (path (identifier) @type .))

(is_expression
  type: (identifier) @type)
(is_expression
  type: (path (identifier) @type .))

(as_expression
  type: (identifier) @type)
(as_expression
  type: (path (identifier) @type .))

(is_pattern
  type: (identifier) @type)
(is_pattern
  type: (path (identifier) @type .))

; The right-hand side of an alias: `type A = B`, `mu type A = B`, `newtype A = B`.
; The alias name was already `@type.definition`; the thing it names was not.
(type_alias_declaration
  value: (identifier) @type)
(type_alias_declaration
  value: (path (identifier) @type .))

(mu_type_declaration
  value: (identifier) @type)
(mu_type_declaration
  value: (path (identifier) @type .))

(newtype_declaration
  value: (identifier) @type)
(newtype_declaration
  value: (path (identifier) @type .))

; Type combinators and bracketed lists. These have no field names -- the
; operands are ordinary children -- so the pattern is on the parent node.
(union_type (identifier) @type)
(union_type (path (identifier) @type .))
(intersection_type (identifier) @type)
(intersection_type (path (identifier) @type .))
(negated_type (identifier) @type)
(negated_type (path (identifier) @type .))
(function_type_parameters (identifier) @type)
(function_type_parameters (path (identifier) @type .))
(type_arguments (identifier) @type)
(type_arguments (path (identifier) @type .))
(turbofish_arguments (identifier) @type)
(turbofish_arguments (path (identifier) @type .))
(tuple_type (identifier) @type)
(tuple_type (path (identifier) @type .))
(parenthesized_type (identifier) @type)
(parenthesized_type (path (identifier) @type .))

; `throws` has its own parallel type grammar (grammar.js `_throws_type`, kept
; separate so `fn f() throws (str) -> i64` cannot read as throwing a function),
; and every one of those nodes is a distinct node type. Missing them left an
; entire language feature -- the whole error-type vocabulary -- as `@variable`.
(throws_clause
  type: (identifier) @type)
(throws_clause
  type: (path (identifier) @type .))
(throws_union_type (identifier) @type)
(throws_union_type (path (identifier) @type .))
(throws_intersection_type (identifier) @type)
(throws_intersection_type (path (identifier) @type .))
(throws_negated_type (identifier) @type)
(throws_negated_type (path (identifier) @type .))
(throws_tuple_type (identifier) @type)
(throws_tuple_type (path (identifier) @type .))
(throws_parenthesized_type (identifier) @type)
(throws_parenthesized_type (path (identifier) @type .))

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
(impl_declaration target: (path (identifier) @type .))

; ---- built-in type names --------------------------------------------------
;
; `i64`, `f64`, `str` and `bool` are ordinary identifiers to the grammar -- only
; `any` is a keyword with its own node -- so nothing but the spelling
; distinguishes them, and a predicate is the only tool available.
;
; This is deliberately not restricted to type position: repeating the thirty-odd
; patterns above with a `#any-of?` bolted on would be thirty more chances to
; forget one when a type context is added. The price is that a *value* named
; `str` also renders as a builtin type. That is a name no Neon program should
; want, and the VS Code TextMate grammar makes exactly the same trade. Every
; binding site that follows -- parameters, fields, functions -- re-captures its
; own name, so a *definition* spelled `str` still wins.

((identifier) @type.builtin
  (#any-of? @type.builtin "i64" "f64" "str" "bool"))

; ---- functions ------------------------------------------------------------

(function_declaration
  name: (identifier) @function)

; A `fn` inside an `impl` or a `protocol` is a method, and reads very
; differently from a free function at a glance. Only the *call* side had this
; distinction before, so definition and use were coloured inconsistently.
(impl_body
  (function_declaration
    name: (identifier) @function.method))

(protocol_body
  (function_declaration
    name: (identifier) @function.method))

(call_expression
  function: (identifier) @function.call)

(call_expression
  function: (field_expression
    field: (identifier) @function.method.call))

; `a::b::c(x)` -- only the last segment is the function.
(call_expression
  function: (path
    (identifier) @function.call .))

; `test "name" { }` and `bench "name" { }` declare something and deserve to look
; like it. The capture is on the content and not on the `string` node, so the
; quotes stay punctuation and only the name takes the definition colour.
(test_declaration
  name: (string (string_content) @function))

; ---- constructors ---------------------------------------------------------
;
; `Point { x: 1 }` names a record type, but in the act of *building* one. These
; were `@type`, which is not wrong so much as flat: a construction site and a
; type annotation are the two things a reader most wants to tell apart at speed.

(record_literal path: (identifier) @constructor)
(record_literal path: (path (identifier) @constructor .))
(record_pattern path: (identifier) @constructor)
(record_pattern path: (path (identifier) @constructor .))

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

; `_` discards. It is not a variable and certainly not a builtin one -- that
; name means `self`/`this` -- it is a placeholder token, which is what
; `@character.special` is for.
(wildcard_pattern) @character.special

; ---- errors ---------------------------------------------------------------
;
; Last, so it wins outright. The grammar only produces an `ERROR` node where the
; lexer and the compiler also disagree with the text, and the two that occur in
; practice are exactly the ones the VS Code grammar flags by hand: an escape
; that is not one of `\n \r \t \0 \\ \" \' \# \xNN \u{...}` (grammar.js
; `ESCAPE`), and a rune literal holding something other than one character or
; one escape. Both fall out of the token definitions rather than needing a rule
; of their own here.

(ERROR) @error
