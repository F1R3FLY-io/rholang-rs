/*
 * External scanner for the Rholang grammar.
 *
 * It decides the two contextual keywords of Reified RSpaces, `use` and
 * `free`. The `externals` section of grammar.js documents the rule, the
 * exclusion tables, a 02cef80 witness for every excluded word, and the
 * reasoning. In short, the scanner emits a keyword token only when the text
 * that follows can never continue a process that parses at 02cef80. In every
 * other case it returns false, and the internal lexer produces an ordinary
 * `var`.
 *
 *   use <separators> <space name> <separators?> {     -> `_use_keyword`
 *   free <separators> <identifier> <separators?> (    -> `_free_keyword`
 *
 * Separators are whitespace and comments. The keyword token covers only the
 * keyword (`mark_end`); the rest is lookahead.
 *
 * It also emits the bare exponent float `-?\d+[eE][+-]?\d+` when no
 * identifier character follows it (see `float_literal` in grammar.js).
 *
 * The scanner keeps no state.
 */

#include "tree_sitter/parser.h"

#include <stdbool.h>
#include <stdint.h>
#include <string.h>

/* The order must match `externals` in grammar.js. */
enum TokenType {
  USE_KEYWORD,
  FREE_KEYWORD,
  BARE_EXPONENT_FLOAT,
  ERROR_RECOVERY_SENTINEL,
};

/*
 * The words that each keyword form excludes. A word W is excluded exactly
 * when the text can continue a process that parses at 02cef80, with `use` or
 * `free` as a variable (scripts/derive_scanner_exclusions.js derives the
 * tables from the 02cef80 grammar.json; grammar.js gives a witness for each
 * word):
 *   USE_EXCLUDED_WORDS    `use W {`  (`W {` follows a complete process);
 *   FREE_EXCLUDED_WORDS   `free W (` (`W (` follows a complete expression);
 *   QUOTE_EXCLUDED_WORDS  `use @W {` (a process starts with `W {`).
 */
static const char *const USE_EXCLUDED_WORDS[] = {
    "and", "bundle", "bundle0", "else", "in", "match",
    "matches", "not", "or", "select", "where",
};

static const char *const FREE_EXCLUDED_WORDS[] = {
    "Set", "and", "else", "for", "if", "in",
    "match", "matches", "not", "or", "where",
};

static const char *const QUOTE_EXCLUDED_WORDS[] = {
    "bundle", "bundle0", "match", "not", "select",
};

typedef struct {
  const char *const *words;
  size_t count;
} WordTable;

#define WORD_TABLE(words) {(words), sizeof(words) / sizeof((words)[0])}

static const WordTable USE_EXCLUSIONS = WORD_TABLE(USE_EXCLUDED_WORDS);
static const WordTable FREE_EXCLUSIONS = WORD_TABLE(FREE_EXCLUDED_WORDS);
static const WordTable QUOTE_EXCLUSIONS = WORD_TABLE(QUOTE_EXCLUDED_WORDS);

/* Length of the longest excluded word ("bundle0", "matches"). */
#define MAX_EXCLUDED_WORD_LENGTH 7

typedef enum {
  SEPARATORS_NONE,
  SEPARATORS_FOUND,
  /* A `/` that starts no comment, or an unterminated block comment. */
  SEPARATORS_INVALID,
} SeparatorScan;

typedef enum {
  IDENTIFIER_NONE,
  IDENTIFIER_EXCLUDED,
  IDENTIFIER_ACCEPTED,
} IdentifierScan;

static inline void advance(TSLexer *lexer) { lexer->advance(lexer, false); }

/* The characters that the grammar's `/\s/` extra matches. */
static inline bool is_whitespace(int32_t c) {
  return c == ' ' || (c >= '\t' && c <= '\r');
}

static inline bool is_ascii_letter(int32_t c) {
  return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z');
}

static inline bool is_ascii_digit(int32_t c) { return c >= '0' && c <= '9'; }

/* A character that can continue a `var` ([a-zA-Z0-9_']). */
static inline bool is_identifier_char(int32_t c) {
  return is_ascii_letter(c) || is_ascii_digit(c) || c == '_' || c == '\'';
}

