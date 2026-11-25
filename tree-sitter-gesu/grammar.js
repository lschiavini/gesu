/// Tree-sitter grammar for Gesù.
///
/// Statements end at a newline, and every block closes with `amen`.
/// Newlines are therefore tokens here, not whitespace.

const PREC = { or: 1, and: 2, deny: 3, compare: 4, add: 5, multiply: 6, unary: 7, call: 8 };

// A comma-separated list that may be spread over several lines.
const list = ($, rule) =>
  seq(repeat($._nl), rule, repeat(seq(",", repeat($._nl), rule)), optional(","), repeat($._nl));

module.exports = grammar({
  name: "gesu",

  extras: ($) => [/[ \t\r]/, $.comment],

  word: ($) => $.identifier,

  // Inside `discern`, a line such as `Found(x)` either continues the
  // block of the case above or opens the next case. Only the `:` after it
  // tells which, so the parser follows both readings until one fails.
  conflicts: ($) => [[$.block]],

  rules: {
    source_file: ($) => seq(repeat(choice($._statement, $._nl)), optional(seq($.amen, repeat($._nl)))),

    _nl: (_) => /\n/,
    comment: (_) => token(seq("--", /[^\n]*/)),
    amen: (_) => "amen",

    block: ($) => repeat1(choice($._statement, $._nl)),

    _statement: ($) =>
      choice(
        $.let_statement,
        $.law_statement,
        $.assignment,
        $.preach_statement,
        $.give_statement,
        $.sin_statement,
        $.if_statement,
        $.while_statement,
        $.repeat_statement,
        $.for_statement,
        $.break_statement,
        $.continue_statement,
        $.repent_statement,
        $.tempt_statement,
        $.discern_statement,
        $.parable,
        $.kind,
        $.pagan_statement,
        $.expression_statement,
      ),

    let_statement: ($) =>
      seq(
        "let",
        optional("mortal"),
        field("name", $.identifier),
        optional(seq(":", field("type", $._type))),
        "=",
        field("value", $._expression),
        $._nl,
      ),

    law_statement: ($) => seq("law", field("name", $.identifier), "=", field("value", $._expression), $._nl),

    assignment: ($) => seq(field("target", $._expression), "=", field("value", $._expression), $._nl),

    preach_statement: ($) => seq("preach", $._expression, $._nl),
    give_statement: ($) => seq("give", optional($._expression), $._nl),
    sin_statement: ($) => seq("sin", $._expression, $._nl),
    break_statement: ($) => seq("break", $._nl),
    continue_statement: ($) => seq("continue", $._nl),
    repent_statement: ($) => seq("repent", $._nl),
    expression_statement: ($) => seq($._expression, $._nl),

    if_statement: ($) =>
      seq(
        "if",
        field("condition", $._expression),
        $._nl,
        optional($.block),
        choice(
          seq($.amen, $._nl),
          seq("otherwise", $._nl, optional($.block), $.amen, $._nl),
          // `otherwise if` shares the one closing `amen`.
          seq("otherwise", $.if_statement),
        ),
      ),

    while_statement: ($) => seq("while", field("condition", $._expression), $._nl, optional($.block), $.amen, $._nl),

    repeat_statement: ($) =>
      seq("repeat", field("count", $._expression), "times", $._nl, optional($.block), $.amen, $._nl),

    for_statement: ($) =>
      seq(
        "for",
        "each",
        field("first", $.identifier),
        optional(seq(",", field("second", $.identifier))),
        "in",
        field("in", $._expression),
        $._nl,
        optional($.block),
        $.amen,
        $._nl,
      ),

    tempt_statement: ($) =>
      seq(
        "tempt",
        $._nl,
        optional($.block),
        "forgive",
        field("trespass", $.identifier),
        $._nl,
        optional($.block),
        $.amen,
        $._nl,
      ),

    discern_statement: ($) =>
      seq("discern", field("subject", $._expression), $._nl, repeat(choice($.arm, $._nl)), $.amen, $._nl),

    // Blank lines after a case belong to its block, hence the right bias.
    arm: ($) =>
      prec.right(seq(field("pattern", $.pattern), ":", choice($._statement, seq($._nl, optional($.block))))),

    pattern: ($) =>
      seq(
        field("case", choice($.identifier, $.truth)),
        optional(seq("(", $.identifier, repeat(seq(",", $.identifier)), ")")),
      ),

    parable: ($) =>
      seq(
        "parable",
        field("name", $.identifier),
        "(",
        optional(list($, $.parameter)),
        ")",
        optional(seq("->", field("gives", $._type))),
        optional($.confession),
        $._nl,
        optional($.block),
        $.amen,
        $._nl,
      ),

    confession: (_) => seq("may", "sin"),

    parameter: ($) =>
      seq(field("name", $.identifier), ":", optional(choice("lend", "entrust")), field("type", $._type)),

    kind: ($) =>
      seq("kind", field("name", $.identifier), $._nl, repeat(choice($.field, $.case, $._nl)), $.amen, $._nl),

    field: ($) => seq(field("name", $.identifier), ":", field("type", $._type), $._nl),

    case: ($) =>
      seq(field("name", $.identifier), optional(seq("(", $._type, repeat(seq(",", $._type)), ")")), $._nl),

    pagan_statement: ($) =>
      seq("pagan", field("tongue", $.identifier), field("source", $.string), "as", field("name", $.identifier), $._nl),

    // ---- types ----

    _type: ($) => choice($.type_name, $.list_type, $.map_type, $.maybe_type),
    type_name: ($) => $.identifier,
    list_type: ($) => seq("[", $._type, "]"),
    map_type: ($) => seq("{", $._type, ":", $._type, "}"),
    maybe_type: ($) => seq("Maybe", $._type),

    // ---- expressions ----

    _expression: ($) =>
      choice(
        $.identifier,
        $.number,
        $.decimal,
        $.string,
        $.truth,
        $.list,
        $.map,
        $.parenthesized,
        $.negation,
        $.denial,
        $.baptism,
        $.binary,
        $.call,
        $.index,
        $.member,
      ),

    truth: (_) => choice("Jesus", "False"),
    number: (_) => /\d[\d_]*/,
    decimal: (_) => /\d[\d_]*\.\d+/,

    string: ($) => seq('"', repeat(choice($.string_content, $.escape, $.interpolation)), '"'),
    string_content: (_) => token.immediate(prec(1, /[^"\\{\n]+/)),
    escape: (_) => token.immediate(/\\./),
    interpolation: ($) => seq("{", $._expression, "}"),

    list: ($) => seq("[", optional(list($, $._expression)), "]"),
    map: ($) => seq("{", optional(list($, $.pair)), "}"),
    pair: ($) => seq(field("key", $._expression), ":", field("value", $._expression)),
    parenthesized: ($) => seq("(", $._expression, ")"),

    negation: ($) => prec(PREC.unary, seq("-", $._expression)),
    denial: ($) => prec(PREC.deny, seq("deny", $._expression)),
    baptism: ($) => prec(PREC.unary, seq("baptize", $._expression, "as", field("type", $._type))),

    binary: ($) =>
      choice(
        prec.left(PREC.or, seq($._expression, "or", $._expression)),
        prec.left(PREC.and, seq($._expression, "and", $._expression)),
        prec.left(PREC.compare, seq($._expression, choice("<", ">", "<=", ">="), $._expression)),
        prec.left(PREC.compare, seq($._expression, "is", optional("not"), $._expression)),
        prec.left(PREC.add, seq($._expression, choice("+", "-"), $._expression)),
        prec.left(PREC.multiply, seq($._expression, choice("*", "/", "%"), $._expression)),
      ),

    call: ($) =>
      prec(PREC.call, seq(field("parable", $._expression), "(", optional(list($, $.argument)), ")")),

    argument: ($) =>
      seq(
        optional(choice("lend", "entrust")),
        optional(seq(field("name", $.identifier), ":")),
        field("value", $._expression),
      ),

    index: ($) => prec(PREC.call, seq($._expression, "[", $._expression, "]")),
    member: ($) => prec(PREC.call, seq($._expression, ".", field("field", $.identifier))),

    identifier: (_) => /[\p{L}_][\p{L}\p{N}_]*/,
  },
});
