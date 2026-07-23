use std::collections::HashMap;

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

{}

{}

{}
            "#,
            self.compile_typedefs(&self.analysis.types),
            self.compile_func_decls(&self.analysis.func_decls),
            self.compile_func_defs(&self.file.body),
        )
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
                            ty, name, self.compile_expression(value)),
                        None => format!("{} {};",
                            ty, name),
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
                    self.compile_func_decl(name, return_type.as_deref(), params),
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
        }
    }

    fn compile_func_decl(&self, name: &str, return_type: Option<&str>, params: &[FuncParam]) -> String {
        format!("{} {}({})",
            return_type.unwrap_or("void"), name,
            params.iter()
                .map(|param| self.compile_param(param))
                .reduce(|acc, s| format!("{acc}, {s}"))
                .unwrap_or_default())
    }

    fn compile_param(&self, param: &FuncParam) -> String {
        format!("{} {}", param.ty, param.name)
    }

    fn compile_typedefs(&self, types: &HashMap<String, NamedType>) -> String {
        types.iter()
            .map(|(to, from)| format!("typedef {} {};", self.compile_named_type(from), to))
            .reduce(|acc, ty| format!("{acc}\n{ty}"))
            .unwrap_or_default()
    }

    fn compile_named_type(&self, ty: &NamedType) -> String {
        match ty {
            NamedType::Primitive(ty) => ty.clone(),
        }
    }

    fn compile_type(&self, ty: &Type) -> String {
        match ty {
            Type::Void => "void".into(),
            Type::Named(name) => name.into(),
            Type::Func(_) => todo!("this is a bit more difficult :("),
        }
    }
}
