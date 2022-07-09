use crate::{
    lexer::Literal,
    parser::{
        ASTNode, ConstValueASTNode, EnumASTNode, EnumItemASTNode, FnASTNode, StructASTNode,
        StructFieldASTNode, TupleFieldASTNode, TypeIDASTNode,
    },
    writer::Writer,
};

const RPC_NEW_DATA_STATUS: &str = "0xFF";
const RPC_NO_DATA_STATUS: &str = "0x00";

pub fn generate(ast: &[ASTNode], models: bool, buffers: bool, rpc: bool) -> String {
    let mut writer = Writer::default();

    writer.writeln("// GENERATED, DO NOT EDIT");
    writer.writeln("");

    if buffers {
        writer.writeln("use tech_paws_buffers::{BytesReader, BytesWriter, IntoVMBuffers};");
    }

    if models {
        writer.write(&generate_models(ast));
    }

    if buffers {
        writer.write(&generate_buffers(ast));
    }

    if rpc {
        writer.write(&generate_rpc(ast));
    }

    writer.show().to_string()
}

pub fn generate_models(ast: &[ASTNode]) -> String {
    let mut writer = Writer::default();

    for node in ast {
        match node {
            ASTNode::Struct(node) => writer.writeln(&generate_struct_model(node, true)),
            ASTNode::Enum(node) => writer.writeln(&generate_enum_model(node)),
            ASTNode::Fn(_) => (),
            ASTNode::Mod(_) => (),
        }
    }

    let mut res = writer.show().to_string();

    if res.ends_with("\n\n") {
        res.pop();
    }

    res
}

pub fn generate_buffers(ast: &[ASTNode]) -> String {
    let mut writer = Writer::default();

    for node in ast {
        match node {
            ASTNode::Struct(node) => writer.writeln(&generate_struct_buffers(node)),
            ASTNode::Enum(node) => writer.writeln(&generate_enum_buffers(node)),
            ASTNode::Fn(_) => (),
            ASTNode::Mod(_) => (),
        }
    }

    let mut res = writer.show().to_string();

    if res.ends_with("\n\n") {
        res.pop();
    }

    res
}

pub fn generate_rpc(ast: &[ASTNode]) -> String {
    let mut writer = Writer::default();

    for node in ast {
        match node {
            ASTNode::Struct(_) => (),
            ASTNode::Enum(_) => (),
            ASTNode::Fn(node) => writer.writeln(&generate_rpc_method(node)),
            ASTNode::Mod(_) => (),
        }
    }

    let mut res = writer.show().to_string();

    if res.ends_with("\n\n") {
        res.pop();
    }

    res
}

pub fn generate_struct_model(node: &StructASTNode, generate_default: bool) -> String {
    let mut writer = Writer::default();

    if node.fields.is_empty() {
        writer.writeln("#[derive(Debug, Clone, PartialEq)]");
        writer.writeln(&format!("pub struct {};", node.id));

        if generate_default {
            writer.writeln("");
            writer.writeln(&format!("impl Default for {} {{", node.id));
            writer.writeln_tab(1, "fn default() -> Self {");
            writer.writeln_tab(2, "Self");
            writer.writeln_tab(1, "}");
            writer.writeln("}");
        }
    } else {
        writer.writeln("#[derive(Debug, Clone, PartialEq)]");
        writer.writeln(&format!("pub struct {} {{", node.id));
        writer.write(&generate_struct_parameters(1, &node.fields));
        writer.writeln("}");

        if generate_default {
            writer.writeln("");
            writer.writeln(&generate_struct_default(&node));
        }
    }

    writer.show().to_string()
}

fn generate_struct_default(node: &StructASTNode) -> String {
    let mut writer = Writer::default();

    writer.writeln(&format!("impl Default for {} {{", node.id));
    writer.writeln_tab(1, "fn default() -> Self {");
    writer.writeln_tab(2, "Self {");

    for field in node.fields.iter() {
        writer.writeln_tab(
            3,
            &format!(
                "{}: {},",
                field.name,
                generate_default_const(&field.type_id)
            ),
        );
    }

    writer.writeln_tab(2, "}");
    writer.writeln_tab(1, "}");
    writer.write("}");

    writer.show().to_string()
}

