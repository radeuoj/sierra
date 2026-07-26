use std::collections::HashMap;
use std::collections::HashSet;

use crate::ast::*;
use crate::analysis::*;

#[allow(unused)]
pub struct Compiler {
    file: File,
    analysis: Analysis,
}

impl Compiler {
    pub fn new(file: File, analysis: Analysis) -> Self {
        Self { file, analysis }
    }

    pub fn compile(&self) -> String {
        format!(r#"// compiled from Sierra
#include <stdint.h>

typedef int8_t i8;
typedef uint8_t u8;
typedef int16_t i16;
typedef uint16_t u16;
typedef int32_t i32;
typedef uint32_t u32;
typedef int64_t i64;
typedef uint64_t u64;
typedef float f32;
typedef double f64;

{}

{}

{}

{}
            "#,
            self.compile_array_decls(&self.analysis.arrays),
            self.compile_array_defs(&self.analysis.arrays),
            self.compile_func_decls(&self.analysis.func_decls),
            self.compile_func_defs(&self.file.body),
        )
    }

    fn compile_array_decls(&self, decls: &HashSet<(Type, u64)>) -> String {
        decls.iter()
            .map(|(ty, size)| format!("struct __Array_{};",
                hash_array(ty, *size)))
            .reduce(|acc, decl| format!("{acc}\n{decl}"))
            .unwrap_or_default()
    }

    fn compile_array_defs(&self, defs: &HashSet<(Type, u64)>) -> String {
        defs.iter()
            .map(|(ty, size)| format!("\
struct __Array_{} {{
    {} inner[{}];
}};",
                hash_array(ty, *size),
                self.compile_type(ty),
                size))
            .reduce(|acc, def| format!("{acc}\n{def}"))
            .unwrap_or_default()
    }

    fn compile_func_decls(&self, decls: &HashMap<String, FuncType>) -> String {
        decls.iter()
            .map(|(name, ty)| format!("{} {}({});",
                self.compile_type(&ty.return_type),
                name,
                ty.params.iter()
                    .map(|param| self.compile_type(param))
                    .reduce(|acc, param| format!("{acc}, {param}"))
                    .unwrap_or_default()
            ))
            .reduce(|acc, decl| format!("{acc}\n{decl}"))
            .unwrap_or_default()
    }

    fn compile_func_defs(&self, body: &[Statement]) -> String {
        body.iter()
            .filter(|stmt| match stmt {
                Statement::Func { body: Some(_), .. } => true,
                _ => false
            })
            .map(|stmt| self.compile_statement(stmt, 0))
            .reduce(|acc, stmt| format!("{acc}\n{stmt}"))
            .unwrap_or_default()
    }

    fn compile_statement(&self, stmt: &Statement, indent: i32) -> String {
        use Statement::*;

        let indent_str = "    ".repeat(indent as usize);
        format!("{}{}",
            indent_str,
            match stmt {
                Let { name, ty, value } => {
                    match value {
                        Some(value) => format!("{} {} = {};",
                            self.compile_type(self.analysis.get_type_of_expr(value)),
                            name,
                            self.compile_expression(value)),
                        None => format!("{} {};",
                            self.compile_type(ty.as_ref().unwrap()), name),
                    }
                }
                Return { value } => format!("return {};",
                    self.compile_expression(value)),
                If { cond, then, else_then } => format!("if ({}) {} else {}",
                    self.compile_expression(cond),
                    self.compile_block_statement(then, indent),
                    self.compile_block_statement(else_then, indent),
                ),
                Expr { value } => format!("{};",
                    self.compile_expression(value)),
                Func { name, return_type, params, body: Some(body) } => format!("{} {}",
                    self.compile_func_decl(name, return_type, params),
                    self.compile_block_statement(body, indent),
                ),
                Func { body: None, .. } => "".into(), // skip
            }
        )
    }

    fn compile_block_statement(&self, block: &BlockStmt, indent: i32) -> String {
        format!("{{\n{}\n{}}}",
            self.compile_statements(block, indent + 1),
            "    ".repeat(indent as usize))
    }

    fn compile_statements(&self, stmts: &[Statement], indent: i32) -> String {
        stmts.iter()
            .map(|stmt| self.compile_statement(stmt, indent))
            .reduce(|acc, stmt| format!("{acc}\n{stmt}"))
            .unwrap_or_default()
    }

    fn compile_expression(&self, expr: &Expression) -> String {
        use Expression::*;

        match expr {
            Ident { value } => value.into(),
            Int { value } => value.into(),
            String { value } => format!("\"{value}\""),
            Unary { op, right } => format!("{op}{}",
                self.compile_expression(right)),
            Binary { op, left, right } => format!("{} {op} {}",
                self.compile_expression(left),
                self.compile_expression(right)),
            Call { func, args } => format!("{}({})",
                self.compile_expression(func),
                args.iter()
                    .map(|arg| self.compile_expression(arg))
                    .reduce(|acc, s| format!("{acc}, {s}"))
                    .unwrap_or_default()),
            At { left, right } => format!("({}).inner[{}]",
                self.compile_expression(left),
                self.compile_expression(right),
            ),
        }
    }

    fn compile_func_decl(&self, name: &str, return_type: &Type, params: &[FuncParam]) -> String {
        format!("{} {}({})",
            self.compile_type(return_type), name,
            params.iter()
                .map(|param| self.compile_param(param))
                .reduce(|acc, s| format!("{acc}, {s}"))
                .unwrap_or_default())
    }

    fn compile_param(&self, param: &FuncParam) -> String {
        format!("{} {}", self.compile_type(&param.ty), param.name)
    }

    fn compile_type(&self, ty: &Type) -> String {
        match ty {
            Type::Void => "void".into(),
            Type::Primitive(ty) => match ty {
                Primitive::I8 => "i8".into(),
                Primitive::U8 => "u8".into(),
                Primitive::I16 => "i16".into(),
                Primitive::U16 => "u16".into(),
                Primitive::I32 => "i32".into(),
                Primitive::U32 => "u32".into(),
                Primitive::I64 => "i64".into(),
                Primitive::U64 => "u64".into(),
                Primitive::F32 => "f32".into(),
                Primitive::F64 => "f64".into(),
            }
            Type::Func(_) => todo!("this is a bit more difficult :("),
            Type::Ptr(ty) => format!("{}*", self.compile_type(ty)),
            Type::Array(ty, size) => format!("struct __Array_{}", hash_array(ty, *size)),
        }
    }
}
