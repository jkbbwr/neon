/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

// Tree-sitter grammar for Neon.
//
// Derived from the compiler, not from intuition:
//   compiler/src/lexer/token.rs   -- the token alphabet and the reserved words
//   compiler/src/lexer/mod.rs     -- literal forms, comments, interpolation
//   compiler/src/parser/mod.rs    -- the grammar itself (chumsky)
//   compiler/src/ops.rs           -- the one precedence table
//
// The precedence numbers below are `ops::BINARY_OPS` verbatim. If that table
// changes, this must change with it.

const PREC = {
  // ops.rs, MIN_PREC..=MAX_PREC. Loosest first.
  orelse: 1,
  or: 2,
  and: 3,
  compare: 4,
  pipe: 5,
  bor: 6,
  bxor: 7,
  band: 8,
  shift: 9,
  add: 10,
  multiply: 11,

  // Above the binary ladder. `prefix_ops` folds over `postfix_ops`, so postfix
  // binds tighter: `-x.f` is `-(x.f)`.
  unary: 12,
  try: 13,
  postfix: 14,

  // Higher than anything an expression can reach: a block-like expression at
  // the start of a statement is a statement, never a left operand.
  statement_block: 30,

  // Type ladder: `!` tightest, then `&`, then `|`.
  union: 1,
  intersection: 2,
  negate: 3,
};

const IDENT = /[_a-zA-Z¡-￿][_0-9a-zA-Z¡-￿]*/;