fn generate_rpc_method(node: &FnASTNode) -> String {
    let mut writer = Writer::default();

    let args_struct_id = format!("__{}_rpc_args__", node.id);

    let mut args_struct_fields = vec![];

    for (i, arg) in node.args.iter().enumerate() {
        args_struct_fields.push(StructFieldASTNode {
            position: i as u32,
            name: arg.id.clone(),
            type_id: arg.type_id.clone(),
        });
    }

    let args_struct = StructASTNode {
        id: args_struct_id.clone(),
        fields: args_struct_fields,
        emplace_buffers: false,
        into_buffers: true,
    };

    writer.writeln(&generate_struct_model(&args_struct, false));

    writer.writeln(&generate_struct_buffers(&args_struct));

    writer.writeln(&format!("pub fn {}_rpc_handler(", node.id));
    writer.writeln_tab(1, "state: &mut vm::CycleState,");
    writer.writeln_tab(1, "client_buffer_address: vm::BufferAddress,");
    writer.writeln_tab(1, "server_buffer_address: vm::BufferAddress,");
    writer.writeln(") -> bool {");

    writer.writeln_tab(
        1,
        "let args = vm::buffer_read(state, server_buffer_address, |bytes_reader| {",
    );
    writer.writeln_tab(2, "let status = bytes_reader.read_byte();");
    writer.writeln("");
    writer.writeln_tab(2, &format!("if status == {} {{", RPC_NEW_DATA_STATUS));
    writer.writeln_tab(
        3,
        &format!(
            "Some({})",
            &generate_read(&TypeIDASTNode::Other { id: args_struct_id })
        ),
    );
    writer.writeln_tab(2, "} else {");
    writer.writeln_tab(3, "None");
    writer.writeln_tab(2, "}");
    writer.writeln_tab(1, "});");
    writer.writeln("");

    writer.writeln_tab(1, "if let Some(args) = &args {");
    writer.writeln_tab(
        2,
        "vm::buffer_write(state, server_buffer_address, |bytes_writer| {",
    );
    writer.writeln_tab(3, "bytes_writer.clear();");
    writer.writeln_tab(
        3,
        &format!("bytes_writer.write_byte({});", RPC_NO_DATA_STATUS),
    );
    writer.writeln_tab(2, "});");

    if let Some(return_type_id) = &node.return_type_id {
        writer.writeln_tab(2, &format!("let ret = {}_rpc_handler_impl(", node.id));
        writer.writeln_tab(3, "state,");

        for arg in node.args.iter() {
            writer.writeln_tab(3, &format!("args.clone().{},", arg.id));
        }

        writer.writeln_tab(2, ");");

        writer.writeln_tab(
            2,
            "vm::buffer_write(state, client_buffer_address, |bytes_writer| {",
        );
        writer.writeln_tab(3, "bytes_writer.clear();");
        writer.writeln_tab(
            3,
            &format!("bytes_writer.write_byte({});", RPC_NEW_DATA_STATUS),
        );
        writer.writeln_tab(3, &generate_write(return_type_id, "ret", false));
        writer.writeln_tab(2, "});");
    } else {
        writer.writeln_tab(2, &format!("{}_rpc_handler_impl(", node.id));
        writer.writeln_tab(3, "state,");

        for arg in node.args.iter() {
            writer.writeln_tab(3, &format!("args.clone().{},", arg.id));
        }

        writer.writeln_tab(2, ");");
    }

    writer.writeln_tab(1, "}");
    writer.writeln("");
    writer.writeln_tab(1, "args.is_some()");
    writer.writeln("}");

    writer.show().to_string()
}

pub fn generate_struct_parameters(tab: usize, params: &[StructFieldASTNode]) -> String {
    let mut writer = Writer::default();

    for param in params {
        let type_id = generate_type_id(&param.type_id);
        writer.writeln_tab(tab, &format!("{}: {},", param.name, type_id));
    }

    writer.show().to_string()
}

pub fn generate_tuple_parameters(tab: usize, params: &[TupleFieldASTNode]) -> String {
    let mut writer = Writer::default();

    for param in params {
        let type_id = generate_type_id(&param.type_id);
        writer.writeln_tab(tab, &format!("{},", type_id));
    }

    writer.show().to_string()
}

