; Indentation for Neon (nvim-treesitter conventions).
;
; Two halves. The first is the easy one: every construct that opens a body in
; Neon is bracket-delimited, so it is "@indent.begin on the node, @indent.branch
; on the closing bracket". The second half is continuation lines -- an
; expression that ran onto the next line without opening anything -- which have
; no bracket to hang off and need a node that already spans them.

; ---- bracketed bodies -----------------------------------------------------

[
  (block)
  (field_declaration_list)
  (structural_type)
  (protocol_body)
  (impl_body)
  (module_body)
  (match_body)
  (record_literal)
  (use_group)
  (list_expression)
  (parameters)
  (arguments)
  (lambda_parameters)
  (tuple_expression)
  (tuple_type)
  (tuple_pattern)
  (function_type_parameters)
  (type_parameters)
  (type_arguments)
  (turbofish_arguments)
  (parenthesized_expression)
  (parenthesized_type)
] @indent.begin

[
  "}"
  ")"
  "]"
] @indent.branch @indent.end

; ---- continuation lines ---------------------------------------------------
;
; A wrapped expression should sit one level in from the line it continues.
; nvim-treesitter counts at most one `@indent.begin` per *starting row*, which is
; what makes the nodes below safe to capture wholesale despite nesting: a
; left-nested chain like
;
;     xs
;       |> map(f)
;       |> filter(g)
;
; is three `binary_expression`s, but all three begin at `xs`, on one row, so the
; pipeline gets one level of indent and not three. The same dedup is why a
; `let` whose value is a block does not double-indent the block: `let_statement`
; and `block` both start on the `let` row.

[
  ; `a +` / `|> f(x)` / `and ...` continued on the next line.
  (binary_expression)

  ; `let x =` or `xs[i] =` with the value on the following line.
  (let_statement)
  (assignment_statement)
  (const_declaration)

  ; A method chain broken before the dot: `client` / `.get(url)` / `.body()`.
  (field_expression)

  ; `where T: Ord,` / `U: Show` across several lines.
  (where_clause)

  ; A match arm whose value runs past the `=>`.
  (match_arm)

  ; A lambda whose body is on the next line, and a `try`/`throw`/`return` the
  ; same. All are prefix forms that start on the row being continued from.
  (lambda_expression)
  (try_expression)
  (throw_expression)
  (return_expression)

  ; A union or intersection type written one alternative per line.
  (union_type)
  (intersection_type)
  (throws_union_type)
  (throws_intersection_type)
] @indent.begin

; A `->` return type moved onto its own line is deliberately NOT handled. The
; only node spanning both the signature and that line is `function_declaration`,
; and capturing it would put every function *body* two levels in whenever the
; opening brace starts a row of its own -- the row-dedup above cannot merge two
; different starting rows. A wrong indent on every function is a far worse trade
; than no indent on a rare line break.

; A comment should keep the indent it is already at rather than forcing one.
[
  (line_comment)
  (doc_comment)
  (block_comment)
] @indent.auto
