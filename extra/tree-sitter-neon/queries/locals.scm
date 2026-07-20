; Local scopes, definitions and references for Neon.
;
; This drives "highlight the other occurrences of the name under the cursor",
; "jump to the definition of this local", and the smarter half of rename. It is
; deliberately a *lexical* approximation: nothing here knows about types,
; protocol dispatch or module resolution, so a reference that resolves to an
; import or a global simply finds no definition and is left alone. That is the
; correct failure mode -- a wrong definition is worse than none.
;
; Neon binds in more places than a C-family language does, which is why the
; definition list below is long: `let`, `for`, every `match` arm, a `catch`
; binding, lambda and function parameters, and the type parameters of six
; different declaration forms.

; ---- scopes ---------------------------------------------------------------
;
; A scope is anything that can shadow. `block` is the obvious one, but the
; declaration forms have to be scopes in their own right and not just via their
; bodies: a `fn`'s parameters and type parameters live *outside* its `block`, so
; scoping them to the block would make them invisible from the signature -- a
; `where` clause could not see the type parameter it bounds.

[
  (source_file)
  (module_body)
  (block)

  (function_declaration)
  (lambda_expression)

  ; Type parameters belong to the declaration, so the declaration is the scope.
  (record_declaration)
  (protocol_declaration)
  (impl_declaration)
  (type_alias_declaration)
  (mu_type_declaration)
  (newtype_declaration)

  ; Each arm binds its own pattern, and those bindings must not leak sideways
  ; into the next arm -- `match x { Some { v } => v, None { } => v }` should
  ; leave the second `v` unresolved, not point it at the first arm's binding.
  (match_arm)

  ; `for p in xs { }` binds `p` before the body, and `catch (e) { }` binds `e`
  ; the same way; both are outside the block they are visible in.
  (for_expression)
  (catch_clause)
] @local.scope

; ---- value definitions ----------------------------------------------------

; `let x = ...`. The pattern forms are enumerated rather than matched with a
; wildcard because `(_)` would also capture the `path:` of a record pattern and
; the type of an `is` pattern, neither of which binds anything.
(let_statement
  pattern: (identifier) @local.definition.var)

(let_statement
  pattern: (tuple_pattern
    (identifier) @local.definition.var))

(let_statement
  pattern: (record_pattern
    (field_pattern
      name: (identifier) @local.definition.var)))

; `Point { x: inner }` binds `inner`, not `x`; the pattern above would otherwise
; claim the field name, which is a member reference and not a binding at all.
(let_statement
  pattern: (record_pattern
    (field_pattern
      pattern: (identifier) @local.definition.var)))

(for_expression
  pattern: (identifier) @local.definition.var)

(for_expression
  pattern: (tuple_pattern
    (identifier) @local.definition.var))

(match_arm
  pattern: (identifier) @local.definition.var)

(match_arm
  pattern: (tuple_pattern
    (identifier) @local.definition.var))

(match_arm
  pattern: (record_pattern
    (field_pattern
      name: (identifier) @local.definition.var)))

(match_arm
  pattern: (record_pattern
    (field_pattern
      pattern: (identifier) @local.definition.var)))

(const_declaration
  name: (identifier) @local.definition.var)

; ---- parameters -----------------------------------------------------------

(parameter
  name: (identifier) @local.definition.parameter)

(lambda_parameter
  name: (identifier) @local.definition.parameter)

; `catch (e)` binds the thrown value for the length of the handler.
(catch_clause
  binding: (identifier) @local.definition.parameter)

; ---- functions ------------------------------------------------------------

(function_declaration
  name: (identifier) @local.definition.function)

; ---- types ----------------------------------------------------------------

(type_parameters
  (identifier) @local.definition.type)

(record_declaration
  name: (identifier) @local.definition.type)

(type_alias_declaration
  name: (identifier) @local.definition.type)

(mu_type_declaration
  name: (identifier) @local.definition.type)

(newtype_declaration
  name: (identifier) @local.definition.type)

(marker_declaration
  name: (identifier) @local.definition.type)

(protocol_declaration
  name: (identifier) @local.definition.type)

; `protocol Show for T` -- `T` is bound by the protocol header, exactly like a
; type parameter, and the `where` clause and every method signature can see it.
(protocol_subject
  name: (identifier) @local.definition.type)

; ---- fields ---------------------------------------------------------------

(field_declaration
  name: (identifier) @local.definition.field)

; ---- imports --------------------------------------------------------------
;
; `use a::b::c` binds `c`; `use a::b as z` binds `z` and not `b`. `use_tree` is
; flat (see grammar.js), so the two cases are separate patterns and the alias
; one comes second, which is what makes it win for `... as z`.

(use_tree
  name: (identifier) @local.definition.import)

(use_tree
  alias: (identifier) @local.definition.import)

(module_declaration
  name: (identifier) @local.definition.namespace)

; ---- references -----------------------------------------------------------
;
; Broad on purpose. A definition capture on the same node takes priority, so
; this does not need to carve out the binding sites; anything left over is a
; use, and a use that resolves to nothing is silently ignored.

(identifier) @local.reference
