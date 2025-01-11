use lalrpop_util::lalrpop_mod;

mod ast;

lalrpop_mod!(pub fa);

fn main() {
	let result = fa::ExpressionParser::new().parse("12(celsius, 12)(521)");
	println!("{:?}", result);
}

#[cfg(test)]
mod tests {
	mod operators;
}
