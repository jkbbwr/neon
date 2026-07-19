; Text objects for Neon (nvim-treesitter-textobjects conventions).

(function_declaration
  body: (block) @function.inner) @function.outer

(lambda_expression
  body: (_) @function.inner) @function.outer

(test_declaration
  body: (block) @function.inner) @function.outer

(record_declaration
  body: (field_declaration_list) @class.inner) @class.outer

(protocol_declaration
  body: (protocol_body) @class.inner) @class.outer

(impl_declaration
  body: (impl_body) @class.inner) @class.outer

(module_declaration
  body: (module_body) @class.inner) @class.outer

(parameter) @parameter.inner
(lambda_parameter) @parameter.inner
(field_declaration) @parameter.inner
(field_initializer) @parameter.inner

(arguments
  (_) @parameter.inner)

(match_arm) @conditional.inner

(if_expression
  consequence: (block) @conditional.inner) @conditional.outer

(match_expression
  body: (match_body) @conditional.inner) @conditional.outer

(while_expression
  body: (block) @loop.inner) @loop.outer

(for_expression
  body: (block) @loop.inner) @loop.outer

(loop_expression
  body: (block) @loop.inner) @loop.outer

[
  (line_comment)
  (doc_comment)
  (block_comment)
] @comment.outer

(call_expression) @call.outer

(call_expression
  arguments: (arguments) @call.inner)
