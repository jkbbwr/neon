" Indentation for Neon.
"
" A deliberately small brace-matching heuristic rather than a real grammar. It is the
" fallback for buffers with no compiled tree-sitter parser -- `extra/tree-sitter-neon`
" ships an `indents.scm`, and nvim-treesitter's indent module supersedes this file
" wherever that parser is installed. `cindent` is not an option either way: it treats
" `:` as a label (so `x: i64` and the atom `:ok` both throw off the indent) and it
" special-cases `#` as a preprocessor line, which collides with `#{` interpolation.
"
" The rule: inherit the previous non-blank line's indent, add a level if that line
" ended inside an opener, remove a level if this line starts with a closer. That is
" correct for the overwhelming majority of real code and never fights the user,
" because a wrong guess is one `==` away from being overridden by hand.

if exists('b:did_indent')
  finish
endif
let b:did_indent = 1

setlocal indentexpr=NeonIndent()
" Re-indent when a closer is typed, so `}` snaps back as you type it.
setlocal indentkeys=0{,0},0),0],!^F,o,O,e
setlocal nosmartindent nocindent nolisp

let b:undo_indent = 'setlocal indentexpr< indentkeys< smartindent< cindent< lisp<'

if exists('*NeonIndent')
  finish
endif

" A copy of the line with comments, strings and runes blanked out, so that a brace
" inside `"a { b"` or `// }` cannot move the indent. Interpolation holes are blanked
" along with the string that contains them; a `{` inside `#{...}` is not a block.
function! s:Code(line) abort
  let l:s = a:line
  " Block comments are handled per-line only: a brace inside a multi-line /* */ can
  " still confuse this. Accepted -- the alternative is a real lexer.
  let l:s = substitute(l:s, '/\*.\{-}\*/', '', 'g')
  let l:s = substitute(l:s, '//.*$', '', '')
  let l:s = substitute(l:s, '"\%(\\.\|[^"\\]\)*"', '""', 'g')
  let l:s = substitute(l:s, "'\\%(\\\\.\\|[^'\\\\]\\)*'", "''", 'g')
  return l:s
endfunction

" How many brackets a line opens and leaves open at its end.
"
" NOT openers-minus-closers. `} else {` has one of each, and a naive net of zero
" would leave the whole else-body flat against the `if` -- observed, then fixed.
" Pairs that open and close within the line cancel; a closer with no opener before
" it on the same line belongs to an earlier line and is handled by the leading-
" closer rule instead, so it is ignored here.
function! s:Opens(line) abort
  let l:code = s:Code(a:line)
  let l:depth = 0
  for l:i in range(strlen(l:code))
    let l:c = l:code[l:i]
    if l:c ==# '{' || l:c ==# '[' || l:c ==# '('
      let l:depth += 1
    elseif l:c ==# '}' || l:c ==# ']' || l:c ==# ')'
      if l:depth > 0
        let l:depth -= 1
      endif
    endif
  endfor
  return l:depth
endfunction

function! NeonIndent() abort
  let l:prevlnum = prevnonblank(v:lnum - 1)
  if l:prevlnum == 0
    return 0
  endif

  let l:prev = getline(l:prevlnum)
  let l:cur = s:Code(getline(v:lnum))
  let l:ind = indent(l:prevlnum)
  let l:sw = shiftwidth()

  " Continuation of a block comment: line the `*` up under the opening `/*`.
  if l:prev =~# '^\s*/\*' && l:cur !~# '\*/'
    return l:ind + 1
  endif
  if l:cur =~# '^\s*\*\%(/\)\?'
    let l:above = getline(l:prevlnum)
    if l:above =~# '^\s*\%(/\*\|\*\)'
      return l:ind
    endif
  endif

  " One level per *line* that leaves a bracket open -- not one per bracket.
  "
  " `File { r: resource::new(fd, (d: i64) => {` leaves three brackets open, and
  " `neon fmt` indents the next line by one level, not three. Checked against every
  " file in stdlib/: one-per-line reproduces the formatter exactly, one-per-bracket
  " does not. The matching dedent below is capped the same way.
  let l:opens = s:Opens(l:prev)
  if l:opens > 0
    let l:ind += l:sw
  endif

  " A trailing `=>` (match arm with a non-brace body), `->` (wrapped signature) or
  " `|>` (pipeline) continues onto the next line.
  if l:opens == 0 && s:Code(l:prev) =~# '\%(=>\|->\||>\)\s*$'
    let l:ind += l:sw
  endif

  " This line starts with a closer -> pull it back one level. `else` and `catch`
  " need no rule of their own: they follow the `}` that this dedents.
  if l:cur =~# '^\s*[}\])]'
    let l:ind -= l:sw
  endif

  return l:ind < 0 ? 0 : l:ind
endfunction