static bool is_excluded_word(const WordTable *table, const char *word,
                             unsigned length) {
  if (length > MAX_EXCLUDED_WORD_LENGTH) {
    return false;
  }
  for (size_t i = 0; i < table->count; i++) {
    const char *excluded = table->words[i];
    if (strlen(excluded) == length && memcmp(excluded, word, length) == 0) {
      return true;
    }
  }
  return false;
}

/*
 * Consumes `keyword` and checks that no identifier character follows it, so
 * that `user` or `freedom` stay identifiers.
 */
static bool consume_keyword(TSLexer *lexer, const char *keyword) {
  for (const char *c = keyword; *c != '\0'; c++) {
    if (lexer->lookahead != *c) {
      return false;
    }
    advance(lexer);
  }
  return !is_identifier_char(lexer->lookahead);
}

/* Consumes the rest of a line comment; the `//` is already consumed. */
static void skip_line_comment(TSLexer *lexer) {
  while (!lexer->eof(lexer) && lexer->lookahead != '\n') {
    advance(lexer);
  }
}

/*
 * Consumes the rest of a block comment; the `/` and `*` are already consumed.
 * Block comments do not nest, as in the grammar's `_block_comment`.
 */
static bool skip_block_comment(TSLexer *lexer) {
  for (;;) {
    if (lexer->eof(lexer)) {
      return false;
    }
    if (lexer->lookahead == '*') {
      while (lexer->lookahead == '*') {
        advance(lexer);
      }
      if (lexer->lookahead == '/') {
        advance(lexer);
        return true;
      }
    } else {
      advance(lexer);
    }
  }
}

/* Consumes whitespace and comments. */
static SeparatorScan skip_separators(TSLexer *lexer) {
  bool found = false;
  for (;;) {
    if (is_whitespace(lexer->lookahead)) {
      advance(lexer);
      found = true;
      continue;
    }
    if (lexer->lookahead != '/') {
      return found ? SEPARATORS_FOUND : SEPARATORS_NONE;
    }
    advance(lexer);
    if (lexer->lookahead == '/') {
      advance(lexer);
      skip_line_comment(lexer);
    } else if (lexer->lookahead == '*') {
      advance(lexer);
      if (!skip_block_comment(lexer)) {
        return SEPARATORS_INVALID;
      }
    } else {
      return SEPARATORS_INVALID;
    }
    found = true;
  }
}

/*
 * Consumes a `var`-shaped identifier ([a-zA-Z][a-zA-Z0-9_']* or
 * _[a-zA-Z0-9_']+) and reports whether it is a word of `exclusions`. A lone
 * `_` is the wildcard, not an identifier.
 */
static IdentifierScan scan_identifier(TSLexer *lexer,
                                      const WordTable *exclusions) {
  if (lexer->lookahead == '_') {
    advance(lexer);
    if (!is_identifier_char(lexer->lookahead)) {
      return IDENTIFIER_NONE;
    }
    while (is_identifier_char(lexer->lookahead)) {
      advance(lexer);
    }
    return IDENTIFIER_ACCEPTED;
  }
  if (!is_ascii_letter(lexer->lookahead)) {
    return IDENTIFIER_NONE;
  }
  char word[MAX_EXCLUDED_WORD_LENGTH];
  unsigned length = 0;
  while (is_identifier_char(lexer->lookahead)) {
    if (length < MAX_EXCLUDED_WORD_LENGTH) {
      word[length] = (char)lexer->lookahead;
    }
    if (length <= MAX_EXCLUDED_WORD_LENGTH) {
      length++;
    }
    advance(lexer);
  }
  return is_excluded_word(exclusions, word, length) ? IDENTIFIER_EXCLUDED
                                                    : IDENTIFIER_ACCEPTED;
}

/* Consumes a string literal, escapes included; the lookahead is `"`. */
static bool skip_string_literal(TSLexer *lexer) {
  advance(lexer);
  for (;;) {
    if (lexer->eof(lexer)) {
      return false;
    }
    switch (lexer->lookahead) {
      case '"':
        advance(lexer);
        return true;
      case '\\':
        advance(lexer);
        if (lexer->eof(lexer)) {
          return false;
        }
        advance(lexer);
        break;
      default:
        advance(lexer);
        break;
    }
  }
}

