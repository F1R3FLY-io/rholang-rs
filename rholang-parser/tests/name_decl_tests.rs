use rholang_parser::RholangParser;
use rholang_parser::parser::errors::ParsingError;
use validated::Validated;

fn duplicate_decl(source: &str) -> Option<(usize, usize)> {
    let parser = RholangParser::new();
    match parser.parse(source) {
        Validated::Good(_) => None,
        Validated::Fail(failures) => failures
            .iter()
            .flat_map(|failure| failure.errors.iter())
            .find_map(|error| match error.error {
                ParsingError::DuplicateNameDecl { first, second } => Some((first.col, second.col)),
                _ => None,
            }),
    }
}

#[test]
fn distinct_names_are_accepted() {
    assert_eq!(duplicate_decl("new a, b, c in { Nil }"), None);
}

#[test]
fn the_earliest_repeated_name_is_reported_with_its_next_declaration() {
    // `a` is declared at bytes 4 and 13, `b` at 7 and 10: `a` comes first.
    let (first, second) = duplicate_decl("new a, b, b, a in { Nil }").expect("a duplicate");
    assert_eq!(second - first, 9);

    let (first, second) = duplicate_decl("new a, b, a, a in { Nil }").expect("a duplicate");
    assert_eq!(second - first, 6);
}

#[test]
fn many_distinct_names_are_accepted() {
    let names = (0..20_000)
        .map(|i| format!("x{i}"))
        .collect::<Vec<_>>()
        .join(", ");
    assert_eq!(duplicate_decl(&format!("new {names} in {{ Nil }}")), None);
}
