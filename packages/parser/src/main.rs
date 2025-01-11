use lalrpop_util::lalrpop_mod;

mod ast;

lalrpop_mod!(pub fa);

fn main() {
    let result = fa::ExpressionParser::new().parse("1
    2, 3 + 4, 4 * 6, 5 + (2 * 3, 4), 6");
    println!("{:?}", result);
}

#[test]
fn test_parser() {
}
