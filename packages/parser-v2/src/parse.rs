use crate::nodes::*;
use crate::tokenize::tokenize;
use crate::tokens::{Token, TokenKind};

struct Position<'input> {
    token: &'input Token,
    index: usize,
    offset: usize,
}

struct Context<'input> {
    input: &'input str,
    tokens: &'input [Token],
    nodes: &'input mut SemanticTree<'input>,
}

fn start_position<'input>(tokens: &'input [Token]) -> Option<Position<'input>> {
    let mut index = 0;
    let mut offset = 0;

    while index < tokens.len() {
        let token = unsafe { tokens.get_unchecked(index) };
        if token.kind != TokenKind::Space {
            return Some(Position {
                token,
                index,
                offset,
            });
        }
        index += 1;
        offset += token.length as usize;
    }

    None
}

/// Return the current token and the position of the next token.
fn next_position<'input>(
    tokens: &'input [Token],
    Position {
        token,
        index,
        offset,
    }: Position<'input>,
) -> Option<Position<'input>> {
    let mut next_index = index + 1;
    let mut next_offset = offset + (token.length as usize);

    // first, skip all spaces
    while next_index < tokens.len() {
        let token = unsafe { tokens.get_unchecked(next_index) };
        if token.kind != TokenKind::Space {
            return Some(Position {
                token,
                index: next_index,
                offset: next_offset,
            });
        }
        next_index += 1;
        next_offset += token.length as usize;
    }

    None
}

pub fn parse_expression<'input>(input: &'input str) -> SemanticTree<'input> {
    let tokens = tokenize(input.as_bytes());
    let mut nodes: SemanticTree<'input> = SemanticTree::with_capacity(tokens.len());

    let mut left = expression_left(input, &mut position, &tokens, &mut nodes, Priority::None);

    while position.index < tokens.len() {
        left = expression_right(
            input,
            &mut position,
            &tokens,
            &mut nodes,
            left,
            Priority::None,
        );
    }

    nodes
}

#[test]
fn test_parse_expression() {
    let input = "a * b + c";
    let semantic_tree = parse_expression(input);
    println!("{}", semantic_tree_to_string(&semantic_tree));
}

#[inline]
fn expression_left<'input>(
    input: &'input str,
    tokens: &[Token],
    nodes: &mut SemanticTree<'input>,
    position: Position,
    priority: Priority,
) -> usize {
    let Position {
        token,
        index,
        offset,
    } = position;

    let node: Node<'input> = match token.kind {
        TokenKind::Identifier => Node::Identifier(&input[offset..offset + (token.length as usize)]),
        TokenKind::Integer => Node::Integer(
            input[offset..offset + (token.length as usize)]
                .parse::<i32>()
                .unwrap(),
        ),
        TokenKind::True => Node::Boolean(true),
        TokenKind::False => Node::Boolean(false),
        TokenKind::Minus => Node::Negate {
            right: expression_left(
                input,
                tokens,
                nodes,
                next_position(tokens, position).unwrap(),
                priority,
            ),
        },
        _ => panic!("Expected left token, received: {:?}", token),
    };

    let left = nodes.insert(node);
    expression_right(
        input,
        tokens,
        nodes,
        next_position(tokens, position).unwrap(),
        left,
        priority,
    )
}

fn expression_right<'input>(
    input: &'input str,
    tokens: &[Token],
    nodes: &mut SemanticTree<'input>,
    position: Option<Position>,
    left: usize,
    priority: Priority,
) -> usize {
    let node: Node<'input> = match position {
        None => {
            return left;
        }
        Some(Position {
            token,
            index,
            offset,
        }) => match token.kind {
            TokenKind::Stop => {
                return left;
            }
            TokenKind::Plus => {
                if priority > Priority::Additive {
                    return left;
                } else {
                    Node::Add {
                        left,
                        right: expression_left(
                            input,
                            tokens,
                            nodes,
                            next_position(tokens, position).unwrap(),
                            Priority::Additive,
                        ),
                    }
                }
            }
            TokenKind::Minus => {
                if priority > Priority::Additive {
                    return left;
                } else {
                    Node::Subtract {
                        left,
                        right: expression_left(input, position, tokens, nodes, Priority::Additive),
                    }
                }
            }
            TokenKind::Star => {
                if priority > Priority::Multiplicative {
                    return left;
                } else {
                    Node::Multiply {
                        left,
                        right: expression_left(
                            input,
                            position,
                            tokens,
                            nodes,
                            Priority::Multiplicative,
                        ),
                    }
                }
            }
            _ => panic!(
                "Unexpected token <{:?}> '{}'",
                token.kind,
                &input[offset..offset + (token.length as usize)]
            ),
        },
    };

    let new_left = nodes.insert(node);
    println!("new_left: {}", node_to_string(nodes, new_left));
    expression_right(input, position, tokens, nodes, new_left, Priority::None)
}
