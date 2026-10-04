#!/usr/bin/env node
'use strict';

// derive_scanner_exclusions.js
// Derives the three exclusion tables of the external scanner
// (rholang-tree-sitter/src/scanner.c) from a tree-sitter grammar.json, and
// checks the facts that make the scanner's lookahead rule safe.
//
// Usage:
//   node scripts/derive_scanner_exclusions.js [--self-test] BASELINE_GRAMMAR_JSON [CURRENT_GRAMMAR_JSON]
//
// The baseline is the 02cef80 grammar.json: the scanner must never change a
// program that parses there. The start symbol is `_proc` (one top-level
// process, the only source shape that the node accepts).
//
// The scanner emits `use` only before `<space name> {` and `free` only before
// `<theory name> (` (see grammar.js). A word W must be excluded exactly when
// that text can continue a 02cef80 program in which `use` or `free` is a
// variable:
//   use   keywords W with the token pair `W {` in FOLLOW_2(_proc)
//         (`use` completes a process, then `W {` follows);
//   free  keywords W with the pair `W (` in FOLLOW_2(_proc_expression)
//         (`free` completes an expression, then `W (` follows);
//   quote keywords W with the pair `W {` in FIRST_2(_proc)
//         (`use @W {`: after `use`, a case pattern starts with `@W {`).
// A keyword is a string token of the grammar that is a whole `var`.
//
// Checks (each failure exits with status 1):
//   1. FOLLOW_2(_proc) has no pair `var {` or `_ {`: a space name that is not
//      a keyword never continues a 02cef80 process;
//   2. FOLLOW_2(_proc_expression) has no pair `var (`;
//   3. FIRST_2(_proc) has no pair `t {` where `t` is `var`, `_` or a literal;
//   4. no string token continues an identifier prefix with `{` or `(` (the
//      scanner reads only the prefix of `bundle+`, then sees `+`);
//   5. the current grammar, when given, adds no keyword to the baseline;
//   6. a name start (`var`, `_`, `@`) follows a `var` token only through
//      match-case and select-branch adjacency: without that adjacency,
//      FOLLOW_1(var) has no name start;
//   7. a complete quoted name that starts a match case or a select branch is
//      never followed by `{` (computed with `quote` as one opaque token);
//   8. a bracket construct (block, map, pathmap, list, tuple, unit,
//      parenthesized process, signed term) that starts a process is never
//      followed by `{` (computed with each construct as one opaque token);
//   9. FOLLOW_2(var), over every position of a `var` token, gives the same
//      `use` and `free` tables.
// Checks 6-8 complete the argument for the space names that are not
// keywords: after `use`, a name starts only a match case or a select branch
// (6); a quoted name ends before `{` (7) unless its process continues with
// `{` right after a leading atom (3) or bracket construct (8).
//
// Output: the lines `use: …`, `free: …` and `quote: …` (sorted), then one
// line per check.
//
// --self-test also runs negative controls: with `source_file` as the start
// symbol, top-level adjacency (`source_file` is `repeat($._proc)`) makes the
// pairs `var {`, `_ {` and `var (` reachable, and checks 1 and 2 must find
// them; with the adjacency kept, check 6 must find name starts; and checks 7
// and 8 must find a `{` that a synthetic process alternative adds.

const fs = require('fs');

const WORD = /^(?:[a-zA-Z][a-zA-Z0-9_']*|_[a-zA-Z0-9_']+)$/;
const SEPARATOR = '\u0001';
const LITERALS = new Set([
  'string_literal', 'uri_literal', 'long_literal', 'float_literal',
  'signed_int_literal', 'unsigned_int_literal', 'bigint_literal',
  'bigrat_literal', 'fixed_point_literal',
]);

