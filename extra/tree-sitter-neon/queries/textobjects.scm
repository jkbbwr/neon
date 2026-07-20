; Text objects for Neon (nvim-treesitter-textobjects conventions).
;
; `.inner` is what you would want to retype, `.outer` is what you would want to
; delete. Where those differ the pair is written out explicitly rather than
; letting one stand in for both.

; ---- functions ------------------------------------------------------------

(function_declaration
  body: (block) @function.inner) @function.outer

(lambda_expression
  body: (_) @function.inner) @function.outer

(test_declaration
  body: (block) @function.inner) @function.outer

; ---- types and containers -------------------------------------------------

(record_declaration
  body: (field_declaration_list) @class.inner) @class.outer

(protocol_declaration
  body: (protocol_body) @class.inner) @class.outer

(impl_declaration
  body: (impl_body) @class.inner) @class.outer

(module_declaration
  body: (module_body) @class.inner) @class.outer

; ---- parameters -----------------------------------------------------------
;
; `.inner` is the parameter alone. `.outer` has to reach the separator, or
; `daa` on the middle of `f(a, b, c)` leaves `f(a, , c)` behind. Two patterns,
; because the separator is on the right for every item but the last, where the
; only one available is on the left.

[
  (parameter)
  (lambda_parameter)
  (field_declaration)
  (field_initializer)
] @parameter.inner

(arguments
  (_) @parameter.inner)

(type_arguments
  (_) @parameter.inner)

(turbofish_arguments
  (_) @parameter.inner)

; Item followed by a comma: take the comma. Written against the anonymous `,`
; rather than against a specific list node, so it covers arguments, parameters,
; list elements, type arguments and record fields in one pattern.
(
  (_) @parameter.inner
  .
  "," @_separator
  (#make-range! "parameter.outer" @parameter.inner @_separator)
)

; Last item, so there is no comma after it: take the one before instead. The
; anchors force adjacency on both sides, so this only fires on a genuine final
; item and never overlaps the pattern above.
(
  "," @_separator
  .
  (_) @parameter.inner
  .
  [")" "]" "}"]
  (#make-range! "parameter.outer" @_separator @parameter.inner)
)

; A sole item has no separator on either side, so inner and outer coincide.
;
; This is the one case that needs its container named. A bare `(_)` between two
; brackets would also match the contents of `(x)` and of a one-statement block,
; and because nvim-treesitter-textobjects resolves overlapping captures by
; picking the *smallest* range containing the cursor, a stray `@parameter.outer`
; anywhere would silently win over the two comma-inclusive ranges above and undo
; the whole point of them.
(parameters "(" . (parameter) @parameter.inner . ")"
  (#make-range! "parameter.outer" @parameter.inner @parameter.inner))

(lambda_parameters "(" . (lambda_parameter) @parameter.inner . ")"
  (#make-range! "parameter.outer" @parameter.inner @parameter.inner))

(arguments "(" . (_) @parameter.inner . ")"
  (#make-range! "parameter.outer" @parameter.inner @parameter.inner))

(field_declaration_list "{" . (field_declaration) @parameter.inner . "}"
  (#make-range! "parameter.outer" @parameter.inner @parameter.inner))

(list_expression "[" . (_) @parameter.inner . "]"
  (#make-range! "parameter.outer" @parameter.inner @parameter.inner))

(type_arguments "[" . (_) @parameter.inner . "]"
  (#make-range! "parameter.outer" @parameter.inner @parameter.inner))

(turbofish_arguments "[" . (_) @parameter.inner . "]"
  (#make-range! "parameter.outer" @parameter.inner @parameter.inner))

; ---- assignments ----------------------------------------------------------
;
; `let` and bare assignment are different nodes but the same text object: the
; whole binding is `.outer`, the value alone is `.inner`, and `.lhs`/`.rhs` are
; the two halves for swapping them.

(let_statement
  pattern: (_) @assignment.lhs
  value: (_) @assignment.rhs @assignment.inner) @assignment.outer

(assignment_statement
  left: (_) @assignment.lhs
  right: (_) @assignment.rhs @assignment.inner) @assignment.outer

(const_declaration
  name: (_) @assignment.lhs
  value: (_) @assignment.rhs @assignment.inner) @assignment.outer

; ---- returns --------------------------------------------------------------
;
; `.inner` is the returned value. `return` with no operand has no inner half,
; which is why the operand is a separate pattern instead of a field on the one
; above -- an optional child in a single pattern would make the whole match
; fail on a bare `return`.

(return_expression) @return.outer

(return_expression
  (_) @return.inner)

; ---- blocks and statements ------------------------------------------------

(block) @block.outer

; `.inner` is the contents without the braces. `#make-range!` between the two
; brace tokens would include them; anchoring to the first and last child inside
; is not expressible, so the block's own statements stand in and an empty block
; simply has no inner range.
(block
  (_) @block.inner)

[
  (let_statement)
  (assignment_statement)
  (expression_statement)
] @statement.outer

; ---- conditionals ---------------------------------------------------------

(match_arm) @conditional.inner

(if_expression
  consequence: (block) @conditional.inner) @conditional.outer

; The `else` half. Without this, `]i` from inside an `else` jumped to the `if`
; branch, because that was the only `@conditional.inner` in range. An `else if`
; captures its own `alternative` in turn, so a chain steps through one link at a
; time rather than treating the whole tail as one object.
(if_expression
  alternative: (block) @conditional.inner)

(if_expression
  alternative: (if_expression) @conditional.outer)

(match_expression
  body: (match_body) @conditional.inner) @conditional.outer

; ---- loops ----------------------------------------------------------------

(while_expression
  body: (block) @loop.inner) @loop.outer

(for_expression
  body: (block) @loop.inner) @loop.outer

(loop_expression
  body: (block) @loop.inner) @loop.outer

; ---- comments -------------------------------------------------------------

[
  (line_comment)
  (doc_comment)
  (block_comment)
] @comment.outer

; ---- calls ----------------------------------------------------------------

(call_expression) @call.outer

(call_expression
  arguments: (arguments) @call.inner)

; ---- numbers --------------------------------------------------------------
;
; No `.outer`: a numeric literal has no surrounding syntax to include, so the
; two would be the same range and the convention is to define only `.inner`.

[
  (integer)
  (float)
] @number.inner
