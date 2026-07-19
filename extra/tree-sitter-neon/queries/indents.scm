; Indentation for Neon (nvim-treesitter conventions).
;
; Every construct that indents in Neon is brace-delimited, so this is almost
; entirely "@indent.begin on the node, @indent.branch on the closing brace".

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

; A comment should keep the indent it is already at rather than forcing one.
[
  (line_comment)
  (doc_comment)
  (block_comment)
] @indent.auto