/* Consumes a URI literal (`…`, at least one character); the lookahead is `. */
static bool skip_uri_literal(TSLexer *lexer) {
  advance(lexer);
  if (lexer->lookahead == '`') {
    return false;
  }
  for (;;) {
    if (lexer->eof(lexer)) {
      return false;
    }
    if (lexer->lookahead == '`') {
      advance(lexer);
      return true;
    }
    advance(lexer);
  }
}

/*
 * Consumes a number literal with its suffix (`7`, `1.5`, `1e3`, `8u16`,
 * `2.5f32`). The lookahead is a digit. The internal lexer validates the exact
 * literal later; this only finds its end.
 */
static bool skip_number_literal(TSLexer *lexer) {
  while (is_ascii_digit(lexer->lookahead)) {
    advance(lexer);
  }
  if (lexer->lookahead == '.') {
    advance(lexer);
    if (!is_ascii_digit(lexer->lookahead)) {
      return false;
    }
    while (is_ascii_digit(lexer->lookahead)) {
      advance(lexer);
    }
  }
  if (lexer->lookahead == 'e' || lexer->lookahead == 'E') {
    advance(lexer);
    if (lexer->lookahead == '+' || lexer->lookahead == '-') {
      advance(lexer);
    }
  }
  while (is_identifier_char(lexer->lookahead)) {
    advance(lexer);
  }
  return true;
}

/*
 * Consumes one balanced `{ … }`, `( … )` or `[ … ]` group. Strings, URIs and
 * comments inside the group are skipped, so their brackets do not count.
 * Every bracket kind shares one depth counter: a valid program nests its
 * brackets properly, and for an invalid program any answer is safe.
 */
static bool skip_balanced_group(TSLexer *lexer) {
  uint32_t depth = 0;
  for (;;) {
    if (lexer->eof(lexer)) {
      return false;
    }
    switch (lexer->lookahead) {
      case '{':
      case '(':
      case '[':
        depth++;
        advance(lexer);
        break;
      case '}':
      case ')':
      case ']':
        advance(lexer);
        depth--;
        if (depth == 0) {
          return true;
        }
        break;
      case '"':
        if (!skip_string_literal(lexer)) {
          return false;
        }
        break;
      case '`':
        if (!skip_uri_literal(lexer)) {
          return false;
        }
        break;
      case '/':
        advance(lexer);
        if (lexer->lookahead == '/') {
          advance(lexer);
          skip_line_comment(lexer);
        } else if (lexer->lookahead == '*') {
          advance(lexer);
          if (!skip_block_comment(lexer)) {
            return false;
          }
        }
        break;
      default:
        advance(lexer);
        break;
    }
  }
}

/*
 * Consumes the process of a quoted space name `@P`, when `P` is simple: an
 * identifier outside QUOTE_EXCLUDED_WORDS, `_`, a string, URI or number
 * literal, or one balanced group. The lookahead is the first character of
 * `P`.
 */
static bool scan_quoted_process(TSLexer *lexer) {
  switch (lexer->lookahead) {
    case '"':
      return skip_string_literal(lexer);
    case '`':
      return skip_uri_literal(lexer);
    case '{':
    case '(':
    case '[':
      return skip_balanced_group(lexer);
    case '_':
      advance(lexer);
      while (is_identifier_char(lexer->lookahead)) {
        advance(lexer);
      }
      return true;
    default:
      if (is_ascii_digit(lexer->lookahead)) {
        return skip_number_literal(lexer);
      }
      return scan_identifier(lexer, &QUOTE_EXCLUSIONS) == IDENTIFIER_ACCEPTED;
  }
}

/*
 * Consumes a space name: an identifier outside USE_EXCLUDED_WORDS, the
 * wildcard `_`, or `@` and a simple quoted process.
 */
static bool scan_space_name(TSLexer *lexer) {
  switch (lexer->lookahead) {
    case '@':
      advance(lexer);
      if (skip_separators(lexer) == SEPARATORS_INVALID) {
        return false;
      }
      return scan_quoted_process(lexer);
    case '_':
      advance(lexer);
      while (is_identifier_char(lexer->lookahead)) {
        advance(lexer);
      }
      return true;
    default:
      return scan_identifier(lexer, &USE_EXCLUSIONS) == IDENTIFIER_ACCEPTED;
  }
}