pub fn generate_type_id(type_id: &TypeIDASTNode) -> String {
    match type_id {
        TypeIDASTNode::Integer {
            id,
            size: _,
            signed: _,
        } => id.clone(),
        TypeIDASTNode::Number { id, size: _ } => id.clone(),
        TypeIDASTNode::Bool { id } => id.clone(),
        TypeIDASTNode::Char { id } => id.clone(),
        TypeIDASTNode::Other { id } => id.clone(),
    }
}

pub fn generate_enum_model(node: &EnumASTNode) -> String {
    let mut writer = Writer::default();

    writer.writeln("#[derive(Debug, Clone, PartialEq)]");
    writer.writeln(&format!("pub enum {} {{", node.id));

    for item in node.items.iter() {
        match item {
            EnumItemASTNode::Empty { position: _, id } => {
                writer.writeln_tab(1, &format!("{},", id))
            }
            EnumItemASTNode::Tuple {
                position: _,
                id,
                values,
            } => {
                writer.writeln_tab(1, &format!("{}(", id));
                writer.write(&generate_tuple_parameters(2, values));
                writer.writeln_tab(1, "),");
            }
            EnumItemASTNode::Struct {
                position: _,
                id,
                fields,
            } => {
                writer.writeln_tab(1, &format!("{} {{", id));
                writer.write(&generate_struct_parameters(2, fields));
                writer.writeln_tab(1, "},");
            }
        }
    }

    writer.writeln("}");
    writer.show().to_string()
}

pub fn generate_const_value(node: &ConstValueASTNode) -> String {
    match node {
        ConstValueASTNode::Literal {
            literal,
            type_id: _,
        } => {
            match literal {
                Literal::StringLiteral(value) => format!("\"{}\"", value),
                Literal::IntLiteral(value) => format!("{}", value),
                Literal::NumberLiteral(value) => format!("{}", value),
            }
        }
    }
}

pub fn generate_struct_buffers(node: &StructASTNode) -> String {
    let mut writer = Writer::default();

    writer.writeln(&format!("impl IntoVMBuffers for {} {{", node.id));

    if node.fields.is_empty() {
        writer.writeln_tab(1, "fn read_from_buffers(_: &mut BytesReader) -> Self {");
        writer.writeln_tab(2, &node.id);
        writer.writeln_tab(1, "}");
        writer.writeln("");
        writer.writeln_tab(1, "fn write_to_buffers(&self, _: &mut BytesWriter) {}");
        writer.writeln("");
        writer.writeln_tab(1, "fn skip_in_buffers(_: &mut BytesReader, _: u64) {}");
    } else {
        writer.writeln_tab(
            1,
            "fn read_from_buffers(bytes_reader: &mut BytesReader) -> Self {",
        );

        writer.writeln_tab(2, "Self {");

        for field in node.fields.iter() {
            writer.writeln_tab(
                3,
                &format!("{}: {},", field.name, generate_read(&field.type_id)),
            );
        }

        writer.writeln_tab(2, "}");
        writer.writeln_tab(1, "}");

        writer.writeln("");

        writer.writeln_tab(
            1,
            "fn write_to_buffers(&self, bytes_writer: &mut BytesWriter) {",
        );

        for field in node.fields.iter() {
            writer.writeln_tab(
                2,
                &generate_write(&field.type_id, &format!("self.{}", field.name), false),
            );
        }

        writer.writeln_tab(1, "}");

        writer.writeln("");
        writer.writeln_tab(
            1,
            "fn skip_in_buffers(bytes_reader: &mut BytesReader, count: u64) {",
        );
        writer.writeln_tab(2, "for _ in 0..count {");

        for field in node.fields.iter() {
            writer.writeln_tab(3, &format!("{};", generate_read(&field.type_id)));
        }

        writer.writeln_tab(2, "}");
        writer.writeln_tab(1, "}");
    }

    writer.writeln("}");

    writer.show().to_string()
}

pub fn generate_enum_buffers(node: &EnumASTNode) -> String {
    let mut writer = Writer::default();

    writer.writeln(&format!("impl IntoVMBuffers for {} {{", node.id));
    writer.write(&generate_enum_buffers_read_from_buffers(node));
    writer.writeln("");
    writer.write(&generate_enum_buffers_write_to_buffers(node));
    writer.writeln("");
    writer.write(&generate_enum_buffers_skip(node));
    writer.writeln("}");

    writer.show().to_string()
}