// FIRST_k and FOLLOW_k of every rule of `grammar` for the start symbol. With
// `follow: false`, the FOLLOW sets stay empty (the FIRST sets are cheaper).
function analyse(grammar, k, start, { follow = true } = {}) {
  const rules = grammar.rules;
  const externals = new Set((grammar.externals || []).map((e) => e.name));
  const strip = (e) => {
    while (['PREC', 'PREC_LEFT', 'PREC_RIGHT', 'PREC_DYNAMIC', 'FIELD', 'ALIAS', 'RESERVED'].includes(e.type)) {
      e = e.content;
    }
    return e;
  };
  const isToken = (name) => ['TOKEN', 'IMMEDIATE_TOKEN', 'PATTERN'].includes(strip(rules[name]).type);
  const terminal = (e) => {
    if (e.type === 'STRING') return `'${e.value}'`;
    if (e.type === 'SYMBOL' && (externals.has(e.name) || isToken(e.name))) return e.name;
    if (['TOKEN', 'IMMEDIATE_TOKEN', 'PATTERN'].includes(e.type)) return '<anonymous>';
    return null;
  };
  const split = (s) => (s === '' ? [] : s.split(SEPARATOR));
  const concat = (a, b) => {
    const out = new Set();
    for (const x of a) {
      const px = split(x);
      if (px.length >= k) { out.add(x); continue; }
      for (const y of b) out.add(px.concat(split(y)).slice(0, k).join(SEPARATOR));
    }
    return out;
  };
  const FIRST = {};
  for (const name of Object.keys(rules)) FIRST[name] = new Set();
  const first = (e) => {
    const t = terminal(e);
    if (t) return new Set([t]);
    switch (e.type) {
      case 'BLANK': return new Set(['']);
      case 'SYMBOL': return FIRST[e.name];
      case 'SEQ': return e.members.reduce((acc, m) => concat(acc, first(m)), new Set(['']));
      case 'CHOICE': return new Set(e.members.flatMap((m) => [...first(m)]));
      case 'REPEAT': {
        let s = new Set(['']);
        const f = first(e.content);
        for (;;) {
          const n = new Set([...s, ...concat(f, s)]);
          if (n.size === s.size) return s;
          s = n;
        }
      }
      case 'REPEAT1': return concat(first(e.content), first({ type: 'REPEAT', content: e.content }));
      default: return first(e.content);
    }
  };
  const addAll = (set, items) => {
    let changed = false;
    for (const x of items) if (!set.has(x)) { set.add(x); changed = true; }
    return changed;
  };
  for (let changed = true; changed;) {
    changed = false;
    for (const name of Object.keys(rules)) {
      if (!isToken(name) && addAll(FIRST[name], first(rules[name]))) changed = true;
    }
  }
  const FOLLOW = {};
  for (const name of Object.keys(rules)) FOLLOW[name] = new Set();
  FOLLOW[start].add('$end');
  let changed = follow;
  const walk = (e, follow) => {
    // A token symbol (such as `var`) also gets a FOLLOW set (check 6).
    if (e.type === 'SYMBOL' && FOLLOW[e.name] && isToken(e.name) && addAll(FOLLOW[e.name], follow)) changed = true;
    if (terminal(e)) return;
    switch (e.type) {
      case 'BLANK': return;
      case 'SYMBOL': if (addAll(FOLLOW[e.name], follow)) changed = true; return;
      case 'SEQ': {
        let rest = follow;
        for (let i = e.members.length - 1; i >= 0; i--) {
          walk(e.members[i], rest);
          rest = concat(first(e.members[i]), rest);
        }
        return;
      }
      case 'CHOICE': for (const m of e.members) walk(m, follow); return;
      case 'REPEAT':
      case 'REPEAT1': walk(e.content, concat(first({ type: 'REPEAT', content: e.content }), follow)); return;
      default: walk(e.content, follow);
    }
  };
  const reachable = new Set([start]);
  (function mark(e) {
    if (!e) return;
    if (e.type === 'SYMBOL') {
      if (!reachable.has(e.name) && rules[e.name]) { reachable.add(e.name); mark(rules[e.name]); }
      return;
    }
    for (const m of e.members || []) mark(m);
    if (e.content) mark(e.content);
  })(rules[start]);
  while (changed) {
    changed = false;
    for (const name of reachable) if (!isToken(name)) walk(rules[name], FOLLOW[name]);
  }
  return {
    first: (rule) => [...FIRST[rule]].map(split),
    follow: (rule) => [...FOLLOW[rule]].map(split),
  };
}

// A copy of `grammar` in which each rule of `names` is one opaque string
// token `<name>`.
function collapse(grammar, names) {
  const copy = JSON.parse(JSON.stringify(grammar));
  for (const name of names) copy.rules[name] = { type: 'STRING', value: `<${name}>` };
  return copy;
}

