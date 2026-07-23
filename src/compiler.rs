use crate::ast::*;
use crate::analysis::*;

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
#include <stdio.h>
#include <stdint.h>
typedef int32_t i32;
typedef const char* str;

{}
            "#, self.file.body.iter()
                .map(|stmt| self.compile_statement(stmt, 0))
                .reduce(|acc, stmt| format!("{acc}\n{stmt}"))
                .unwrap_or_default()
        )
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
                Func { name, return_type, params, body: None } => format!("{};",
                    self.compile_func_decl(name, return_type, params)),
                Func { name, return_type, params, body: Some(body) } => format!("{} {}",
                    self.compile_func_decl(name, return_type, params),
                    self.compile_block_statement(body, indent),
                ),
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

    fn compile_func_decl(&self, name: &str, return_type: &str, params: &[FuncParam]) -> String {
        format!("{} {}({})",
            return_type, name,
            params.iter()
                .map(|param| self.compile_param(param))
                .reduce(|acc, s| format!("{acc}, {s}"))
                .unwrap_or_default())
    }

    fn compile_param(&self, param: &FuncParam) -> String {
        format!("{} {}", param.ty, param.name)
    }
}
