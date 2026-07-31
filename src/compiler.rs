use std::collections::HashMap;

use crate::analysis::{Analysis, Builtin, Type};
use crate::ast::{self, BlockStmt, Expression, File, FuncParam, Statement};

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
        let ordered_types = self.get_ordered_types();

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
            self.compile_type_decls(&ordered_types),
            self.compile_type_defs(&ordered_types),
            self.compile_func_decls(&self.file.body),
            self.compile_func_defs(&self.file.body),
        )
    }

    fn get_ordered_types(&self) -> Vec<Type> {
        let mut aux: HashMap<Type, (Vec<Type>, u64)> = HashMap::new();
        // this maps every type to a list of types that have it as a dep
        // and the number of deps it has left

        for ty in &self.analysis.types_used {
            aux.insert(ty.clone(), (vec![], 0));
        }

        for ty in &self.analysis.types_used {
            match ty {
                Type::Func(inner) => {
                    aux.get_mut(&ty).unwrap().1 += 1 + inner.params.len() as u64;

                    aux.get_mut(&inner.return_type).unwrap().0.push(ty.clone());
                    for param in &inner.params {
                        aux.get_mut(param).unwrap().0.push(ty.clone());
                    }
                }
                Type::Ptr(_) => (),
                Type::Array(inner, _) => {
                    aux.get_mut(&ty).unwrap().1 += 1;
                    aux.get_mut(inner).unwrap().0.push(ty.clone());
                }
                Type::Slice(_) => (),
                Type::Void | Type::Primitive(_) | Type::Builtin(_) => (),
            }
        }

        let mut q = vec![];
        for (ty, (_, deps)) in &aux {
            if *deps == 0 {
                q.push(ty.clone());
            }
        }

        let mut res = vec![];
        while !q.is_empty() {
            let ty = q.pop().unwrap();

            let children = aux.get(&ty).unwrap().0.clone();
            for child in children {
                let (_, deps) = aux.get_mut(&child).unwrap();
                *deps -= 1;

                if *deps == 0 {
                    q.push(child);
                }
            }

            res.push(ty);
        }

        res
    }

    fn compile_type_decls(&self, decls: &[Type]) -> String {
        decls.iter()
            .filter_map(|ty| match ty {
                Type::Array(..) => Some(format!("struct __Array_{};", ty.get_hash())),
                Type::Slice(..) => Some(format!("struct __Slice_{};", ty.get_hash())),
                _ => None,
            })
            .reduce(|acc, decl| format!("{acc}\n{decl}"))
            .unwrap_or_default()
    }

    fn compile_type_defs(&self, defs: &[Type]) -> String {
        defs.iter()
            .filter_map(|ty| match ty {
                Type::Array(inner, size) => Some(format!("\
struct __Array_{} {{
    {} inner[{}];
}};",
                    ty.get_hash(),
                    self.compile_analysis_type(inner),
                    size)),
                Type::Slice(inner) => Some(format!("\
struct __Slice_{} {{
    {}* inner;
    u64 len;
}};",
                    ty.get_hash(),
                    self.compile_analysis_type(inner))),
                _ => None,
            })
            .reduce(|acc, decl| format!("{acc}\n{decl}"))
            .unwrap_or_default()
    }

    fn compile_func_decls(&self, body: &[Statement]) -> String {
        body.iter()
            .filter_map(|stmt| match stmt {
                Statement::Func { name, return_type, params, .. }
                    => Some((name, return_type, params)),
                _ => None,
            })
            .map(|(name, return_type, params)| format!("{} {}({});",
                self.compile_type(return_type),
                name,
                params.iter()
                    .map(|param| self.compile_param(&param.name, &param.ty))
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
                            match ty {
                                Some(ty) => self.compile_type(ty),
                                None => self.compile_analysis_type(self.analysis.get_type_of_expr(value)),
                            },
                            name,
                            match ty {
                                Some(ty) => self.coerce_and_compile_expr(value,
                                    self.analysis.type_map.get(ty).unwrap()),
                                None => self.compile_expression(value),
                            }),
                        None => format!("{} {};",
                            self.compile_type(ty.as_ref().unwrap()),
                            name),
                    }
                }
                Return { value } => format!("return {};",
                    self.compile_expression(value)),
                If { cond, then, else_then } => format!("if ({}) {} else {}",
                    self.compile_expression(cond),
                    self.compile_block_statement(then, indent),
                    self.compile_block_statement(else_then, indent),
                ),
                While { cond, block } => format!("while ({}) {}",
                    self.compile_expression(cond),
                    self.compile_block_statement(block, indent),
                ),
                Expr { value } => format!("{};",
                    self.compile_expression(value)),
                Func { name, return_type, params, body: Some(body), } => format!("{} {}",
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
            Ident { value, .. } => value.into(),
            Int { value } => value.into(),
            String { value } => format!("\"{value}\""),
            Unary { op, right } => format!("{op}{}",
                self.compile_expression(right)),
            Binary { op, left, right } => format!("{} {op} {}",
                self.compile_expression(left),
                self.compile_expression(right)),
            Call { func, args } => self.compile_call_expr(func, args),
            At { left, right } => format!("({}).inner[{}]",
                self.compile_expression(left),
                self.compile_expression(right),
            ),
        }
    }

    fn compile_call_expr(&self, func: &Expression, args: &[Expression]) -> String {
        match self.analysis.get_type_of_expr(func) {
            Type::Builtin(builtin) => self.compile_builtin_call_expr(*builtin, args),
            Type::Func(ty) => format!("{}({})",
                self.compile_expression(func),
                args.iter()
                    .zip(&ty.params)
                    .map(|(arg, ty)| self.coerce_and_compile_expr(arg, ty))
                    .reduce(|acc, s| format!("{acc}, {s}"))
                    .unwrap_or_default()),
            _ => unreachable!(),
        }
    }

    fn compile_builtin_call_expr(&self, builtin: Builtin, args: &[Expression]) -> String {
        match builtin {
            Builtin::Len => {
                let arg = &args[0];

                match self.analysis.get_type_of_expr(arg) {
                    Type::Array(_, len) => format!("{len}"),
                    Type::Slice(_) => format!("({}).len", self.compile_expression(arg)),
                    _ => unreachable!(),
                }
            }
        }
    }

    fn coerce_and_compile_expr(&self, expr: &Expression, expected: &Type) -> String {
        let got = self.analysis.get_type_of_expr(expr);

        match (expected, got) {
            (exp_ty @ Type::Slice(expected), Type::Ptr(inner)) => match **inner {
                Type::Array(ref got, size) if expected == got => format!("({}){{ ({}*)({}), {} }}",
                    self.compile_analysis_type(exp_ty),
                    self.compile_analysis_type(expected),
                    self.compile_expression(expr),
                    size),
                _ => unreachable!(),
            }
            _ => self.compile_expression(expr),
        }
    }

    fn compile_func_decl(&self, name: &str, return_type: &ast::Type, params: &[FuncParam]) -> String {
        format!("{} {}({})",
            self.compile_type(return_type), name,
            params.iter()
                .map(|param| self.compile_param(&param.name, &param.ty))
                .reduce(|acc, s| format!("{acc}, {s}"))
                .unwrap_or_default())
    }

    fn compile_param(&self, name: &str, ty: &ast::Type) -> String {
        format!("{} {}", self.compile_type(ty), name)
    }

    fn compile_type(&self, ty: &ast::Type) -> String {
        self.compile_analysis_type(self.analysis.type_map.get(ty).unwrap())
    }

    fn compile_analysis_type(&self, ty: &Type) -> String {
        match ty {
            Type::Void => "void".into(),
            Type::Primitive(ty) => format!("{ty}"),
            Type::Ptr(ty) => format!("{}*", self.compile_analysis_type(ty)),
            ty @ Type::Array(_, _) => format!("struct __Array_{}", ty.get_hash()),
            ty @ Type::Slice(_) => format!("struct __Slice_{}", ty.get_hash()),
            Type::Func(_) => todo!("this is a bit more difficult :("),
            Type::Builtin(_) => unreachable!(),
        }
    }
}