pub fn generate_enum_buffers_read_from_buffers(node: &EnumASTNode) -> String {
    let mut writer = Writer::default();

    writer.writeln_tab(
        1,
        "fn read_from_buffers(bytes_reader: &mut BytesReader) -> Self {",
    );

    writer.writeln_tab(
        2,
        &format!(
            "let value = {};",
            generate_read(&TypeIDASTNode::u32_type_id())
        ),
    );
    writer.writeln("");
    writer.writeln_tab(2, "match value {");

    for item in node.items.iter() {
        match item {
            EnumItemASTNode::Empty { position, id } => {
                writer.writeln_tab(3, &format!("{} => return {}::{},", position, node.id, id));
            }
            EnumItemASTNode::Tuple {
                position,
                id,
                values,
            } => {
                writer.writeln_tab(3, &format!("{} => return {}::{}(", position, node.id, id));

                for value in values {
                    writer.writeln_tab(4, &format!("{},", &generate_read(&value.type_id)));
                }

                writer.writeln_tab(3, "),");
            }
            EnumItemASTNode::Struct {
                position,
                id,
                fields,
            } => {
                writer.writeln_tab(3, &format!("{} => return {}::{} {{", position, node.id, id));

                for field in fields {
                    writer.writeln_tab(
                        4,
                        &format!("{}: {},", field.name, &generate_read(&field.type_id)),
                    );
                }

                writer.writeln_tab(3, "},");
            }
        }
    }

    writer.writeln_tab(3, "_ => panic!(\"Unsupported enum value: {}\", value),");
    writer.writeln_tab(2, "}");
    writer.writeln_tab(1, "}");

    writer.show().to_string()
}

pub fn generate_enum_buffers_write_to_buffers(node: &EnumASTNode) -> String {
    let mut writer = Writer::default();

    writer.writeln_tab(
        1,
        "fn write_to_buffers(&self, bytes_writer: &mut BytesWriter) {",
    );

    writer.writeln_tab(2, "match self {");

    for item in node.items.iter() {
        match item {
            EnumItemASTNode::Empty { position, id } => {
                writer.writeln_tab(3, &format!("{}::{} => {{", node.id, id));
                writer.writeln_tab(
                    4,
                    &generate_write(&TypeIDASTNode::u32_type_id(), &position.to_string(), false),
                );
                writer.writeln_tab(3, "},");
            }
            EnumItemASTNode::Tuple {
                position,
                id,
                values,
            } => {
                writer.writeln_tab(3, &format!("{}::{}(", node.id, id));

                for (i, _) in values.iter().enumerate() {
                    writer.writeln_tab(4, &format!("v{},", i));
                }

                writer.writeln_tab(3, ") => {");
                writer.writeln_tab(
                    4,
                    &generate_write(&TypeIDASTNode::u32_type_id(), &position.to_string(), false),
                );

                for (i, value) in values.iter().enumerate() {
                    writer
                        .writeln_tab(4, &generate_write(&value.type_id, &format!("v{}", i), true));
                }

                writer.writeln_tab(3, "},");
            }
            EnumItemASTNode::Struct {
                position,
                id,
                fields,
            } => {
                writer.writeln_tab(3, &format!("{}::{} {{", node.id, id));

                for field in fields {
                    writer.writeln_tab(4, &format!("{},", field.name));
                }

                writer.writeln_tab(3, "} => {");
                writer.writeln_tab(
                    4,
                    &generate_write(&TypeIDASTNode::u32_type_id(), &position.to_string(), false),
                );

                for field in fields {
                    writer.writeln_tab(4, &generate_write(&field.type_id, &field.name, true));
                }

                writer.writeln_tab(3, "},");
            }
        }
    }

    writer.writeln_tab(2, "}");
    writer.writeln_tab(1, "}");

    writer.show().to_string()
}