// A copy of `grammar` in which every repetition of `case` or `branch` is one
// occurrence: match cases and select branches are no longer adjacent.
function withoutAdjacency(grammar) {
  const single = (e) => {
    if (!e || typeof e !== 'object') return e;
    if (['REPEAT', 'REPEAT1'].includes(e.type) && e.content.type === 'SYMBOL'
      && ['case', 'branch'].includes(e.content.name)) return e.content;
    const out = { ...e };
    if (out.members) out.members = out.members.map(single);
    if (out.content) out.content = single(out.content);
    return out;
  };
  const copy = JSON.parse(JSON.stringify(grammar));
  for (const name of Object.keys(copy.rules)) copy.rules[name] = single(copy.rules[name]);
  return copy;
}

// A copy of `grammar` with one more alternative `members` of `_proc`.
function withProcAlternative(grammar, members) {
  const copy = JSON.parse(JSON.stringify(grammar));
  copy.rules._proc = { type: 'CHOICE', members: [copy.rules._proc, { type: 'SEQ', members }] };
  return copy;
}

const NAME_STARTS = ['var', "'_'", "'@'"];
const BRACKET_CONSTRUCTS = ['block', 'map', 'pathmap', 'list', 'tuple', 'unit', '_parenthesized', 'signed_term'];

// Check 6: the name starts in FOLLOW_1(var).
const nameStartsAfterVar = (grammar) => {
  const after = new Set(analyse(grammar, 1, '_proc').follow('var').map(([t]) => t));
  return NAME_STARTS.filter((t) => after.has(t));
};

// Check 7: the pairs `<quote> {` that start a match case or a select branch.
const quoteBraces = (grammar) => {
  const collapsed = analyse(collapse(grammar, ['quote']), 2, '_proc', { follow: false });
  return ['case', 'branch'].flatMap((rule) => collapsed.first(rule)
    .filter(([t1, t2]) => t1 === "'<quote>'" && t2 === "'{'")
    .map((p) => `${rule}: ${p.join(' ')}`));
};

// Check 8: the pairs `<construct> {` that start a process, and the bracket
// tokens that still start a process after the collapse (there must be none,
// so that the list of constructs is complete).
const bracketBraces = (grammar) => {
  const names = BRACKET_CONSTRUCTS.filter((name) => grammar.rules[name]);
  const firsts = analyse(collapse(grammar, names), 2, '_proc', { follow: false }).first('_proc');
  return [
    ...firsts.filter(([t1, t2]) => names.some((n) => t1 === `'<${n}>'`) && t2 === "'{'").map((p) => p.join(' ')),
    ...[...new Set(firsts.map(([t1]) => t1).filter((t) => ["'{'", "'('", "'['", "'{|'", "'{%'"].includes(t)))]
      .map((t) => `uncollapsed ${t}`),
  ];
};

// The identifier-shaped string tokens of a grammar, and all its strings.
function strings(grammar) {
  const all = new Set();
  (function collect(e) {
    if (!e) return;
    if (e.type === 'STRING') all.add(e.value);
    for (const m of e.members || []) collect(m);
    if (e.content) collect(e.content);
  })({ type: 'SEQ', members: Object.values(grammar.rules) });
  return { keywords: new Set([...all].filter((s) => WORD.test(s))), all };
}

const args = process.argv.slice(2);
const selfTest = args[0] === '--self-test';
const [baselinePath, currentPath] = selfTest ? args.slice(1) : args;
if (!baselinePath) {
  console.error('usage: derive_scanner_exclusions.js [--self-test] BASELINE_GRAMMAR_JSON [CURRENT_GRAMMAR_JSON]');
  process.exit(2);
}
const baseline = JSON.parse(fs.readFileSync(baselinePath, 'utf8'));
const baselineStrings = strings(baseline);
const keywordOf = (t) => (t.startsWith("'") && baselineStrings.keywords.has(t.slice(1, -1)) ? t.slice(1, -1) : null);

const analysis = analyse(baseline, 2, '_proc');
const followProc = analysis.follow('_proc');
const followExpression = analysis.follow('_proc_expression');
const firstProc = analysis.first('_proc');

// The keywords W of the pairs `W next`.
const wordsBefore = (pairs, next) => [...new Set(pairs
  .filter(([t1, t2]) => t2 === `'${next}'` && keywordOf(t1) !== null)
  .map(([t1]) => keywordOf(t1)))].sort();