// \n \r \t \0 \\ \" \' \# \xNN \u{NNNNNN} -- lexer/mod.rs `escape`.
const ESCAPE = /\\([nrt0\\"'#]|x[0-9a-fA-F]{2}|u\{[0-9a-fA-F]{1,6}\})/;

module.exports = grammar({
  name: 'neon',

  externals: ($) => [$.block_comment],

  extras: ($) => [/\s/, $.line_comment, $.doc_comment, $.block_comment],

  word: ($) => $.identifier,

  supertypes: ($) => [$._declaration, $._expression, $._type, $._pattern, $._statement],

  conflicts: ($) => [
    // `(x) => e` and `(x)`/`(x, y)`/`()` share every token up to the `)`; only
    // the `=>` tells them apart, which is why the parser tries `lambda` first.
    [$.lambda_parameters, $.unit_expression],
    [$.lambda_parameter, $._expression],
    // Inside `x[y]` a bare name is an expression if this is an index and a type
    // if it is a turbofish; only the token after the `]` decides.
    [$._expression, $._simple_type],
    // `while a { }` could read `a { }` as an empty record and then find no
    // body. The parser switches record literals off in condition position;
    // here GLR explores both and the record reading dies for want of a block,
    // which reaches the same answer. Parenthesise to get a record back.
    [$._expression, $.record_literal],
    // A block-like expression at the start of a statement is a statement, not
    // the left operand of what follows. `prec(1)` on `expression_statement`
    // picks the statement reading; the conflict is what lets GLR see both.
    [$._expression, $._expression_ending_with_block],
  ],

  rules: {
    source_file: ($) => repeat($._declaration),

    // ---- comments -------------------------------------------------------
    // `///` attaches to what follows; `////` is an ordinary line comment.
    // Block comments nest, which no regex can express -- see src/scanner.c.
    line_comment: (_) => token(choice(seq('//', /[^\/\n][^\n]*/), '//', seq('///', /\/[^\n]*/))),
    doc_comment: (_) => token(prec(1, choice(seq('///', /[^\/\n][^\n]*/), '///'))),

    // ---- declarations ---------------------------------------------------

    _declaration: ($) =>
      choice(
        $.function_declaration,
        $.record_declaration,
        $.protocol_declaration,
        $.marker_declaration,
        $.impl_declaration,
        $.mu_type_declaration,
        $.type_alias_declaration,
        $.newtype_declaration,
        $.use_declaration,
        $.module_declaration,
        $.const_declaration,
        $.test_declaration,
      ),

    // `@name` or `@name("literal")`. The argument is a plain string, never an
    // expression -- parser/mod.rs `annotations`.
    annotation: ($) =>
      seq('@', field('name', $.identifier), optional(seq('(', field('argument', $.string), ')'))),

    function_declaration: ($) =>
      seq(
        repeat($.annotation),
        'fn',
        field('name', $.identifier),
        optional(field('type_parameters', $.type_parameters)),
        field('parameters', $.parameters),
        // `throws E` comes before `->` and must not swallow it.
        optional(field('throws', $.throws_clause)),
        optional(seq('->', field('return_type', $._type))),
        optional(field('where', $.where_clause)),
        // Optional: a protocol method or an `@native` fn may stop at the signature.
        optional(field('body', $.block)),
      ),

    type_parameters: ($) => seq('[', commaSep1($.identifier), optional(','), ']'),

    parameters: ($) => seq('(', optional(seq(commaSep1($.parameter), optional(','))), ')'),

    parameter: ($) => seq(field('name', $.identifier), ':', field('type', $._type)),

    throws_clause: ($) => seq('throws', field('type', $._throws_type)),

    where_clause: ($) => seq('where', commaSep1($.where_bound)),

    where_bound: ($) => seq(field('parameter', $.identifier), ':', field('bound', $._type)),

    record_declaration: ($) =>
      seq(
        repeat($.annotation),
        optional('opaque'),
        'record',
        field('name', $.identifier),
        optional(field('type_parameters', $.type_parameters)),
        field('body', $.field_declaration_list),
      ),

    field_declaration_list: ($) =>
      seq('{', optional(seq(commaSep1($.field_declaration), optional(','))), '}'),

    field_declaration: ($) => seq(field('name', $.identifier), ':', field('type', $._type)),

    // `marker Ord` -- a bound with no methods.
    marker_declaration: ($) => seq(repeat($.annotation), 'marker', field('name', $.identifier)),

    protocol_declaration: ($) =>
      seq(
        repeat($.annotation),
        'protocol',
        field('name', $.identifier),
        'for',
        field('subject', $.protocol_subject),
        optional(field('where', $.where_clause)),
        field('body', $.protocol_body),
      ),

    // `for T` or `for C[_]` -- a type, or a type constructor of some arity.
    protocol_subject: ($) =>
      seq(field('name', $.identifier), optional(seq('[', commaSep1('_'), ']'))),

    protocol_body: ($) => seq('{', repeat($.function_declaration), '}'),

    impl_declaration: ($) =>
      seq(
        repeat($.annotation),
        // Contextual: `orphan` is only special immediately before `impl`.
        optional('orphan'),
        'impl',
        optional(field('type_parameters', $.type_parameters)),
        field('protocol', choice($.path, $.identifier)),
        'for',
        field('target', $._type),
        field('body', $.impl_body),
      ),

    impl_body: ($) => seq('{', repeat($.function_declaration), '}'),

    type_alias_declaration: ($) =>
      seq(
        'type',
        field('name', $.identifier),
        optional(field('type_parameters', $.type_parameters)),
        '=',
        field('value', $._type),
      ),

    mu_type_declaration: ($) =>
      seq(
        'mu',
        'type',
        field('name', $.identifier),
        optional(field('type_parameters', $.type_parameters)),
        '=',
        field('value', $._type),
      ),

    newtype_declaration: ($) =>
      seq(
        'newtype',
        field('name', $.identifier),
        optional(field('type_parameters', $.type_parameters)),
        '=',
        field('value', $._type),
      ),

    use_declaration: ($) => seq('use', field('tree', $.use_tree), optional(';')),

    // `a::b::c`, `a::b as z`, `a::*`, `a::{ b, c as d, sub::* }`.
    // Flattened deliberately: a separate `path` rule here would need two tokens
    // of lookahead to decide whether a segment is a prefix or the leaf.
    use_tree: ($) =>
      seq(
        repeat(seq(field('prefix', $.identifier), '::')),
        choice(
          field('glob', '*'),
          field('group', $.use_group),
          seq(field('name', $.identifier), optional(seq('as', field('alias', $.identifier)))),
        ),
      ),

    use_group: ($) => seq('{', optional(seq(commaSep1($.use_tree), optional(','))), '}'),

    module_declaration: ($) =>
      seq(
        repeat($.annotation),
        optional('internal'),
        'mod',
        field('name', $.identifier),
        field('body', $.module_body),
      ),

    module_body: ($) => seq('{', repeat($._declaration), '}'),

    const_declaration: ($) =>
      seq(
        'const',
        field('name', $.identifier),
        optional(seq(':', field('type', $._type))),
        '=',
        field('value', $._expression),
        optional(';'),
      ),

    test_declaration: ($) =>
      seq(choice('test', 'bench'), field('name', $.string), field('body', $.block)),

    // ---- types ----------------------------------------------------------

    _type: ($) =>
      choice(
        $.union_type,
        $.intersection_type,
        $.negated_type,
        $.function_type,
        $.tuple_type,
        $.parenthesized_type,
        $.structural_type,
        $.generic_type,
        $.path,
        $.identifier,
        $.atom,
        $.any_type,
        $.null,
      ),

    union_type: ($) => prec.left(PREC.union, seq($._type, '|', $._type)),
    intersection_type: ($) => prec.left(PREC.intersection, seq($._type, '&', $._type)),
    negated_type: ($) => prec.right(PREC.negate, seq('!', $._type)),

    // `(A, B) throws E -> C`. The codomain binds to the arrow, so a `throws`
    // with no arrow is an error rather than a tuple that ate its clause.
    function_type: ($) =>
      prec.right(
        seq(
          field('parameters', $.function_type_parameters),
          optional(field('throws', $.throws_clause)),
          '->',
          field('return_type', $._type),
        ),
      ),

    function_type_parameters: ($) => seq('(', optional(seq(commaSep1($._type), optional(','))), ')'),

    // `()` is unit and `(A, B)` a tuple; there is nothing at arity one, so
    // `(A)` is a grouping and `(A,)` is an error the compiler reports.
    tuple_type: ($) =>
      choice(
        seq('(', ')'),
        seq('(', $._type, ',', ')'),
        seq('(', $._type, repeat1(seq(',', $._type)), optional(','), ')'),
      ),

    parenthesized_type: ($) => seq('(', $._type, ')'),

    structural_type: ($) =>
      seq('{', optional(seq(commaSep1($.field_declaration), optional(','))), '}'),

    // `x as List[i64]` -- the type grabs its arguments greedily, as
    // `atomic_type`'s `named` does; the `[` is never an index on the cast.
    generic_type: ($) =>
      prec(20, seq(field('name', choice($.path, $.identifier)), field('arguments', $.type_arguments))),

    type_arguments: ($) => seq('[', optional(seq(commaSep1($._type), optional(','))), ']'),

    // A turbofish and an index are the same tokens until the `(` after the `]`.
    // Restricting the arguments to types that cannot start like a parenthesised
    // expression keeps that ambiguity to the one case GLR can actually settle:
    // a bare name. `f[(i64) -> str]()` is the casualty; write a named alias.
    turbofish_arguments: ($) => seq('[', commaSep1($._simple_type), optional(','), ']'),

    _simple_type: ($) =>
      choice(
        $.simple_union_type,
        $.simple_intersection_type,
        $.simple_negated_type,
        $.generic_type,
        $.path,
        $.identifier,
        $.atom,
        $.any_type,
        $.null,
      ),

    simple_union_type: ($) => prec.left(PREC.union, seq($._simple_type, '|', $._simple_type)),
    simple_intersection_type: ($) =>
      prec.left(PREC.intersection, seq($._simple_type, '&', $._simple_type)),
    simple_negated_type: ($) => prec.right(PREC.negate, seq('!', $._simple_type)),

    any_type: (_) => 'any',

    // The type of a `throws` clause: everything except a top-level arrow, or
    // `fn f() throws (str) -> i64` reads as throwing a function.
    _throws_type: ($) =>
      choice(
        $.throws_union_type,
        $.throws_intersection_type,
        $.throws_negated_type,
        $.throws_tuple_type,
        $.throws_parenthesized_type,
        $.structural_type,
        $.generic_type,
        $.path,
        $.identifier,
        $.atom,
        $.any_type,
        $.null,
      ),

    throws_union_type: ($) => prec.left(PREC.union, seq($._throws_type, '|', $._throws_type)),
    throws_intersection_type: ($) =>
      prec.left(PREC.intersection, seq($._throws_type, '&', $._throws_type)),
    throws_negated_type: ($) => prec.right(PREC.negate, seq('!', $._throws_type)),
    throws_tuple_type: ($) =>
      choice(
        seq('(', ')'),
        seq('(', $._type, ',', ')'),
        seq('(', $._type, repeat1(seq(',', $._type)), optional(','), ')'),
      ),
    throws_parenthesized_type: ($) => seq('(', $._type, ')'),

    // ---- statements -----------------------------------------------------

    block: ($) => seq('{', repeat($._statement), optional(field('tail', $._expression)), '}'),

    _statement: ($) =>
      choice($.let_statement, $.assignment_statement, $.expression_statement),

    // Semicolons are optional throughout: the parser writes `.or_not()` on
    // every one of them.
    let_statement: ($) =>
      seq(
        'let',
        field('pattern', $._pattern),
        optional(seq(':', field('type', $._type))),
        '=',
        field('value', $._expression),
        optional(';'),
      ),

    // Bindings rebind; there is no `mut`. `p.f = e` and `xs[i] = e` parse here
    // so the compiler can reject them with advice rather than a syntax error.
    assignment_statement: ($) =>
      seq(field('left', $._expression), '=', field('right', $._expression), optional(';')),

    expression_statement: ($) =>
      choice(
        seq($._expression, ';'),
        // Above the whole binary ladder, or `-` would win the tie and
        // `if a {} else {}` followed by `-1;` would silently become one
        // subtraction -- the exact bug parser/mod.rs calls out.
        prec.dynamic(PREC.statement_block, seq($._expression_ending_with_block, optional(';'))),
      ),

    // A block-like expression at the START of a statement is a statement, not
    // the left operand of whatever follows -- otherwise `if a {} else {}`
    // followed by a line beginning `-1;` becomes one subtraction.
    _expression_ending_with_block: ($) =>
      choice($.if_expression, $.match_expression, $.loop_expression, $.while_expression, $.for_expression, $.block),

    // ---- patterns -------------------------------------------------------

    _pattern: ($) =>
      choice(
        $.wildcard_pattern,
        $.is_pattern,
        $.record_pattern,
        $.tuple_pattern,
        $.literal_pattern,
        $.identifier,
      ),

    wildcard_pattern: (_) => '_',

    is_pattern: ($) => seq('is', field('type', $._type)),

    record_pattern: ($) =>
      seq(
        optional(field('path', choice($.path, $.identifier))),
        '{',
        optional(seq(commaSep1($.field_pattern), optional(','))),
        optional('..'),
        '}',
      ),

    // `x` alone binds the field to `x`.
    field_pattern: ($) =>
      seq(field('name', $.identifier), optional(seq(':', field('pattern', $._pattern)))),

    tuple_pattern: ($) =>
      choice(
        seq('(', ')'),
        seq('(', $._pattern, ',', ')'),
        seq('(', $._pattern, repeat1(seq(',', $._pattern)), optional(','), ')'),
      ),

    literal_pattern: ($) =>
      choice(
        $.integer,
        $.float,
        $.rune,
        $.atom,
        $.string,
        $.boolean,
        $.null,
        seq('-', choice($.integer, $.float)),
      ),

    // ---- expressions ----------------------------------------------------

    _expression: ($) =>
      choice(
        $.binary_expression,
        $.unary_expression,
        $.try_expression,
        $.call_expression,
        $.index_expression,
        $.field_expression,
        $.is_expression,
        $.as_expression,
        $.lambda_expression,
        $.record_literal,
        $.list_expression,
        $.tuple_expression,
        $.unit_expression,
        $.parenthesized_expression,
        $.if_expression,
        $.match_expression,
        $.loop_expression,
        $.while_expression,
        $.for_expression,
        $.block,
        $.break_expression,
        $.continue_expression,
        $.return_expression,
        $.throw_expression,
        $.assert_expression,
        $.string,
        $.integer,
        $.float,
        $.rune,
        $.atom,
        $.boolean,
        $.null,
        $.path,
        $.identifier,
      ),

    binary_expression: ($) => {
      // ops::BINARY_OPS, verbatim. Every binary operator is left-associative.
      const table = [
        [PREC.orelse, 'orelse'],
        [PREC.or, 'or'],
        [PREC.and, 'and'],
        [PREC.compare, choice('==', '!=', '<=', '>=', '<', '>')],
        [PREC.pipe, '|>'],
        [PREC.bor, 'bor'],
        [PREC.bxor, 'bxor'],
        [PREC.band, 'band'],
        [PREC.shift, choice('bsl', 'bsr')],
        [PREC.add, choice('+', '-')],
        [PREC.multiply, choice('*', '/', '%')],
      ];
      return choice(
        ...table.map(([precedence, operator]) =>
          prec.left(
            precedence,
            seq(
              field('left', $._expression),
              field('operator', operator),
              field('right', $._expression),
            ),
          ),
        ),
      );
    },

    unary_expression: ($) =>
      prec.right(
        PREC.unary,
        seq(field('operator', choice('-', '!', 'bnot')), field('operand', $._expression)),
      ),

    // `try` binds at the unary level, not the full expression: with the full
    // parser `try? get(m, k) orelse 30` becomes `try? (get(m, k) orelse 30)`,
    // an orelse on a non-nullable type, so the default silently never applies.
    try_expression: ($) =>
      prec.right(
        PREC.try,
        seq(
          'try',
          optional(field('form', choice('?', '!'))),
          field('body', $._expression),
          optional(field('catch', $.catch_clause)),
        ),
      ),

    catch_clause: ($) =>
      seq('catch', '(', field('binding', $.identifier), ')', field('body', $.block)),

    call_expression: ($) =>
      prec(
        PREC.postfix,
        seq(
          field('function', $._expression),
          // Turbofish: `f[i64](x)`.
          optional(field('type_arguments', $.turbofish_arguments)),
          field('arguments', $.arguments),
        ),
      ),

    arguments: ($) => seq('(', optional(seq(commaSep1($._expression), optional(','))), ')'),

    index_expression: ($) =>
      prec(PREC.postfix, seq(field('base', $._expression), '[', field('index', $._expression), ']')),

    field_expression: ($) =>
      prec(PREC.postfix, seq(field('base', $._expression), '.', field('field', $.identifier))),

    is_expression: ($) =>
      prec(PREC.postfix, seq(field('left', $._expression), 'is', field('type', $._type))),

    as_expression: ($) =>
      prec(PREC.postfix, seq(field('left', $._expression), 'as', field('type', $._type))),

    lambda_expression: ($) =>
      prec.right(seq(field('parameters', $.lambda_parameters), '=>', field('body', $._expression))),

    lambda_parameters: ($) =>
      seq('(', optional(seq(commaSep1($.lambda_parameter), optional(','))), ')'),

    lambda_parameter: ($) =>
      seq(field('name', $.identifier), optional(seq(':', field('type', $._type)))),

    // `Point { x: 1, ..base }`, or `{ x: 1 }` with no path.
    //
    // A path or at least one field is required. The compiler accepts a bare
    // `{}` as an empty record literal too, but that spelling is also an empty
    // block, and requiring content is what keeps the two apart here.
    record_literal: ($) =>
      choice(
        seq(
          field('path', choice($.path, $.identifier)),
          '{',
          optional(seq(commaSep1($.field_initializer), optional(','))),
          optional(seq('..', field('spread', $._expression))),
          '}',
        ),
        seq(
          '{',
          choice(
            seq(commaSep1($.field_initializer), optional(','), optional(seq('..', field('spread', $._expression)))),
            seq('..', field('spread', $._expression)),
          ),
          '}',
        ),
      ),

    field_initializer: ($) =>
      seq(field('name', $.identifier), ':', field('value', $._expression)),

    list_expression: ($) =>
      seq('[', optional(seq(commaSep1(choice($._expression, $.spread_element)), optional(','))), ']'),

    spread_element: ($) => seq('..', $._expression),

    tuple_expression: ($) =>
      choice(
        seq('(', $._expression, ',', ')'),
        seq('(', $._expression, repeat1(seq(',', $._expression)), optional(','), ')'),
      ),

    unit_expression: (_) => seq('(', ')'),

    parenthesized_expression: ($) => seq('(', $._expression, ')'),

    if_expression: ($) =>
      prec.right(
        seq(
          'if',
          field('condition', $._expression),
          field('consequence', $.block),
          optional(seq('else', field('alternative', choice($.block, $.if_expression)))),
        ),
      ),

    match_expression: ($) =>
      seq('match', field('value', $._expression), field('body', $.match_body)),

    match_body: ($) => seq('{', optional(seq(commaSep1($.match_arm), optional(','))), '}'),

    match_arm: ($) =>
      seq(
        field('pattern', $._pattern),
        optional(seq('if', field('guard', $._expression))),
        '=>',
        field('value', $._expression),
      ),

    loop_expression: ($) => seq('loop', field('body', $.block)),

    while_expression: ($) =>
      seq('while', field('condition', $._expression), field('body', $.block)),

    for_expression: ($) =>
      seq(
        'for',
        field('pattern', $._pattern),
        'in',
        field('value', $._expression),
        field('body', $.block),
      ),

    break_expression: ($) => prec.right(seq('break', optional($._expression))),
    continue_expression: (_) => 'continue',
    return_expression: ($) => prec.right(seq('return', optional($._expression))),
    throw_expression: ($) => prec.right(seq('throw', $._expression)),

    assert_expression: ($) =>
      seq(
        field('kind', choice('assert', 'assert_eq', 'assert_ne', 'assert_throws')),
        field('arguments', $.arguments),
      ),

    // ---- leaves ---------------------------------------------------------

    path: ($) => prec.left(seq($.identifier, repeat1(seq('::', $.identifier)))),

    identifier: (_) => token(IDENT),

    // `:name`. The lexer only reads a `:` as an atom when an identifier follows
    // it immediately and it does not directly follow one, so `{ x:y }` is a
    // field and `f(:ok)` is an atom.
    atom: (_) => token(seq(':', IDENT)),

    boolean: (_) => choice('true', 'false'),
    null: (_) => 'null',

    integer: (_) =>
      token(
        choice(
          /0[xX][0-9a-fA-F](_?[0-9a-fA-F])*/,
          /0[oO][0-7](_?[0-7])*/,
          /0[bB][01](_?[01])*/,
          /[0-9](_?[0-9])*/,
        ),
      ),

    // A float needs a digit after the dot, so `x.0` stays field access and
    // `xs..1` stays a spread.
    float: (_) =>
      token(
        choice(
          /[0-9](_?[0-9])*\.[0-9](_?[0-9])*([eE][+-]?[0-9](_?[0-9])*)?/,
          /[0-9](_?[0-9])*[eE][+-]?[0-9](_?[0-9])*/,
        ),
      ),

    rune: (_) => token(seq("'", choice(/[^'\\]/, ESCAPE), "'")),

    // A string lexes as a run of parts because interpolation nests:
    // `"a #{f("b")} c"` puts a string inside a hole inside a string.
    string: ($) =>
      seq(
        '"',
        repeat(choice($.string_content, $.escape_sequence, $.interpolation)),
        '"',
      ),

    // `#{` opens a hole; a bare `#` is literal text, which is the point of the
    // delimiter -- `{` never needs escaping.
    string_content: (_) => token.immediate(prec(2, choice(/[^"\\#]+/, '#'))),

    escape_sequence: (_) => token.immediate(prec(2, ESCAPE)),

    interpolation: ($) => seq(token.immediate(prec(2, '#{')), $._expression, '}'),
  },
});

function commaSep1(rule) {
  return seq(rule, repeat(seq(',', rule)));
}
