use std::collections::HashMap;

use crate::analysis::{Analysis, FuncType, Primitive, Type};
use crate::ast::{BlockStmt, Expression, File, FuncParam, Statement};

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
            self.compile_func_decls(&self.analysis.func_decls),
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
                Type::Void | Type::Primitive(_) => (),
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
                    self.compile_type(inner),
                    size)),
                Type::Slice(inner) => Some(format!("\
struct __Slice_{} {{
    {}* inner;
    u64 len;
}};",
                    ty.get_hash(),
                    self.compile_type(inner))),
                _ => None,
            })
            .reduce(|acc, decl| format!("{acc}\n{decl}"))
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
                            self.compile_type(self.analysis.type_map.get(ty.as_ref().unwrap()).unwrap()),
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
                Expr { value } => format!("{};",
                    self.compile_expression(value)),
                Func { name, params, body: Some(body), .. } => format!("{} {}",
                    self.compile_func_decl(name, self.analysis.func_decls.get(name).unwrap(), params),
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

    fn compile_func_decl(&self, name: &str, func_type: &FuncType, params: &[FuncParam]) -> String {
        format!("{} {}({})",
            self.compile_type(&func_type.return_type), name,
            params.iter()
                .zip(func_type.params.iter())
                .map(|(param, ty)| self.compile_param(&param.name, ty))
                .reduce(|acc, s| format!("{acc}, {s}"))
                .unwrap_or_default())
    }

    fn compile_param(&self, name: &str, ty: &Type) -> String {
        format!("{} {}", self.compile_type(ty), name)
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
            ty @ Type::Array(_, _) => format!("struct __Array_{}", ty.get_hash()),
            ty @ Type::Slice(_) => format!("struct __Slice_{}", ty.get_hash()),
        }
    }
}