const tables = {
  use: wordsBefore(followProc, '{'),
  free: wordsBefore(followExpression, '('),
  quote: wordsBefore(firstProc, '{'),
};
for (const [name, words] of Object.entries(tables)) console.log(`${name}: ${words.join(' ')}`);

let failed = false;
const report = (ok, label, detail) => {
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${label}${detail ? `: ${detail}` : ''}`);
  if (!ok) failed = true;
};
const pairsBefore = (pairs, firsts, next) => pairs
  .filter(([t1, t2]) => firsts.includes(t1) && t2 === `'${next}'`)
  .map((p) => p.join(' '));

const spaceHits = pairsBefore(followProc, ['var', "'_'"], '{');
report(spaceHits.length === 0, 'FOLLOW_2(_proc) has no `var {` or `_ {` pair', spaceHits.join(', '));
const theoryHits = pairsBefore(followExpression, ['var'], '(');
report(theoryHits.length === 0, 'FOLLOW_2(_proc_expression) has no `var (` pair', theoryHits.join(', '));
const atomHits = firstProc
  .filter(([t1, t2]) => (t1 === 'var' || t1 === "'_'" || LITERALS.has(t1)) && t2 === "'{'")
  .map((p) => p.join(' '));
report(atomHits.length === 0, 'FIRST_2(_proc) has no `atom {` pair', atomHits.join(', '));
const prefixHits = [...baselineStrings.all].filter((s) => {
  const prefix = s.match(/^[a-zA-Z_][a-zA-Z0-9_']*/);
  return prefix !== null && prefix[0] !== s && ['{', '('].includes(s[prefix[0].length]);
});
report(prefixHits.length === 0, 'no token continues an identifier prefix with `{` or `(`', prefixHits.join(' '));

const followVar = analysis.follow('var');
const varTables = { use: wordsBefore(followVar, '{'), free: wordsBefore(followVar, '(') };
const sameTables = ['use', 'free'].every((form) => varTables[form].join(' ') === tables[form].join(' '));
report(sameTables, 'FOLLOW_2(var) gives the same `use` and `free` tables',
  sameTables ? '' : `use: ${varTables.use.join(' ')}; free: ${varTables.free.join(' ')}`);
const adjacencyFree = nameStartsAfterVar(withoutAdjacency(baseline));
report(adjacencyFree.length === 0,
  'a name start follows `var` only through match-case and select-branch adjacency', adjacencyFree.join(' '));
const quoteHits = quoteBraces(baseline);
report(quoteHits.length === 0, 'a quoted name that starts a case or a branch is never followed by `{`', quoteHits.join(', '));
const bracketHits = bracketBraces(baseline);
report(bracketHits.length === 0, 'a bracket construct that starts a process is never followed by `{`', bracketHits.join(', '));

if (currentPath) {
  const current = JSON.parse(fs.readFileSync(currentPath, 'utf8'));
  const added = [...strings(current).keywords].filter((w) => !baselineStrings.keywords.has(w)).sort();
  report(added.length === 0, 'the current grammar adds no keyword', added.join(' '));
}

if (selfTest) {
  const topLevel = analyse(baseline, 2, 'source_file');
  const found = [
    ...pairsBefore(topLevel.follow('_proc'), ['var', "'_'"], '{'),
    ...pairsBefore(topLevel.follow('_proc_expression'), ['var'], '('),
  ].sort();
  const expected = ["'_' '{'", "var '{'", "var '('"];
  report(expected.every((p) => found.includes(p)),
    'self-test: top-level adjacency is detected', found.join(', '));
  const adjacent = nameStartsAfterVar(baseline);
  report(adjacent.length === NAME_STARTS.length,
    'self-test: name starts after `var` through adjacency are detected', adjacent.join(' '));
  const brace = { type: 'STRING', value: '{' };
  const quoteControl = quoteBraces(withProcAlternative(baseline, [{ type: 'SYMBOL', name: 'quote' }, brace]));
  report(quoteControl.length > 0, 'self-test: a synthetic `@P {` process is detected', quoteControl.join(', '));
  const blockControl = bracketBraces(withProcAlternative(baseline, [{ type: 'SYMBOL', name: 'block' }, brace]));
  report(blockControl.length > 0, 'self-test: a synthetic `{ P } {` process is detected', blockControl.join(', '));
}
process.exit(failed ? 1 : 0);
