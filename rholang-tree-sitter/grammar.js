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

    inline: $ => [$.name, $.quotable],

    rules: {
        // Starting point of the grammar
        source_file: $ => repeat($._proc),

        _proc: $ => choice(
            $.par,
            $.send_sync,
            $.new,
            $.ifElse,
            $.let,
            $.bundle,
            $.match,
            $.choice,
            $.contract,
            $.input,
            $.send,
            $._proc_expression
        ),

        par: $ => prec.left(0, seq($._proc, '|', $._proc)),

        send_sync: $ => prec(1, seq(
            field('channel', $.name),
            '!?',
            field('inputs', alias($._proc_list, $.messages)), field('cont', $.sync_send_cont))
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

        input: $ => prec(2, seq(
            'for', '(', field('receipts', $.receipts), ')',
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
            $.quote,
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
        quote: $ => prec(12, seq('@', $.quotable)),
        quotable: $ => choice(
            $.var_ref,
            $.eval,
            $.disjunction,
            $.conjunction,
            $.negation,
            $._ground_expression),
        var_ref: $ => prec(13, seq(field('kind', $.var_ref_kind), field('var', $.var))),
        var_ref_kind: $ => choice('=', '=*'),
        disjunction: $ => prec.left(13, seq($._proc, '\\/', $._proc)),
        conjunction: $ => prec.left(14, seq($._proc, '/\\', $._proc)),
        negation: $ => prec(15, seq('~', $._proc)),
        _ground_expression: $ => prec(16, choice($.block, $._literal, $.nil, $.collection, $._proc_var, $.simple_type, $.unit)),

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

        linear_decls: $ => semiSep1($.decl),
        conc_decls: $ => prec(-1, conc1($.decl)),
        decl: $ => seq(field('names', $.names), '=', field('procs', alias(commaSep1($._proc), $.procs))),

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
        case: $ => seq(field('pattern', $._proc), '=>', field('proc', $._proc)),

        // branch in select expression
        branch: $ => seq(field('pattern', conc1($.linear_bind)), '=>', field('proc', choice($.send, $._proc_expression))),

        // for comprehensions
        receipts: $ => semiSep1($.receipt),
        receipt: $ => conc1(choice($.linear_bind, $.repeated_bind, $.peek_bind)),

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
            $.send_receive_source),

        simple_source: $ => $.name,
        receive_send_source: $ => seq($.name, '?!'),
        send_receive_source: $ => seq($.name, '!?', field('inputs', alias($._proc_list, $.inputs))),

        // sends
        _send_type: $ => choice($.send_single, $.send_multiple),
        send_single: $ => '!',
        send_multiple: $ => '!!',

        // ground terms and expressions

        block: $ => seq('{', $._proc, '}'),

        simple_type: $ => choice('Bool', 'Int', 'String', 'Uri', 'ByteArray'),

        _literal: $ => choice(
            $.bool_literal,
            $.long_literal,
            $.string_literal,
            $.uri_literal),
        bool_literal: $ => choice('true', 'false'),
        long_literal: $ => token(/-?\d+/),
        string_literal: $ => token(/"([^"\\]|(\\[0nrt\\"])|(\\[0-9]+))*"/),
        uri_literal: $ => token(/`[^`]+`/),

        unit: $ => seq('(', ')'),

        nil: $ => 'Nil',

        // Collections
        collection: $ => choice($.list, $.tuple, $.set, $.map),

        list: $ => seq('[', commaSep($._proc), optional($._proc_remainder), ']'),

        set: $ => seq('Set', '(', commaSep($._proc), optional($._proc_remainder), ')'),

        map: $ => seq('{', commaSep($.key_value_pair), optional($._proc_remainder), '}'),
        key_value_pair: $ => seq(field('key', $._proc), ':', field('value', $._proc)),

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