pub fn generate_enum_buffers_skip(node: &EnumASTNode) -> String {
    let mut writer = Writer::default();

    writer.writeln_tab(
        1,
        "fn skip_in_buffers(bytes_reader: &mut BytesReader, count: u64) {",
    );

    writer.writeln_tab(2, "for _ in 0..count {");

    writer.writeln_tab(
        3,
        &format!(
            "let value = {};",
            generate_read(&TypeIDASTNode::u32_type_id())
        ),
    );
    writer.writeln("");
    writer.writeln_tab(3, "match value {");

    for item in node.items.iter() {
        match item {
            EnumItemASTNode::Empty { position, id: _ } => {
                writer.writeln_tab(4, &format!("{} => (),", position));
            }
            EnumItemASTNode::Tuple {
                position,
                id: _,
                values,
            } => {
                writer.writeln_tab(4, &format!("{} => {{", position));

                for value in values {
                    writer.writeln_tab(5, &format!("{};", &generate_read(&value.type_id)));
                }

                writer.writeln_tab(4, "},");
            }
            EnumItemASTNode::Struct {
                position,
                id: _,
                fields,
            } => {
                writer.writeln_tab(4, &format!("{} => {{", position));

                for field in fields {
                    writer.writeln_tab(5, &format!("{};", &generate_read(&field.type_id)));
                }

                writer.writeln_tab(4, "},");
            }
        }
    }

    writer.writeln_tab(4, "_ => panic!(\"Unsupported enum value: {}\", value),");
    writer.writeln_tab(3, "}");
    writer.writeln_tab(2, "}");

    writer.writeln_tab(1, "}");

    writer.show().to_string()
}

pub fn generate_read(type_id: &TypeIDASTNode) -> String {
    match type_id {
        TypeIDASTNode::Integer {
            id,
            size: _,
            signed: _,
        } => format!("bytes_reader.read_{}()", id),
        TypeIDASTNode::Number { id, size: _ } => format!("bytes_reader.read_{}()", id),
        TypeIDASTNode::Bool { id } => format!("bytes_reader.read_{}()", id),
        TypeIDASTNode::Char { id } => format!("bytes_reader.read_{}()", id),
        TypeIDASTNode::Other { id } => format!("{}::read_from_buffers(bytes_reader)", id),
    }
}

pub fn generate_write(type_id: &TypeIDASTNode, accessor: &str, deref: bool) -> String {
    let deref_accessor = format!("*{}", accessor);
    let primitive_accessor = if deref { &deref_accessor } else { accessor };

    match type_id {
        TypeIDASTNode::Integer {
            id,
            size: _,
            signed: _,
        } => format!("bytes_writer.write_{}({});", id, primitive_accessor),
        TypeIDASTNode::Number { id, size: _ } => {
            format!("bytes_writer.write_{}({});", id, primitive_accessor)
        }
        TypeIDASTNode::Bool { id } => format!("bytes_writer.write_{}({});", id, primitive_accessor),
        TypeIDASTNode::Char { id } => format!("bytes_writer.write_{}({});", id, primitive_accessor),
        TypeIDASTNode::Other { id: _ } => {
            format!("{}.write_to_buffers(bytes_writer);", accessor)
        }
    }
}

