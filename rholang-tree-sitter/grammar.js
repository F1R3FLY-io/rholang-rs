module.exports = grammar({
    name: 'rholang',

    extras: $ => [
        $._line_comment,
        $._block_comment,
        /\s/,
    ],

    // ─── Contextual keywords `use` and `free` (Reified RSpaces) ─────────────
    //
    // `use` and `free` are not reserved. At 02cef80 they are ordinary
    // identifiers (`use!(1)`, `contract f(use) = { … }`, `free | Nil`), and
    // every program that parses at 02cef80 must keep its exact syntax tree.
    // The external scanner (src/scanner.c) therefore emits `_use_keyword` and
    // `_free_keyword` only when the text that follows can never continue a
    // valid 02cef80 process. In every other case it returns false, and the
    // internal lexer produces an ordinary `var`.
    //
    // `use` is a keyword only when all of these hold:
    //   1. `_use_keyword` is valid in the current parse state, and the parser
    //      is not in error recovery (`_error_recovery_sentinel` is used by no
    //      rule, so it is valid only when tree-sitter marks every external
    //      token valid during error recovery);
    //   2. the text is `use`, and the next character cannot continue an
    //      identifier ([a-zA-Z0-9_']);
    //   3. at least one whitespace character or comment follows;
    //   4. a space name follows: an identifier that is not a `use W {`
    //      exclusion, `_`, or `@` and a simple quoted process (an identifier
    //      that is not a `use @W {` exclusion, `_`, a string, URI or number
    //      literal, or one balanced `{ … }`, `( … )` or `[ … ]` group);
    //   5. after optional whitespace and comments, the next character is `{`.
    // `free` is a keyword only when conditions 1–3 hold for `free`, an
    // identifier that is not a `free W (` exclusion follows, and then, after
    // optional whitespace and comments, the next character is `(`.
    //
    // Why the lookahead reaches the `{` or the `(`: `cases` (match) and
    // `branches` (select) are repetitions with no separator, so inside one
    // 02cef80 process a `var` can be followed by another `var`, by `_`, or
    // by `@`:
    //   match e { 1 => use _ => Nil }          (body `use`, next pattern `_`)
    //   match e { 1 => free x => Nil }         (body `free`, next pattern `x`)
    //   select { a <- b => free c <- d => P }  (next branch starts with `c`)
    // This adjacency is the only way that a name can follow a `var`. A name
    // followed by `{`, or an identifier followed by `(`, never continues a
    // 02cef80 process. These facts of the 02cef80 grammar.json, with `_proc`
    // as the start symbol, show it:
    //   - FOLLOW_2(_proc) has no pair `var {` or `_ {`, and
    //     FOLLOW_2(_proc_expression) has no pair `var (`;
    //   - FIRST_2(_proc) has no pair `var {`, `_ {` or `<literal> {`, and no
    //     bracket construct that starts a process is followed by `{`, so a
    //     simple quoted process ends before the `{`;
    //   - a complete quoted name that starts a case is followed by `!`, `!!`
    //     or `!?`, and one that starts a branch by `,`, `...` or `<-`.
    // scripts/derive_scanner_exclusions.js checks each of these facts.
    //
    // Exclusions. A keyword W is excluded from a form exactly when the form
    // continues a 02cef80 program in which `use` or `free` is a variable.
    // scripts/derive_scanner_exclusions.js derives the three tables from the
    // 02cef80 grammar.json: the pairs `W {` in FOLLOW_2(_proc), `W (` in
    // FOLLOW_2(_proc_expression), and `W {` in FIRST_2(_proc). FOLLOW_2 of
    // the `var` token itself, over every position of a variable, gives the
    // same `use` and `free` tables. Each witness below parses at 02cef80, and
    // test/corpus/conservative_extension.txt pins its 02cef80 tree.
    //   `use W {` (11 words)
    //     and      use and { Nil }
    //     or       use or { Nil }
    //     matches  use matches { Nil }
    //     else     if (c) use else { Nil }
    //     in       let x = use in { x }
    //     where    match e { use where { true } => Nil }
    //     bundle   match e { 1 => use bundle { Nil } => Nil }
    //     bundle0  match e { 1 => use bundle0 { Nil } => Nil }
    //     match    match e { 1 => use match { x } { _ => Nil } => Nil }
    //     not      match e { 1 => use not { true } => Nil }
    //     select   match e { 1 => use select { a <- b => Nil } => Nil }
    //   `free W (` (11 words)
    //     and      free and (true)
    //     or       free or (true)
    //     matches  free matches (true)
    //     else     if (c) free else (Nil)
    //     in       let x = free in (x)
    //     where    match e { free where (true) => Nil }
    //     Set      match e { 1 => free Set(1) => Nil }
    //     if       match e { 1 => free if (c) Nil => Nil }
    //     for      match e { 1 => free for (x <- c) { Nil } => Nil }
    //     not      match e { 1 => free not (true) => Nil }
    //     match    match e { 1 => free match (x) { _ => Nil } => Nil }
    //   `use @W {` (5 words)
    //     bundle   match e { 1 => use @bundle { Nil }!(1) => Nil }
    //     bundle0  match e { 1 => use @bundle0 { Nil }!(1) => Nil }
    //     match    match e { 1 => use @match { x } { _ => Nil }!(1) => Nil }
    //     not      match e { 1 => use @not { true }!(1) => Nil }
    //     select   match e { 1 => use @select { a <- b => Nil }!(1) => Nil }
    // The infix keywords (`and`, `or`, `matches`) and the continuation
    // keywords (`else`, `in`, `where`) follow a complete process. The other
    // excluded words start a process whose second token is `{` or `(`, and
    // match-case adjacency puts such a process right after a complete one.
    //
    // Every other keyword is accepted, because the form can never continue a
    // 02cef80 program. A simple type (`Bool`, `Int`, `String`, `Uri`,
    // `ByteArray`), `Nil`, `true` and `false` are complete atoms, and `{` or
    // `(` never follows an atom. `new`, `let`, `contract`, `agent` and `try`
    // are followed by declarations, a name or `<-`. `Set`, `if` and `for` are
    // followed only by `(`, and `bundle`, `bundle0` and `select` only by `{`.
    // `catch`, `constructor`, `default`, `method` and `private` never follow
    // a variable. So `free Int()` and `use Set { P }` are keyword forms. A
    // reserved word in the name position (`use new { P }`) stays a parse
    // error, as at 02cef80.
    //
    // Limits:
    //   - `source_file` is `repeat($._proc)`, so at top level 02cef80 parses
    //     `use s { P }` as the three processes `use`, `s` and `{ P }`, and
    //     `free Th()` as `free`, `Th` and `()`. These sources now parse as
    //     one process. The node rejects every source with more than one
    //     top-level process ("Expected single process"), so no program that
    //     the node accepts changes.
    //   - A compound quoted space name needs a block or parentheses:
    //     `use @{ x /\ y } { P }`, not `use @x /\ y { P }`.
    //   - A space or theory name cannot be an excluded word of its form.
    //
    // The scanner also emits `_bare_exponent_float` (see `float_literal`).
    externals: $ => [
        $._use_keyword,
        $._free_keyword,
        $._bare_exponent_float,
        $._error_recovery_sentinel,
    ],

    word: $ => $.var,

    supertypes: $ => [
        $._let_decls,
        $._bundle,
        $._send_type,
        $._source,
        $._proc_var,
        $._literal],

    inline: $ => [$.name],

    reserved: {
        global: $ => [
            "new",
            "if",
            "else",
            "let",
            "match",
            "select",
            "contract",
            "for",
            "or",
            "and",
            "matches",
            "not",
            "bundle",
            "true",
            "false",
            "where"
            // NOTE: `agent`, `constructor`, `method`, `default`, and
            // `private` are NOT reserved globally. Tree-sitter's GLR
            // parser disambiguates these by context: when followed
            // by the syntactic patterns of an agent_block / agent_decl
            // they're recognized as the keyword form; elsewhere they
            // remain ordinary `var` identifiers. This preserves
            // backward compatibility with existing Rholang code (e.g.
            // SystemVault.rho's `contract _create(@vaultAddress,
            // constructor, retCh) = { ... }` continues to parse).
        ],
    },

    rules: {
        // Starting point of the grammar
        source_file: $ => repeat($._proc),

        _proc: $ => choice(
            $.par,
            $.send_sync,
            $.send_method,
            $.new,
            $.ifElse,
            $.let,
            $.bundle,
            $.match,
            $.choice,
            $.contract,
            $.use_block,
            $.agent_block,
            $.try_block,
            $.input,
            $.send,
            $.signed_term,
            $.token_stack,
            $._proc_expression
        ),

        par: $ => prec.left(0, seq($._proc, '|', $._proc)),

        send_sync: $ => prec(1, seq(
            field('channel', $.name),
            '!?',
            field('inputs', choice(alias($._proc_list, $.messages), $.hyper_inputs)),
            field('cont', $.sync_send_cont))
        ),

        send_method: $ => prec(1, seq(
            field('channel', $.name),
            '!',
            field('method', $.var),
            field('inputs', choice(alias($._proc_list, $.messages), $.hyper_inputs)),
            field('cont', $.sync_send_cont))
        ),

        new: $ => prec(1, seq(
            'new',
            field('decls', $.name_decls),
            'in',
            field('proc', $._proc))),

        ifElse: $ => prec.right(1, seq(
            'if', '(', field('condition', $._proc), ')',
            field('consequence', $._proc),
            optional(seq(
                'else',
                field('alternative', $._proc)))
        )),

        let: $ => prec(2, seq(
            'let',
            field('decls', $._let_decls),
            'in',
            field('proc', $._proc))),

        bundle: $ => prec(2, seq(
            field('bundle_type', $._bundle),
            field('proc', $.block))),

        match: $ => prec(2, seq(
            'match',
            field('expression', $._proc_expression),
            '{',
            field('cases',
                alias(repeat1($.case), $.cases)),
            '}'
        )),

        choice: $ => prec(2, seq(
            'select',
            '{',
            field('branches',
                alias(repeat1($.branch), $.branches)),
            '}'
        )),

        contract: $ => prec(2, seq(
            'contract',
            field('name', $.name),
            '(', optional(field('formals', $.names)), ')',
            '=',
            field('proc', $.block)
        )),

        // Reified RSpaces `use` block: `use s { P }` makes `s` the default
        // space for the untagged channels of `P`. The keyword comes from the
        // external scanner (see `externals`).
        use_block: $ => prec(2, seq(
            alias($._use_keyword, 'use'),
            field('space', $.name),
            field('proc', $.block)
        )),

        // Agent block sugar (FIP 2025-08-20 Agents + 2026-01-28 Private Methods).
        // Desugars at parse time per Agents.md:19-35 to:
        //   for (r, <ctorPtrns> <= fooCtor) {
        //     new this, private in {
        //       for (...@args <= this)    { match args { /* pub  */ } } |
        //       for (...@args <= private) { match args { /* priv */ } } |  (only if private decls)
        //       Pc |
        //       r!(bundle+{*this})
        //     }
        //   }
        agent_block: $ => prec(2, seq(
            'agent',
            field('name', $.name),
            '{',
            field('decls', $.agent_decls),
            '}'
        )),

        agent_decls: $ => seq(
            $.agent_decl,
            repeat(seq('|', $.agent_decl))
        ),

        agent_decl: $ => choice(
            $.constructor_decl,
            $.method_decl,
            $.default_decl
        ),

        constructor_decl: $ => seq(
            'constructor',
            '(', optional(field('formals', $.names)), ')',
            field('body', $.block)
        ),

        method_decl: $ => seq(
            optional(field('private', 'private')),
            'method',
            field('name', $.var),
            '(', optional(field('formals', $.names)), ')',
            field('body', $.block)
        ),

        default_decl: $ => seq(
            optional(field('private', 'private')),
            'default',
            '(', optional(field('formals', $.names)), ')',
            field('body', $.block)
        ),

        // try/catch sugar (FIP 2026-02-06 File-I/O §"Error syntax").
        //
        //   try @<try_pat>? <- <source> { <try_body> }
        //   catch @<catch_pat> { <catch_body> }
        //
        // Desugars at parse time to:
        //   for (@[ok, ...rest] <- <source>) {
        //     if (ok) {
        //       let @<try_pat> <- rest in { <try_body> }
        //     } else {
        //       let @<catch_pat> <- rest in { <catch_body> }
        //     }
        //   }
        //
        // If the try line has no `@<try_pat>` (e.g. for methods returning
        // just `[true]`), the success branch drops the let and runs the
        // try body directly.
        //
        // No `finally` clause: Rholang branch bodies may be asynchronous
        // (a body that ends by starting a for-receive doesn't complete
        // when the `{...}` textually ends), so a keyword-based finally
        // would silently race the branch. Programmers who want
        // after-both-branches semantics use an explicit `done` channel
        // in each branch and a `for(<- done)` receiver in parallel with
        // the try block. See the FIP for the pattern.
        //
        // `try` and `catch` are NOT globally reserved for the same
        // backward-compat reason as `agent` / `constructor`: existing
        // Rholang may use these as ordinary identifiers. GLR
        // disambiguation kicks in when the following tokens match the
        // try_block shape.
        try_block: $ => seq(
            'try',
            optional(field('try_pattern', $.name)),
            '<-',
            field('source', $._source),
            field('try_body', $.block),
            'catch',
            field('catch_pattern', $.name),
            field('catch_body', $.block)
        ),

        input: $ => prec(2, seq(
            'for', '(', field('receipts', $.receipts), ')',
            field('proc', choice($.block, $.signed_cont))
        )),

        // Greg app:concrete PSignedInput: a signed receive continuation
        // `for(...) {% SignedProc %}` (the body is a signed process, no trailing
        // `[s]` — the wrapper-with-signature `{% P %}[s]` is the OUTER signed_term).
        signed_cont: $ => prec(2, seq('{%', field('proc', $._proc), '%}')),

        send: $ => prec(3, seq(
            field('channel', $.name),
            field('send_type', $._send_type),
            field('inputs', choice(alias($._proc_list, $.inputs), $.hyper_inputs))
        )),

        // Reified RSpaces hyperparameters (the parens-`;` rule).
        //
        // A parenthesized list with a `;` is `hyper_inputs`: data before the
        // `;`, parameters after it. Without a `;` the list keeps the 02cef80
        // shape (`inputs`/`messages`), so the tree of every ordinary send is
        // unchanged:
        //   x!(a, b)            inputs (a, b)                no parameters
        //   x!(a, b;)           data (a, b), empty section   parameters []
        //   x!(a; p, k = v)     data (a), positional p, named k = v
        //   x!(; k = v)         no data, named k = v
        // The same rule holds for `!!`, `!?`, the method sugar `x!m(…)`, and
        // the `!?` and `!m` receive sources. A receive query uses it too
        // (see `query_spec`): `ch ~ (a, b)` queries the one tuple `(a, b)`,
        // and `ch ~ (a, b;)` queries the two payload elements `a` and `b`.
        //
        // The grammar accepts any process as a parameter key, and positional
        // and named parameters in any order. The AST builder rejects a key
        // that is not an identifier (`NonIdentifierParamKey`), a positional
        // parameter after a named one (`PositionalAfterNamed`), and a repeated
        // key (`DuplicateParamKey`), so these errors carry precise spans.
        hyper_inputs: $ => seq(
            '(',
            optional(field('data', $.send_data)),
            field('params', $.params_section),
            ')'
        ),
        send_data: $ => commaSep1($._proc),
        params_section: $ => seq(';', optional($.param_list)),
        param_list: $ => commaSep1($.param),
        param: $ => choice($.named_param, $.positional_param),
        named_param: $ => prec(2, seq(field('key', $._proc), '=', field('value', $._proc))),
        positional_param: $ => $._proc,

        // ─── Cost-accounted Rholang surface ──────────────────────────────────
        // Constructs + semantics are authoritative in the mettail
        // cost-accounting worktree ({p}_s, () | s:S, g | #P | s*s, meter(K)) and
        // the cost-accounted-rho paper (sugars, joins, located purses). The
        // concrete ASCII delimiters here are a collision-safe rendering (bare
        // {P}_s collides with `block`; bare s:S with `map` key:value).

        // Signed term {P}_s (worktree `Wrap`).
        signed_term: $ => prec(2, seq(
            '{%', field('proc', $._proc), '%}',
            '[', field('sig', $.signature), ']'
        )),

        // Token stack S ::= () | s :: S  (worktree `SNil`/`SCons`; Greg app:concrete
        // `Stk`). A bare stack is DIRECTLY a signed process (no `purse(...)` wrapper);
        // located stacks are ordinary parallel composition `S1 | S2`, and ring-fencing
        // is via `new`-bound signatures (binding-sensitive Σ⟦s⟧ in the normalizer). The
        // proc-level stack is cons-headed (`s :: …`); a bare empty `()` is plain unit/Nil.
        token_stack: $ => prec.right(2, seq(
            field('head', $.stack_sig), '::', field('tail', $._stack_tail)
        )),
        _stack_tail: $ => choice($.unit, $.token_stack),
        // A token-stack layer is a fundable signature — ground / `# P` / compound —
        // but NOT a lollipop (a transfer capability is not a fundable atom; the
        // normalizer rejects `Transfer` in a stack). Excluding `-o` here also keeps
        // `-o` out of the bare-process lexer state, so `x-owed` still lexes as `x - owed`.
        stack_sig: $ => $._sig1,

        // Signatures s ::= g | # P | s1 (*) s2 | s1 -o s2  (Greg app:concrete:
        // SigGround/SigHash/SigCompound/SigTransfer). `# P` takes a high-precedence
        // principal (Proc12 — use `# { P }` to hash a complex process); compound uses
        // the distinct `(*)` lexeme (bare `*` is dereference/multiply, hence reserved);
        // `-o` is the lollipop. Precedence: ground/# tightest, (*) tighter, -o loosest.
        // Stratified per Greg's BNFC: Sig ::= Sig1 -o Sig | Sig1 ; Sig1 ::= Sig1 (*) Sig2
        // | Sig2 ; Sig2 ::= ground | # P. The lollipop `-o` lives ONLY at the top Sig
        // (reachable only inside the `[ … ]` of a signed term/bind), so it never
        // enters the bare-process lexer state — `x-owed` still lexes as `x - owed`.
        signature: $ => choice($.transfer_sig, $._sig1),
        _sig1: $ => choice($.compound_sig, $._sig2),
        _sig2: $ => choice($.ground_sig, $.hash_sig),
        ground_sig: $ => prec(20, field('name', $._proc_var)),
        hash_sig: $ => prec.right(19, seq('#', field('proc', choice($._ground_expression, $.eval)))),
        compound_sig: $ => prec.left(17, seq(
            field('left', $._sig1), '(*)', field('right', $._sig2)
        )),
        transfer_sig: $ => prec.right(16, seq(
            field('left', $._sig1), '-o', field('right', $.signature)
        )),

        _proc_expression: $ => choice(
            $._ground_expression,
            $._parenthesized,
            $.add,
            $.and,
            $.concat,
            $.conjunction,
            $.diff,
            $.disjunction,
            $.div,
            $.eq,
            $.eval,
            $.free_call,
            $.gt,
            $.gte,
            $.interpolation,
            $.lt,
            $.lte,
            $.matches,
            $.method,
            $.mod,
            $.mult,
            $.neg,
            $.negation,
            $.neq,
            $.not,
            $.or,
            $.sub,
            $.var_ref
        ),

        // expressions in precedence order

        or: $ => prec.left(4, seq($._proc, 'or', $._proc)),
        and: $ => prec.left(5, seq($._proc, 'and', $._proc)),
        matches: $ => prec.right(6, seq($._proc, 'matches', $._proc)),
        eq: $ => prec.left(6, seq($._proc, '==', $._proc)),
        neq: $ => prec.left(6, seq($._proc, '!=', $._proc)),
        lt: $ => prec.left(7, seq($._proc, '<', $._proc)),
        lte: $ => prec.left(7, seq($._proc, '<=', $._proc)),
        gt: $ => prec.left(7, seq($._proc, '>', $._proc)),
        gte: $ => prec.left(7, seq($._proc, '>=', $._proc)),
        concat: $ => prec.left(8, seq($._proc, '++', $._proc)),
        diff: $ => prec.left(8, seq($._proc, '--', $._proc)),
        add: $ => prec.left(8, seq($._proc, '+', $._proc)),
        sub: $ => prec.left(8, seq($._proc, '-', $._proc)),
        interpolation: $ => prec.left(9, seq($._proc, '%%', $._proc)),
        mult: $ => prec.left(9, seq($._proc, '*', $._proc)),
        div: $ => prec.left(9, seq($._proc, '/', $._proc)),
        mod: $ => prec.left(9, seq($._proc, '%', $._proc)),
        not: $ => prec(10, seq('not', $._proc)),
        neg: $ => prec(10, seq('-', $._proc)),
        _parenthesized: $ => prec(11, seq('(', $._proc_expression, ')')),
        method: $ => prec(11, seq(
            field('receiver', $._proc),
            '.',
            field('name', $.var),
            field('args', alias($._proc_list, $.args)))
        ),
        eval: $ => prec(12, seq('*', $.name)),
        var_ref: $ => prec(13, seq(field('kind', $.var_ref_kind), field('var', $.var))),
        var_ref_kind: $ => choice('=', '=*'),
        disjunction: $ => prec.left(13, seq($._proc, '\\/', $._proc)),
        conjunction: $ => prec.left(14, seq($._proc, '/\\', $._proc)),
        negation: $ => prec(15, seq('~', $._proc)),
        // Reified RSpaces theory reference `free Th(args)` (the tree-sitter
        // rendering of `free theory_call`; the AST node is
        // `Proc::FreeTheory`). The keyword comes from the external scanner
        // (see `externals`). The builtin theories `Nat`, `Int`, `String`,
        // `Bool` and `Any` all parse here: a type word such as `Int` lexes as
        // `var` in this position, because a contextual (unreserved) keyword
        // becomes a keyword token only in a parse state where it has an
        // action, and a theory name accepts only `var`.
        free_call: $ => prec(10, seq(
            alias($._free_keyword, 'free'),
            field('name', $.var),
            '(', optional(field('args', $.free_args)), ')'
        )),
        free_args: $ => commaSep1($._proc),
        _ground_expression: $ => prec(16, choice($.block, $._literal, $.nil, $.collection, $._proc_var, $.simple_type, $.unit)),

        // synchronous send continuations
        sync_send_cont: $ => choice($.empty_cont, $.non_empty_cont),

        non_empty_cont: $ => prec(1, seq(';', $._proc)),
        empty_cont: $ => '.',

        // new name declaration
        name_decls: $ => commaSep1($.name_decl),
        // A binder is plain (`x`), annotated with a Reified RSpaces space
        // (`x : s`, where `s` is any name, quoted names included), or bound
        // to a system URI (`x(`rho:io:stdout`)`). A URI binder names a fixed
        // system channel, so it cannot carry a space annotation: both
        // `x : s(`uri`)` and `x(`uri`) : s` are syntax errors.
        name_decl: $ => seq(
            $.var,
            optional(choice(
                seq(':', field('space_type', $.name)),
                seq('(', field('uri', $.uri_literal), ')')
            ))
        ),

        // let declarations
        _let_decls: $ => choice(
            $.linear_decls,
            $.conc_decls),

        linear_decls: $ => semiSep1($._decl),
        conc_decls: $ => prec(-1, conc1($._decl)),
        _decl: $ => choice($.decl, $.let_var_decl),
        decl: $ => seq(field('names', $.names), '<-', field('procs', alias(commaSep1($._proc), $.procs))),
        let_var_decl: $ => seq($.var, '=', $._proc),

        // bundles
        _bundle: $ => choice(
            $.bundle_read,
            $.bundle_write,
            $.bundle_equiv,
            $.bundle_read_write),
        bundle_read: $ => 'bundle-',
        bundle_write: $ => 'bundle+',
        bundle_equiv: $ => 'bundle0',
        bundle_read_write: $ => 'bundle',

        // case in match expression
        // Optional `where` guard: `pattern where guard => proc`. If the guard
        // evaluates to true, the case fires; if false, fall through to the
        // next case. Non-bool guard also falls through.
        case: $ => seq(
            field('pattern', $._proc),
            optional(seq('where', field('guard', $._proc))),
            '=>',
            field('proc', $._proc)
        ),

        // branch in select expression
        // Optional `where` guard: only commit (consume messages) if every
        // spatial pattern matches AND the guard evaluates to true.
        // Guard-false is indistinguishable from a spatial mismatch.
        branch: $ => seq(
            field('pattern', semiSep1($.linear_bind)),
            optional(seq('where', field('guard', $._proc))),
            '=>',
            field('proc', choice($.send, $._proc_expression))
        ),

        // for comprehensions
        // Optional `where` guard on a receipt: only commit (consume messages)
        // if every spatial pattern matches AND the guard evaluates to true.
        // Guard-false is indistinguishable from a spatial mismatch.
        receipts: $ => semiSep1($.receipt),
        receipt: $ => seq(
            conc1(choice($.linear_bind, $.repeated_bind, $.peek_bind, $.signed_bind)),
            optional(seq('where', field('guard', $._proc)))
        ),

        // Greg app:concrete SignedBind: a per-clause signed bind `{% y <- x %}[s]`
        // (Axis-C join — each rendezvous surface is funded independently). Plain
        // binds (linear_bind, etc.) remain unsigned; signed and plain binds compose
        // with `&` inside one receipt.
        signed_bind: $ => prec(2, seq(
            '{%', optional(field('names', $.names)), '<-', field('input', $._source), '%}',
            '[', field('sig', $.signature), ']'
        )),

        linear_bind: $ => seq(
            optional(field('names', $.names)),
            '<-',
            field('input', $._source),
            optional(field('query_spec', $.query_spec))
        ),

        // Reified RSpaces receive query `ch ~ q` (linear binds only). The
        // query is a payload list plus optional named parameters, and it
        // follows the parens-`;` rule of `hyper_inputs`:
        //   ch ~ e               payload [e]
        //   ch ~ (a, b)          payload [(a, b)]   (one tuple)
        //   ch ~ (a, b;)         payload [a, b]
        //   ch ~ (a; k = v)      payload [a], named k = v
        //   ch ~ (; k = v)       empty payload, named k = v
        // `ch ~ (e;)` and `ch ~ e` build the same AST. The AST builder
        // rejects (`MalformedQuerySpec`) a positional parameter in a query,
        // the empty group `ch ~ (;)`, and a query on a `!?`, `?!` or `!m`
        // source, whose reply channel is fresh.
        query_spec: $ => seq('~', field('query', choice($.hyper_inputs, $._proc))),

        repeated_bind: $ => seq(
            optional(field('names', $.names)),
            '<=',
            field('input', $.name)
        ),

        peek_bind: $ => seq(
            optional(field('names', $.names)),
            '<<-',
            field('input', $.name)
        ),

        // source definitions
        _source: $ => choice(
            $.simple_source,
            $.receive_send_source,
            $.send_receive_source,
            $.send_method_source),

        simple_source: $ => $.name,
        receive_send_source: $ => seq($.name, '?!'),
        send_receive_source: $ => seq($.name, '!?', field('inputs', choice(alias($._proc_list, $.inputs), $.hyper_inputs))),

        // For-source position of the method-call sugar (FIP 2025-08-20
        // Agents §"Method-call sugar"):
        //
        //   for (x <- inst!y(args)) { P }
        //   =
        //   for (x <- inst!?("y", args)) { P }
        //
        // No AST variant; the visitor rewrites this into a
        // Source::SendReceive at parse time with "y" prepended.
        send_method_source: $ => seq(
            $.name, '!',
            field('method', $.var),
            field('inputs', choice(alias($._proc_list, $.inputs), $.hyper_inputs))
        ),

        // sends
        _send_type: $ => choice($.send_single, $.send_multiple),
        send_single: $ => '!',
        send_multiple: $ => '!!',

        // ground terms and expressions

        block: $ => seq('{', $._proc, '}'),

        simple_type: $ => choice('Bool', 'Int', 'String', 'Uri', 'ByteArray'),

        _literal: $ => choice(
            $.bool_literal,
            $.signed_int_literal,
            $.unsigned_int_literal,
            $.bigint_literal,
            $.bigrat_literal,
            $.float_literal,
            alias($._bare_exponent_float, $.float_literal),
            $.fixed_point_literal,
            $.long_literal,
            $.string_literal,
            $.uri_literal),
        bool_literal: $ => choice('true', 'false'),
        signed_int_literal: $ => token(/-?\d+i[1-9]\d*/),
        unsigned_int_literal: $ => token(/\d+u[1-9]\d*/),
        bigint_literal: $ => token(/-?\d+n/),
        bigrat_literal: $ => token(/-?\d+r/),
        // Floats. A suffix (`f32`, `f64`, `f128`, `f256`) gives the width,
        // and a bare float (no suffix) is an f64:
        //   1.5f32   2.5   .5   1.5e3   -2.5e-3      (this token)
        //   1e3   2E-4   -7e+2                        (`_bare_exponent_float`)
        // A bare float needs a digit after its dot, so `5.method()` stays a
        // method call on an integer. `5.` and `5.e3` are not bare floats.
        //
        // Bare decimal floats keep every 02cef80 tree: at 02cef80 the text
        // `1.5` or `.5` lexes as `.` followed by digits, and `.` can then only
        // be a method call (which needs an identifier) or the empty
        // continuation of a synchronous send. In `x!?(a).5` only `.` and `;`
        // are valid after `x!?(a)`, so the lexer still produces `.` and then
        // the integer `5`, exactly as at 02cef80.
        //
        // The bare exponent form comes from the external scanner, which emits
        // it only when no identifier character follows the exponent digits.
        // At 02cef80 `1e5x` lexes as the integer `1` and the identifier `e5x`
        // (two adjacent match cases in `match v { 1 => 1e5x => Nil }`), and
        // the node's `preprocess_floats` leaves that text unchanged; a regex
        // token would read `1e5` and `x` instead. Directly after a complete
        // process, `1e5` alone is a float now, where 02cef80 read `1` and
        // `e5`; the node's `preprocess_floats` already reads it as `1e5f64`,
        // so programs that the node accepts keep their meaning.
        float_literal: $ => token(choice(
            /-?(?:\d+\.\d*|\.\d+|\d+)(?:[eE][+-]?\d+)?f(?:32|64|128|256)/,
            /-?(?:\d+\.\d+|\.\d+)(?:[eE][+-]?\d+)?/)),
        fixed_point_literal: $ => token(/-?(?:\d+\.\d*|\.\d+|\d+)p\d+/),
        long_literal: $ => token(/-?\d+/),
        string_literal: $ => token(/"([^"\\]|(\\[0nrt\\"])|(\\[0-9]+))*"/),
        uri_literal: $ => token(/`[^`]+`/),

        unit: $ => seq('(', ')'),

        nil: $ => 'Nil',

        // Collections
        collection: $ => choice($.list, $.tuple, $.set, $.map, $.pathmap),

        list: $ => seq('[', commaSep($._proc), optional($._proc_remainder), ']'),

        set: $ => seq('Set', '(', commaSep($._proc), optional($._proc_remainder), ')'),

        map: $ => seq('{', commaSep($.key_value_pair), optional($._proc_remainder), '}'),
        key_value_pair: $ => seq(field('key', $._proc), ':', field('value', $._proc)),

        pathmap: $ => seq('{|', commaSep($._proc), optional($._proc_remainder), '|}'),

        tuple: $ => choice(
            seq('(', $._proc, ',)'),
            seq('(', commaSep1($._proc), ')')
        ),

        _proc_remainder: $ => seq('...', field('remainder', $._proc_var)),

        // process lists
        _proc_list: $ => seq('(', commaSep($._proc), ')'),

        // process variables and names
        name: $ => choice($._proc_var, $.quote),
        _proc_var: $ => choice($.wildcard, $.var),
        quote: $ => prec(12, seq('@', $._proc)),
        wildcard: $ => '_',
        var: $ => token(/[a-zA-Z]([a-zA-Z0-9_'])*|_([a-zA-Z0-9_'])+/),

        names: $ => choice(seq(commaSep1($.name), optional($._name_remainder)), $._name_remainder),
        _name_remainder: $ => seq('...', '@', field('cont', $._proc_var)),

        // comments

        _line_comment: $ => token(seq('//', /[^\n]*/)),
        _block_comment: $ => token(seq(
            '/*',
            /[^*]*\*+([^/*][^*]*\*+)*/,
            '/',
        )),
        // http://stackoverflow.com/questions/13014947/regex-to-match-a-c-style-multiline-comment/36328890#36328890
    }
});

function commaSep(rule) {
    return optional(commaSep1(rule))
}

function commaSep1(rule) {
    return seq(rule, repeat(seq(',', rule)), optional(','))
}

function semiSep(rule) {
    return optional(semiSep1(rule))
}

function semiSep1(rule) {
    return seq(rule, repeat(seq(';', rule)))
}

function conc(rule) {
    return optional(conc1(rule))
}

function conc1(rule) {
    return seq(rule, repeat(seq('&', rule)))
}
