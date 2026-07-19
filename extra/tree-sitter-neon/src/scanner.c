#include "tree_sitter/parser.h"

// Neon's block comments nest, so a commented-out block containing a comment
// does not end early (compiler/src/lexer/mod.rs, `skip_trivia`). A regex
// cannot count, which is the whole reason this file exists.

enum TokenType {
  BLOCK_COMMENT,
};

void *tree_sitter_neon_external_scanner_create(void) { return NULL; }
void tree_sitter_neon_external_scanner_destroy(void *payload) { (void)payload; }
unsigned tree_sitter_neon_external_scanner_serialize(void *payload, char *buffer) {
  (void)payload;
  (void)buffer;
  return 0;
}
void tree_sitter_neon_external_scanner_deserialize(void *payload, const char *buffer,
                                                   unsigned length) {
  (void)payload;
  (void)buffer;
  (void)length;
}

bool tree_sitter_neon_external_scanner_scan(void *payload, TSLexer *lexer,
                                            const bool *valid_symbols) {
  (void)payload;
  if (!valid_symbols[BLOCK_COMMENT]) {
    return false;
  }

  while (lexer->lookahead == ' ' || lexer->lookahead == '\t' || lexer->lookahead == '\n' ||
         lexer->lookahead == '\r' || lexer->lookahead == '\f') {
    lexer->advance(lexer, true);
  }

  if (lexer->lookahead != '/') {
    return false;
  }
  lexer->advance(lexer, false);
  if (lexer->lookahead != '*') {
    return false;
  }
  lexer->advance(lexer, false);

  unsigned depth = 1;
  while (depth > 0) {
    if (lexer->eof(lexer)) {
      // Unterminated. Report what we have so the rest of the file still parses;
      // the compiler is the one that complains about it.
      break;
    }
    if (lexer->lookahead == '/') {
      lexer->advance(lexer, false);
      if (lexer->lookahead == '*') {
        lexer->advance(lexer, false);
        depth++;
      }
    } else if (lexer->lookahead == '*') {
      lexer->advance(lexer, false);
      if (lexer->lookahead == '/') {
        lexer->advance(lexer, false);
        depth--;
      }
    } else {
      lexer->advance(lexer, false);
    }
  }

  lexer->result_symbol = BLOCK_COMMENT;
  return true;
}
