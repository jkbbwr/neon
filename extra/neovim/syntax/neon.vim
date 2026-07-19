" Vim syntax file for Neon.
"
" The keyword and operator lists here are transcribed from
" `compiler/src/lexer/token.rs` -- specifically `Token::keyword`, which is the one
" hand-maintained reserved-word table in the compiler, and the `Display` impl,
" which is the complete punctuation alphabet. If a token is added there, add it
" here. Nothing in this file is guessed from other languages.
"
" Three deliberate omissions:
"   * `enum` is NOT a keyword. token.rs says so explicitly: sum types are unions of
"     records, and `enum` lexes as an ordinary identifier. It is not highlighted.
"   * `..=` does not exist. The only range-ish token is `..` (record/list spread);
"     ranges are the function `range(a, b)`.
"   * There is no Neon tree-sitter grammar. This regex syntax is the whole story.
"
" ORDERING: when two items match at the same position, the one defined LAST wins.
" That is why comments are defined at the bottom -- the `/` operator would
" otherwise swallow the `//` that starts a line comment.

if exists('b:current_syntax')
  finish
endif

syntax case match

" ---- keywords ----------------------------------------------------------------
" Declarations.
syntax keyword neonKeyword fn let record opaque newtype type mu protocol marker
syntax keyword neonKeyword impl where use mod internal const

" Control flow.
syntax keyword neonConditional if else match
syntax keyword neonRepeat      loop while for
syntax keyword neonKeyword     in
syntax keyword neonStatement   break continue return

" Errors.
syntax keyword neonException throws throw try catch orelse

" Tests. `assert_eq`/`assert_ne`/`assert_throws` are single reserved words, not
" calls to functions of those names.
syntax keyword neonKeyword test bench
syntax keyword neonKeyword assert assert_eq assert_ne assert_throws

" Word-operators. These are tokens, not identifiers: `band`, `bsl` and friends are
" the bitwise operators, spelled as words because `<<` and `&` are taken.
syntax keyword neonOperator and or band bor bxor bnot bsl bsr is as

" ---- literals that are words -------------------------------------------------
syntax keyword neonBoolean true false
syntax keyword neonNull    null

" ---- types -------------------------------------------------------------------
" The four primitives that `primitive()` in typecheck/resolve.rs recognises, plus
" `any`, which the parser handles as its own type-spec kind rather than a name.
syntax keyword neonType i64 f64 str bool any

" Prelude names. These are ordinary declarations, not keywords, but they are in
" scope in every file without a `use` (stdlib/prelude.neon), so highlighting them
" as types is accurate and useful.
syntax keyword neonPreludeType List Map Display Error Ord Ordering IndexError

" A capitalised identifier is conventionally a type or record name. Keyword items
" always outrank match items, so the lists above are unaffected by this.
syntax match neonTypeName "\<\u\w*\>"

" ---- annotations -------------------------------------------------------------
" The complete registry is `lookup()` in compiler/src/expand.rs: exactly five.
" Anything else is a hard error from the compiler ("unknown annotation `@wat`"),
" so the catch-all is defined first and the five valid names override it.
syntax match neonAnnotationBad "@\w\+"
syntax match neonAnnotation    "@\%(native\|cfg\|doc\|runtime\|pure\)\>"

" ---- atoms -------------------------------------------------------------------
" `:name`. The lexer's rule (`atom_ahead`) is that a `:` opens an atom only when an
" identifier follows it *immediately* and it does not *immediately* follow one --
" which is what keeps `x: i64` an annotation and `x:y` a punctuation colon. The
" look-behind encodes exactly that; `\d\@!` keeps `:0` from being an atom.
syntax match neonAtom "\w\@<!:\d\@!\k\+"

" `::` is a path separator, never an atom. It starts one column earlier than the
" atom pattern could, and the earliest start wins, so `std::io` is safe.
syntax match neonDelimiter "::"