/* `use <separators> <space name> <separators?> {` */
static bool scan_use_keyword(TSLexer *lexer) {
  if (!consume_keyword(lexer, "use")) {
    return false;
  }
  lexer->mark_end(lexer);
  if (skip_separators(lexer) != SEPARATORS_FOUND) {
    return false;
  }
  if (!scan_space_name(lexer)) {
    return false;
  }
  if (skip_separators(lexer) == SEPARATORS_INVALID) {
    return false;
  }
  if (lexer->lookahead != '{') {
    return false;
  }
  lexer->result_symbol = USE_KEYWORD;
  return true;
}

/* `free <separators> <identifier> <separators?> (` */
static bool scan_free_keyword(TSLexer *lexer) {
  if (!consume_keyword(lexer, "free")) {
    return false;
  }
  lexer->mark_end(lexer);
  if (skip_separators(lexer) != SEPARATORS_FOUND) {
    return false;
  }
  if (scan_identifier(lexer, &FREE_EXCLUSIONS) != IDENTIFIER_ACCEPTED) {
    return false;
  }
  if (skip_separators(lexer) == SEPARATORS_INVALID) {
    return false;
  }
  if (lexer->lookahead != '(') {
    return false;
  }
  lexer->result_symbol = FREE_KEYWORD;
  return true;
}

/*
 * `-?\d+[eE][+-]?\d+` not followed by an identifier character. A following
 * identifier character means that 02cef80 lexed the text as an integer and an
 * identifier (`1e5x`) or as a longer literal (`1e5f64`), so the internal
 * lexer must decide it.
 */
static bool scan_bare_exponent_float(TSLexer *lexer) {
  if (lexer->lookahead == '-') {
    advance(lexer);
  }
  if (!is_ascii_digit(lexer->lookahead)) {
    return false;
  }
  while (is_ascii_digit(lexer->lookahead)) {
    advance(lexer);
  }
  if (lexer->lookahead != 'e' && lexer->lookahead != 'E') {
    return false;
  }
  advance(lexer);
  if (lexer->lookahead == '+' || lexer->lookahead == '-') {
    advance(lexer);
  }
  if (!is_ascii_digit(lexer->lookahead)) {
    return false;
  }
  while (is_ascii_digit(lexer->lookahead)) {
    advance(lexer);
  }
  if (is_identifier_char(lexer->lookahead)) {
    return false;
  }
  lexer->mark_end(lexer);
  lexer->result_symbol = BARE_EXPONENT_FLOAT;
  return true;
}

void *tree_sitter_rholang_external_scanner_create(void) { return NULL; }

void tree_sitter_rholang_external_scanner_destroy(void *payload) {
  (void)payload;
}

unsigned tree_sitter_rholang_external_scanner_serialize(void *payload,
                                                        char *buffer) {
  (void)payload;
  (void)buffer;
  return 0;
}

void tree_sitter_rholang_external_scanner_deserialize(void *payload,
                                                      const char *buffer,
                                                      unsigned length) {
  (void)payload;
  (void)buffer;
  (void)length;
}

bool tree_sitter_rholang_external_scanner_scan(void *payload, TSLexer *lexer,
                                               const bool *valid_symbols) {
  (void)payload;
  /*
   * During error recovery tree-sitter marks every external token valid, and
   * the sentinel is valid only then. Recovery never needs these tokens.
   */
  if (valid_symbols[ERROR_RECOVERY_SENTINEL]) {
    return false;
  }
  bool use_valid = valid_symbols[USE_KEYWORD];
  bool free_valid = valid_symbols[FREE_KEYWORD];
  bool float_valid = valid_symbols[BARE_EXPONENT_FLOAT];
  if (!use_valid && !free_valid && !float_valid) {
    return false;
  }
  while (is_whitespace(lexer->lookahead)) {
    lexer->advance(lexer, true);
  }
  int32_t first = lexer->lookahead;
  if (use_valid && first == 'u') {
    return scan_use_keyword(lexer);
  }
  if (free_valid && first == 'f') {
    return scan_free_keyword(lexer);
  }
  if (float_valid && (first == '-' || is_ascii_digit(first))) {
    return scan_bare_exponent_float(lexer);
  }
  return false;
}