pub fn generate_default_const(type_id: &TypeIDASTNode) -> String {
    match type_id {
        TypeIDASTNode::Integer {
            id: _,
            size: _,
            signed: _,
        } => String::from(""),
        TypeIDASTNode::Number { id: _, size: _ } => String::from("0.0"),
        TypeIDASTNode::Bool { id: _ } => String::from("false"),
        TypeIDASTNode::Char { id: _ } => String::from("0"),
        TypeIDASTNode::Other { id } => format!("{}::default()", id),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::{lexer::Lexer, parser::parse};

    use super::*;

    #[test]
    fn generate_empty_file() {
        let src = fs::read_to_string("test_resources/empty.tpb").unwrap();
        let target = fs::read_to_string("test_resources/rust/empty.rs").unwrap();
        let mut lexer = Lexer::tokenize(&src);
        let ast = parse(&mut lexer);
        let actual = generate(&ast, true, true, true);
        assert_eq!(actual, target);
    }

    #[test]
    fn generate_empty_struct_model() {
        let src = fs::read_to_string("test_resources/empty_struct.tpb").unwrap();
        let target = fs::read_to_string("test_resources/rust/empty_struct_models.rs").unwrap();
        let mut lexer = Lexer::tokenize(&src);
        let ast = parse(&mut lexer);
        let actual = generate_models(&ast);
        println!("{}", actual);
        assert_eq!(actual, target);
    }

    #[test]
    fn generate_struct_with_parameters_model() {
        let src = fs::read_to_string("test_resources/struct_with_parameters.tpb").unwrap();
        let target =
            fs::read_to_string("test_resources/rust/struct_with_parameters_models.rs").unwrap();
        let mut lexer = Lexer::tokenize(&src);
        let ast = parse(&mut lexer);
        let actual = generate_models(&ast);
        println!("{}", actual);
        assert_eq!(actual, target);
    }

    #[test]
    fn generate_two_structs_models() {
        let src = fs::read_to_string("test_resources/two_empty_structs.tpb").unwrap();
        let target = fs::read_to_string("test_resources/rust/two_empty_structs_models.rs").unwrap();
        let mut lexer = Lexer::tokenize(&src);
        let ast = parse(&mut lexer);
        let actual = generate_models(&ast);
        println!("{}", actual);
        assert_eq!(actual, target);
    }

    #[test]
    fn generate_enum_models() {
        let src = fs::read_to_string("test_resources/enum.tpb").unwrap();
        let target = fs::read_to_string("test_resources/rust/enum_models.rs").unwrap();
        let mut lexer = Lexer::tokenize(&src);
        let ast = parse(&mut lexer);
        let actual = generate_models(&ast);
        println!("{}", actual);
        assert_eq!(actual, target);
    }

    #[test]
    fn generate_empty_struct_buffers() {
        let src = fs::read_to_string("test_resources/empty_struct.tpb").unwrap();
        let target = fs::read_to_string("test_resources/rust/empty_struct_buffers.rs").unwrap();
        let mut lexer = Lexer::tokenize(&src);
        let ast = parse(&mut lexer);
        let actual = generate_buffers(&ast);
        println!("{}", actual);
        assert_eq!(actual, target);
    }

    #[test]
    fn generate_struct_with_parameters_buffer() {
        let src = fs::read_to_string("test_resources/struct_with_parameters.tpb").unwrap();
        let target =
            fs::read_to_string("test_resources/rust/struct_with_parameters_buffers.rs").unwrap();
        let mut lexer = Lexer::tokenize(&src);
        let ast = parse(&mut lexer);
        let actual = generate_buffers(&ast);
        println!("{}", actual);
        assert_eq!(actual, target);
    }

    #[test]
    fn generate_struct_with_external_type_buffer() {
        let src = fs::read_to_string("test_resources/struct_with_external_type.tpb").unwrap();
        let target =
            fs::read_to_string("test_resources/rust/struct_with_external_type_buffers.rs").unwrap();
        let mut lexer = Lexer::tokenize(&src);
        let ast = parse(&mut lexer);
        let actual = generate_buffers(&ast);
        println!("{}", actual);
        assert_eq!(actual, target);
    }

    #[test]
    fn generate_enum_buffers() {
        let src = fs::read_to_string("test_resources/enum.tpb").unwrap();
        let target = fs::read_to_string("test_resources/rust/enum_buffers.rs").unwrap();
        let mut lexer = Lexer::tokenize(&src);
        let ast = parse(&mut lexer);
        let actual = generate_buffers(&ast);
        println!("{}", actual);
        assert_eq!(actual, target);
    }

    #[test]
    fn generate_rpc_method() {
        let src = fs::read_to_string("test_resources/rpc_method.tpb").unwrap();
        let target = fs::read_to_string("test_resources/rust/rpc_method.rs").unwrap();
        let mut lexer = Lexer::tokenize(&src);
        let ast = parse(&mut lexer);
        let actual = generate_rpc(&ast);
        println!("{}", actual);
        assert_eq!(actual, target);
    }

    #[test]
    fn generate_rpc_method_without_ret() {
        let src = fs::read_to_string("test_resources/rpc_method_without_ret.tpb").unwrap();
        let target = fs::read_to_string("test_resources/rust/rpc_method_without_ret.rs").unwrap();
        let mut lexer = Lexer::tokenize(&src);
        let ast = parse(&mut lexer);
        let actual = generate_rpc(&ast);
        println!("{}", actual);
        assert_eq!(actual, target);
    }
}
