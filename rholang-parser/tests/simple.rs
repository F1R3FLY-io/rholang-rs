use rholang_parser::RholangParser;

#[test]
fn test_simple() {
    let parser = RholangParser::new();

    // let input = r#"
    //     new hello, stdout(`rho:io:stdout`) in {
    //         for (@msg <- hello) {
    //             stdout!(msg)
    //         }|
    //         hello!("awesome")
    //     }
    // "#;

    // let input = r#"
    //     new randoChannel in {
    //         randoChannel!("Message")
    //     }
    // "#;

    // let input = r#"
    //     new sending in {
    //         sending!({"a": 3, "b": 4, "c": 5})
    //     }
    // "#;

    // let input = r#"
    //     new sending in {
    //         sending!({"a": 3, "b": 4, "c": 5}.delete("c"))
    //     }
    // "#;

    // let input = r#"
    //     new sending in {
    //         sending!(Sett(3, 4, 5))
    //     }
    // "#;

    // let input = r#"
    //     new sending in {
    //         sending!([33, 44, 55, 66])
    //     }
    // "#;

    let input = r#"
        new sending in {
            sending!({| 1, 2, "hi" |})
        }
    "#;

    let result = parser.parse(input);

    dbg!(result);

    // let mut parser = tree_sitter::Parser::new();
    // let rholang_language = rholang_tree_sitter::LANGUAGE.into();
    // parser
    //     .set_language(&rholang_language)
    //     .expect("Error loading Rholang parser");

    // let tree = parsing::parse_to_tree(input);

    // dbg!(result);
}