" ---- numbers -----------------------------------------------------------------
" Radix prefixes are 0x/0o/0b (each accepting an uppercase marker), `_` is a digit
" separator, and a float needs a digit after the dot -- `x.0` is field access and
" `xs..1` is a spread, neither is a float. See `number()` in the lexer.
syntax match neonNumber "\<0[xX][0-9a-fA-F][0-9a-fA-F_]*\>"
syntax match neonNumber "\<0[oO][0-7][0-7_]*\>"
syntax match neonNumber "\<0[bB][01][01_]*\>"
syntax match neonNumber "\<\d[0-9_]*\>"
syntax match neonFloat  "\<\d[0-9_]*\.\d[0-9_]*\%([eE][-+]\=\d[0-9_]*\)\=\>"
syntax match neonFloat  "\<\d[0-9_]*[eE][-+]\=\d[0-9_]*\>"

" ---- strings and runes -------------------------------------------------------
" Escapes shared by strings and runes (`escape()` in the lexer): the single-char
" forms, `\xNN` capped at 0xFF, and `\u{...}`. `\#` escapes an interpolation.
syntax match neonEscape "\\[nrt0\\'\"#]" contained
syntax match neonEscape "\\x[0-9a-fA-F]\{2}" contained
syntax match neonEscape "\\u{[0-9a-fA-F]\{1,6}}" contained

" `"a #{expr} b"` -- interpolation nests arbitrarily, including strings inside the
" hole, so the hole contains the top-level cluster and itself.
syntax region neonString start=+"+ skip=+\\.+ end=+"+ contains=neonEscape,neonInterp,@Spell
syntax region neonInterp matchgroup=neonInterpDelim start="#{" end="}" contained contains=TOP,neonInterp

" Runes: exactly one character, or one escape.
syntax match neonRune "'\%([^'\\]\|\\[nrt0\\'\"#]\|\\x[0-9a-fA-F]\{2}\|\\u{[0-9a-fA-F]\{1,6}}\)'" contains=neonEscape

" ---- punctuation operators ---------------------------------------------------
" Transcribed from the `Display` impl in token.rs. Multi-character forms are
" listed first so the longer match wins at a shared start position.
syntax match neonOperator "|>\|=>\|->\|==\|!=\|<=\|>=\|\.\."
syntax match neonOperator "[-+*/%=<>?!&|]"

" ---- function definitions ----------------------------------------------------
" A look-BEHIND, not `\zs`. `fn` is a keyword item, and keyword items outrank match
" items, so a pattern starting at `fn` is discarded before `\zs` ever applies --
" scanning then resumes past the keyword, where the pattern no longer matches.
" Verified: `\zs` here highlights nothing.
syntax match neonFunction "\%(\<fn\>\s\+\)\@<=\k\+"
syntax match neonFunction "\%(\<\%(test\|bench\)\>\s\+\)\@<=\k\+"

" ---- comments ----------------------------------------------------------------
" Defined last, on purpose: see the ORDERING note at the top of this file.
" `///` is a doc comment and attaches to the item below it; `////` is not (see
" `next_trivia` in the lexer). Block comments nest, which the self-reference in
" `contains` gives us.
syntax keyword neonTodo contained TODO FIXME XXX NOTE HACK SAFETY
syntax match   neonComment      "//.*$" contains=neonTodo,@Spell
syntax match   neonDocComment   "///\%(/\)\@!.*$" contains=neonTodo,@Spell
syntax region  neonBlockComment start="/\*" end="\*/" contains=neonBlockComment,neonTodo,@Spell

" ---- highlight links ---------------------------------------------------------
highlight default link neonComment       Comment
highlight default link neonDocComment    SpecialComment
highlight default link neonBlockComment  Comment
highlight default link neonTodo          Todo
highlight default link neonKeyword       Keyword
highlight default link neonConditional   Conditional
highlight default link neonRepeat        Repeat
highlight default link neonStatement     Statement
highlight default link neonException     Exception
highlight default link neonOperator      Operator
highlight default link neonDelimiter     Delimiter
highlight default link neonBoolean       Boolean
highlight default link neonNull          Constant
highlight default link neonType          Type
highlight default link neonPreludeType   Type
highlight default link neonTypeName      Structure
highlight default link neonAnnotation    PreProc
highlight default link neonAnnotationBad Error
highlight default link neonAtom          Constant
highlight default link neonNumber        Number
highlight default link neonFloat         Float
highlight default link neonString        String
highlight default link neonEscape        SpecialChar
highlight default link neonRune          Character
highlight default link neonInterpDelim   Special
highlight default link neonFunction      Function

let b:current_syntax = 'neon'
