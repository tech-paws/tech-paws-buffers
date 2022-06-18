use crate::lexer::{Lexer, Literal, Token};

#[derive(Debug)]
pub enum ASTNode {
    Enum(Vec<EnumItemASTNode>),
    ValueEnum(Vec<IdValuePair>),
    Struct(StructASTNode),
}

#[derive(Debug)]
pub struct StructASTNode {
    pub id: String,
    pub fields: Vec<StructFieldASTNode>,
}

#[derive(Debug)]
pub enum EnumItemASTNode {
    Empty {
        position: u32,
        id: String,
    },
    Tuple {
        position: u32,
        id: String,
        values: Vec<TupleFieldASTNode>,
    },
    Struct {
        position: u32,
        id: String,
        fields: Vec<StructFieldASTNode>,
    },
}

#[derive(Debug)]
pub enum ConstValueASTNode {
    Literal { literal: Literal },
}

#[derive(Debug)]
pub struct IdValuePair {
    pub id: String,
    pub value: ConstValueASTNode,
}

#[derive(Debug)]
pub enum TypeIDASTNode {
    Primitive { id: String },
    Generic { id: String, generics: Vec<TypeIDASTNode> },
}

#[derive(Debug)]
pub struct TupleFieldASTNode {
    pub position: u32,
    pub type_id: TypeIDASTNode,
}

#[derive(Debug)]
pub struct StructFieldASTNode {
    pub position: u32,
    pub name: String,
    pub type_id: TypeIDASTNode,
}

pub fn parse(lexer: &mut Lexer) -> Vec<ASTNode> {
    let mut ast_nodes = vec![];

    while *lexer.current_token() != Token::EOF {
        if *lexer.current_token() == Token::Struct {
            ast_nodes.push(parse_struct(lexer));
        } else {
            panic!("Unexpected token: {:?}", lexer.current_token());
        }
    }

    ast_nodes
}

pub fn parse_struct(lexer: &mut Lexer) -> ASTNode {
    if *lexer.current_token() != Token::Struct {
        panic!("Expected 'struct' but got {:?}", lexer.current_token());
    }

    let name = if let Token::ID { name } = lexer.next_token() {
        name.clone()
    } else {
        panic!("Expected string value, but got {:?}", lexer.current_token());
    };

    if *lexer.next_token() == Token::Symbol(';') {
        lexer.next_token();

        return ASTNode::Struct(StructASTNode {
            id: name,
            fields: Vec::new(),
        });
    }

    if *lexer.current_token() != Token::Symbol('{') {
        panic!("Expected ';' or '{{', but got {:?}", lexer.current_token());
    }

    lexer.next_token();
    let parameters = parse_struct_parameters(lexer);

    if *lexer.current_token() != Token::Symbol('}') {
        panic!("Expected '}}', but got {:?}", lexer.current_token());
    }

    lexer.next_token();

    ASTNode::Struct(StructASTNode {
        id: name.clone(),
        fields: parameters,
    })
}

pub fn parse_struct_parameters(lexer: &mut Lexer) -> Vec<StructFieldASTNode> {
    let mut fields = vec![];

    while *lexer.current_token() == Token::Symbol('#') {
        let position = parse_position(lexer);
        let name = if let Token::ID { name } = lexer.current_token() {
            name.clone()
        } else {
            panic!("Expected string value, but got {:?}", lexer.current_token());
        };

        if *lexer.next_token() != Token::Symbol(':') {
            panic!("Expected ':', but got {:?}", lexer.current_token());
        }

        lexer.next_token();
        let type_id = parse_type_id(lexer);
        fields.push(StructFieldASTNode {
            position,
            name,
            type_id,
        });

        if *lexer.current_token() != Token::Symbol(',') {
            break;
        }

        lexer.next_token();
    }

    return fields;
}

pub fn parse_type_id(lexer: &mut Lexer) -> TypeIDASTNode {
    let name = if let Token::ID { name } = lexer.current_token() {
        name.clone()
    } else {
        panic!("Expected string value, but got {:?}", lexer.current_token());
    };
    lexer.next_token();
    TypeIDASTNode::Primitive { id: name }
}

/// Parse #[<number>]
pub fn parse_position(lexer: &mut Lexer) -> u32 {
    if *lexer.current_token() != Token::Symbol('#') {
        panic!("Expected '#' but got {:?}", lexer.current_token());
    }

    if *lexer.next_token() != Token::Symbol('[') {
        panic!("Expected '[' but got {:?}", lexer.current_token());
    }

    let position = if let Token::Literal(Literal::IntLiteral(value)) = lexer.next_token() {
        *value
    } else {
        panic!("Expected int but got {:?}", lexer.current_token());
    };

    if *lexer.next_token() != Token::Symbol(']') {
        panic!("Expected ']' but got {:?}", lexer.current_token());
    }

    lexer.next_token();
    position as u32
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn stringify_ast(ast: Vec<ASTNode>) -> String {
        let mut res = String::new();

        for node in ast {
            match node {
                ASTNode::Enum(_) => todo!(),
                ASTNode::ValueEnum(_) => todo!(),
                ASTNode::Struct(StructASTNode { id, fields }) => {
                    let mut fields_res = String::new();

                    for field in fields {
                        fields_res += &format!("    {:?}\n", field);
                    }

                    res += &format!(
                        "Struct {{\n  id: \"{}\",\n  fields: [\n{}  ]\n}}\n",
                        id, fields_res
                    );
                }
            }
        }

        println!("{}", res);
        res
    }

    #[test]
    fn parse_position_test() {
        let mut lexer = Lexer::tokenize("#[123]");
        let position = parse_position(&mut lexer);
        assert_eq!(position, 123);
    }

    #[test]
    fn parse_empty_file_test() {
        let src = fs::read_to_string("test_resources/empty.tpb").unwrap();
        let target_ast = fs::read_to_string("test_resources/empty.ast").unwrap();
        let mut lexer = Lexer::tokenize(&src);
        let actual_ast = stringify_ast(parse(&mut lexer));

        assert_eq!(actual_ast, target_ast);
    }

    #[test]
    fn parse_empty_struct_test() {
        let src = fs::read_to_string("test_resources/empty_struct.tpb").unwrap();
        let target_ast = fs::read_to_string("test_resources/empty_struct.ast").unwrap();
        let mut lexer = Lexer::tokenize(&src);
        let actual_ast = stringify_ast(parse(&mut lexer));

        assert_eq!(actual_ast, target_ast);
    }

    #[test]
    fn parse_two_empty_structs_test() {
        let src = fs::read_to_string("test_resources/two_empty_structs.tpb").unwrap();
        let target_ast = fs::read_to_string("test_resources/two_empty_structs.ast").unwrap();
        let mut lexer = Lexer::tokenize(&src);
        let actual_ast = stringify_ast(parse(&mut lexer));

        assert_eq!(actual_ast, target_ast);
    }

    #[test]
    fn parse_struct_with_parameters_test() {
        let src = fs::read_to_string("test_resources/struct_with_parameters.tpb").unwrap();
        let target_ast = fs::read_to_string("test_resources/struct_with_parameters.ast").unwrap();
        let mut lexer = Lexer::tokenize(&src);
        let actual_ast = stringify_ast(parse(&mut lexer));

        assert_eq!(actual_ast, target_ast);
    }
}
