module.exports = grammar({
    name: 'rholang',

    extras: $ => [
        $._line_comment,
        $._block_comment,
        /\s/,
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

    conflicts: $ => [
        // `(x)` — parenthesized proc_var vs bare lambda_param (no type
        // annotation). `prec.dynamic` on lambda_param biases GLR toward
        // lambda when the enclosing `=>` completes the parse; otherwise
        // the parenthesized interpretation wins.
        [$.lambda_param, $._proc_var],
        // `(x: A)` — lambda_param vs pi_param. GLR picks based on the
        // outer rule's arrow: lambda uses `=>`, Pi uses `->`.
        [$.lambda_param, $.pi_param],
    ],

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
            $.agent_block,
            $.try_block,
            $.input,
            $.foreach,
            $.send,
            $._proc_expression
        ),

        par: $ => prec.left(0, seq($._proc, '|', $._proc)),

        send_sync: $ => prec(1, seq(
            field('channel', $.name),
            '!?',
            field('inputs', alias($._proc_list, $.messages)), field('cont', $.sync_send_cont))
        ),

        send_method: $ => prec(1, seq(
            field('channel', $.name),
            '!',
            field('method', $.var),
            field('inputs', alias($._proc_list, $.messages)),
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
            field('proc', $.block)
        )),

        // f1r3lang-with-monads Stage B: the `foreach` comprehension.
        // Minimal surface — single binder, no `where`, no `&`, no `;`,
        // no `by`. Source is a term of a monadic sort (not a channel);
        // yield-name is the result channel the aggregator publishes on.
        // Richer surface awaits the DDL on feature/f1r3lang-mettail-only.
        foreach: $ => prec(2, seq(
            'foreach', '(',
                field('binder', $.var),
                '<-',
                field('source', $._proc_expression),
            ')',
            'yield', '(', field('yield_name', $.var), ')',
            field('proc', $.block)
        )),

        send: $ => prec(3, seq(
            field('channel', $.name),
            field('send_type', $._send_type),
            field('inputs', alias($._proc_list, $.inputs))
        )),

        _proc_expression: $ => choice(
            $._ground_expression,
            $._parenthesized,
            $.add,
            $.and,
            $.app,
            $.concat,
            $.conjunction,
            $.diff,
            $.disjunction,
            $.div,
            $.eq,
            $.eval,
            $.gt,
            $.gte,
            $.interpolation,
            $.lambda,
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
            $.pi,
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

        // Calculus of Constructions (Phase 1c): lambda abstraction.
        //
        // Surface: `(x_0, …, x_{n-1}) => body` or `() => body`.
        // `prec.dynamic(100, ...)` to bias GLR toward the lambda parse
        // in the ambiguous `match m { (x) => body }` position (where
        // `(x)` could be a parenthesized pattern or lambda params). In
        // practice the user writes match cases with simpler patterns,
        // and the dynamic bias matches their likely intent. The
        // `conflicts` declaration lets tree-sitter emit the GLR states.
        lambda: $ => prec.dynamic(100, prec.right(1, seq(
            '(',
            optional(field('params', $.lambda_params)),
            ')',
            '=>',
            field('body', $._proc)
        ))),

        // Named rule so tree-sitter keeps the var children addressable
        // under the params node (alias would collapse single-element seqs).
        lambda_params: $ => commaSep1($.lambda_param),

        // Calculus of Constructions (Phase 2b): lambda param with optional
        // type annotation `x: A`. Bare `x` is syntactic sugar for `x: _`
        // (type inference placeholder — not yet implemented). The parser
        // threads annotations through; the Phase 2c typechecker reads them.
        // `prec.dynamic` biases the GLR parser toward lambda_param when
        // `(x)` or `(x: T)` is ambiguous with alternatives (contract
        // formals, map key-value, parenthesized var), but only when the
        // enclosing lambda rule's `=>` completes the parse — otherwise
        // the alternative wins via normal conflict resolution.
        lambda_param: $ => prec.dynamic(100, seq(
            field('name', $.var),
            optional(seq(':', field('type', $._proc)))
        )),

        // Calculus of Constructions (Phase 2a): dependent function type.
        //
        // Surface forms:
        //   `(x: A, y: B) -> C`  — dependent, codomain can reference the
        //                           binders via their surface names
        //   `A -> B`              — non-dependent sugar (no binder names)
        //
        // `->` is reserved for Pi types; lambda uses `=>`. The dependent
        // form starts with `(` and a typed-param list; the sugar form
        // starts with any expression and `->`.
        pi: $ => prec.right(1, choice(
            seq(
                '(',
                field('params', $.pi_params),
                ')',
                '->',
                field('codomain', $._proc)
            ),
            seq(
                field('dom', $._proc_expression),
                '->',
                field('codomain', $._proc)
            )
        )),

        pi_params: $ => commaSep1($.pi_param),
        pi_param: $ => seq(field('name', $.var), ':', field('type', $._proc)),

        // Calculus of Constructions (Phase 1c): function application with
        // paren-delimited args. Postfix `fun(args)`, whitespace-tolerant.
        //
        // `fun` is restricted to callable forms — a lambda, a
        // parenthesized expression, a var, an eval, or a chained
        // app/method — rather than the full `_proc_expression`. The
        // restriction keeps literals (string, int, …) out of the `fun`
        // slot, which in turn prevents `@"name"(formals)` from parsing
        // as `quote(app("name", formals))` and stealing the formals of
        // the surrounding `contract`.
        app: $ => prec(11, seq(
            field('fun', choice(
                $._parenthesized,
                $._proc_var,
                $.eval,
                $.method,
                $.app
            )),
            token.immediate('('),
            optional(field('args', $.app_args)),
            ')'
        )),

        // Named rule so tree-sitter keeps a field with a stable kind under
        // single-arg applications (alias would collapse the single-element
        // seq). Empty-arg application has no field (tree-sitter disallows
        // rules matching the empty string); parsing.rs handles the missing
        // field case as zero arity.
        app_args: $ => prec(20, commaSep1($._proc)),
        var_ref: $ => prec(13, seq(field('kind', $.var_ref_kind), field('var', $.var))),
        var_ref_kind: $ => choice('=', '=*'),
        disjunction: $ => prec.left(13, seq($._proc, '\\/', $._proc)),
        conjunction: $ => prec.left(14, seq($._proc, '/\\', $._proc)),
        negation: $ => prec(15, seq('~', $._proc)),
        _ground_expression: $ => prec(16, choice($.block, $._literal, $.nil, $.collection, $._proc_var, $.simple_type, $.fixed_point_pattern, $.unit)),

        // synchronous send continuations
        sync_send_cont: $ => choice($.empty_cont, $.non_empty_cont),

        non_empty_cont: $ => prec(1, seq(';', $._proc)),
        empty_cont: $ => '.',

        // new name declaration
        name_decls: $ => commaSep1($.name_decl),
        name_decl: $ => seq($.var, optional(seq('(', field('uri', $.uri_literal), ')'))),

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
            conc1(choice($.linear_bind, $.repeated_bind, $.peek_bind)),
            optional(seq('where', field('guard', $._proc)))
        ),

        linear_bind: $ => seq(
            optional(field('names', $.names)),
            '<-',
            field('input', $._source)
        ),

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
        send_receive_source: $ => seq($.name, '!?', field('inputs', alias($._proc_list, $.inputs))),

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
            field('inputs', alias($._proc_list, $.inputs))
        ),

        // sends
        _send_type: $ => choice($.send_single, $.send_multiple),
        send_single: $ => '!',
        send_multiple: $ => '!!',

        // ground terms and expressions

        block: $ => seq('{', $._proc, '}'),

        simple_type: $ => choice(
            'Bool',
            'Int',
            'Int64',
            'Int32',
            'UInt64',
            'UInt32',
            'UInt16',
            'UInt8',
            'Float64',
            'Float32',
            'BigInt',
            'BigRat',
            'String',
            'Uri',
            'ByteArray',
            'Type'),

        // FixedPoint type pattern: bare `FixedPoint` matches any scale;
        // `FixedPoint(<pat>)` matches by pattern against the scale. The
        // left-prec resolves the ambiguity with a following parenthesized
        // expression: `FixedPoint(x)` is always the parameterized form.
        fixed_point_pattern: $ => prec.left(16, choice(
            'FixedPoint',
            seq('FixedPoint', '(', field('scale', $._proc), ')')
        )),

        _literal: $ => choice(
            $.bool_literal,
            $.signed_int_literal,
            $.unsigned_int_literal,
            $.bigint_literal,
            $.bigrat_literal,
            $.float_literal,
            $.fixed_point_literal,
            $.long_literal,
            $.string_literal,
            $.uri_literal),
        bool_literal: $ => choice('true', 'false'),
        signed_int_literal: $ => token(/-?\d+i[1-9]\d*/),
        unsigned_int_literal: $ => token(/\d+u[1-9]\d*/),
        bigint_literal: $ => token(/-?\d+n/),
        bigrat_literal: $ => token(/-?\d+r/),
        float_literal: $ => token(/-?(?:\d+\.\d*|\.\d+|\d+)(?:[eE][+-]?\d+)?f(?:32|64|128|256)/),
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
