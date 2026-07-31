use std::{collections::{HashMap, HashSet}, fmt::Display, hash::{DefaultHasher, Hash, Hasher}};

use anyhow::{Context, Result, bail};

use crate::ast::{self, BlockStmt, ExprHash, Expression, File, FuncParam, Statement};
use crate::token::Token;

#[derive(Debug)]
pub struct Analysis {
    pub named_types: HashMap<String, Type>,
    pub type_map: HashMap<ast::Type, Type>,
    pub expr_types: HashMap<ExprHash, Type>,
    pub types_used: HashSet<Type>,
}

/**
 * so check functions do type checking and add types to all expressions
 */
impl Analysis {
    pub fn new(file: &File) -> Result<Self> {
        let mut analysis = Self {
            expr_types: HashMap::new(),
            type_map: HashMap::new(),
            types_used: HashSet::new(),
            named_types: HashMap::new(),
        };

        let mut global = Scope::new(Type::Void);

        analysis.add_primitive_types();
        analysis.check_top_level(file, &mut global)?;
        analysis.check_func_bodies(file, &global)?;
        analysis.collect_used_types(&global);

        Ok(analysis)
    }

    fn add_primitive_types(&mut self) {
        self.named_types.insert("i8".into(), Type::Primitive(Primitive::I8));
        self.named_types.insert("u8".into(), Type::Primitive(Primitive::U8));
        self.named_types.insert("i16".into(), Type::Primitive(Primitive::I16));
        self.named_types.insert("u16".into(), Type::Primitive(Primitive::U16));
        self.named_types.insert("i32".into(), Type::Primitive(Primitive::I32));
        self.named_types.insert("u32".into(), Type::Primitive(Primitive::U32));
        self.named_types.insert("i64".into(), Type::Primitive(Primitive::I64));
        self.named_types.insert("u64".into(), Type::Primitive(Primitive::U64));
        self.named_types.insert("f32".into(), Type::Primitive(Primitive::F32));
        self.named_types.insert("f64".into(), Type::Primitive(Primitive::F64));
    }

    /**
     * panics if expression has not been checked
     */
    pub fn get_type_of_expr(&self, expr: &Expression) -> &Type {
        self.expr_types.get(&expr.get_hash()).unwrap()
    }

    /**
     * turns a type expression from the AST into a semantic type
     */
    fn resolve_type(&mut self, ty: &ast::Type) -> Result<Type> {
        let resolved = match ty {
            ast::Type::Void => Type::Void,
            ast::Type::Named(name) => self.named_types.get(name).cloned()
                .with_context(|| format!("{name} is not a type"))?,
            ast::Type::Ptr(inner) => Type::Ptr(Box::new(self.resolve_type(inner)?)),
            ast::Type::Array(inner, size) => Type::Array(Box::new(self.resolve_type(inner)?), *size),
            ast::Type::Slice(inner) => Type::Slice(Box::new(self.resolve_type(inner)?)),
        };

        self.type_map.insert(ty.clone(), resolved.clone());
        Ok(resolved)
    }

    fn check_top_level(&mut self, file: &File, scope: &mut Scope) -> Result<()> {
        for stmt in &file.body {
            match stmt {
                Statement::Func {
                    name,
                    return_type,
                    params,
                    ..
                } => self.check_func(name, return_type, params, scope)?,
                _ => bail!("only functions are allowed at top level"),
            }
        }

        Ok(())
    }

    fn check_func(
        &mut self,
        name: &str,
        return_type: &ast::Type,
        params: &[FuncParam],
        scope: &mut Scope,
    ) -> Result<()> {
        let return_type = self.resolve_type(return_type)?;

        let mut param_types = vec![];
        for param in params {
            let ty = self.resolve_type(&param.ty)?;
            param_types.push(ty);
        }

        if scope.contains_local(name) {
            bail!("{name} already exists");
        }

        let mut param_names = HashSet::new();
        for param in params {
            if param_names.contains(&param.name) {
                bail!("{} already exists", param.name);
            }

            param_names.insert(param.name.clone());
        }

        scope.add(name, Type::Func(Box::new(FuncType {
            return_type,
            params: param_types,
        })));

        Ok(())
    }

    /**
     * this assumes that param types and names and function return type have been already checked
     */
    fn check_func_bodies(&mut self, file: &File, scope: &Scope) -> Result<()> {
        let mut errs = vec![];

        for stmt in &file.body {
            let Statement::Func {
                name,
                params,
                body,
                ..
            } = stmt else {
                unreachable!("already checked");
            };

            let Some(body) = body else { continue };

            let Type::Func(ty) = scope.get_type_of(name).unwrap() else {
                unreachable!("already checked");
            };

            if let Err(err) = self.check_func_body(*ty, params, body, scope) {
                errs.push(err);
            }
        }

        join_errs(errs)
    }

    fn check_func_body(&mut self, func_type: FuncType, params: &[FuncParam], body: &BlockStmt, scope: &Scope) -> Result<()> {
        let return_type = &func_type.return_type;
        let params = params.iter().map(|param| &param.name).zip(func_type.params.iter());
        let mut scope = scope.get_child_with(return_type.clone());

        for (param, ty) in params {
            scope.add(param, ty.clone());
        }

        self.check_block(body, &mut scope)?;

        Ok(())
    }

    fn check_expr(&mut self, expr: &Expression, scope: &Scope) -> Result<()> {
        match expr {
            Expression::Ident { value } => self.check_ident(expr, value, scope),
            Expression::Int { .. } => self.check_int(expr),
            Expression::String { .. } => todo!("we don't support strings atm"),
            Expression::Unary { op, right } => self.check_unary(expr, op, right, scope),
            Expression::Binary { op, left, right } => self.check_binary(expr, op, left, right, scope),
            Expression::Call { func, args } => self.check_call(expr, func, args, scope),
            Expression::At { left, right } => self.check_at(expr, left, right, scope),
        }
    }

    fn check_ident(&mut self, expr: &Expression, value: &str, scope: &Scope) -> Result<()> {
        let Some(ty) = scope.get_type_of(value) else {
            bail!("{value} does not exist");
        };

        self.expr_types.insert(expr.get_hash(), ty);

        Ok(())
    }

    fn check_int(&mut self, expr: &Expression) -> Result<()> {
        self.expr_types.insert(expr.get_hash(), Type::Primitive(Primitive::I32));

        Ok(())
    }

    fn check_unary(&mut self, expr: &Expression, op: &Token, right: &Expression, scope: &Scope) -> Result<()> {
        self.check_expr(right, scope)?;

        let ty = match (op, self.get_type_of_expr(right)) {
            (Token::Ampersand, ty) => match right {
                Expression::Ident { .. } | Expression::Unary { op: Token::Asterisk, .. }
                | Expression::At { .. } => Type::Ptr(Box::new(ty.clone())),
                _ => bail!("right side of & unary operator must be an lvalue"),
            }
            (Token::Asterisk, Type::Ptr(ty)) => *ty.clone(),
            (op, ty @ Type::Primitive(_)) if *op != Token::Asterisk => ty.clone(),
            (op, ty) => bail!("type {} is incompatible with unary operator {}", ty, op),
        };

        self.expr_types.insert(expr.get_hash(), ty);

        Ok(())
    }

    fn check_binary(&mut self, expr: &Expression, op: &Token, left: &Expression, right: &Expression, scope: &Scope) -> Result<()> {
        self.check_expr(left, scope)?;
        self.check_expr(right, scope)?;

        let return_ty = match (op, self.get_type_of_expr(left), self.get_type_of_expr(right)) {
            (op, Type::Primitive(left_ty), Type::Primitive(right_ty)) if *op != Token::Assign
                => Type::Primitive(left_ty.max(right_ty).clone()),
            (Token::Assign, left_ty, right_ty) if left_ty == right_ty => match left {
                Expression::Ident { .. } | Expression::Unary { op: Token::Asterisk, .. }
                | Expression::At { .. } => right_ty.clone(),
                _ => bail!("left side of = operator must be an lvalue"),
            }
            (op, left_ty, right_ty) => bail!("types {} and {} are incompatible with binary operator {}", left_ty, right_ty, op),
        };

        self.expr_types.insert(expr.get_hash(), return_ty);

        Ok(())
    }

    fn try_coercion(&self, expected: &Type, got: &Type) -> Result<Type> {
        match (expected, got) {
            (expected, got) if expected == got => Ok(expected.clone()),
            (Type::Primitive(expected), Type::Primitive(_)) => Ok(Type::Primitive(expected.clone())),
            (expected, got) => bail!("expected type {} but got {}", expected, got),
        }
    }

    fn check_call(&mut self, expr: &Expression, func: &Expression, args: &[Expression], scope: &Scope) -> Result<()> {
        self.check_expr(func, scope)?;

        let Type::Func(ty) = self.expr_types.get(&func.get_hash()).unwrap() else {
            bail!("left of call expr is not a function");
        };

        let ty = *ty.clone();

        if args.len() != ty.params.len() {
            bail!("expected {} args but got {}", ty.params.len(), args.len());
        }

        let mut errs = vec![];
        for (arg, ty) in args.iter().zip(ty.params.iter()) {
            if let Err(err) = self.check_expr(arg, scope) {
                errs.push(err);
            }

            if let Err(err) = self.try_coercion(ty, self.expr_types.get(&arg.get_hash()).unwrap()) {
                errs.push(err);
            }
        }

        self.expr_types.insert(expr.get_hash(), ty.return_type.clone());

        join_errs(errs)
    }

    fn check_at(&mut self, expr: &Expression, left: &Expression, right: &Expression, scope: &Scope) -> Result<()> {
        self.check_expr(left, scope)?;
        self.check_expr(right, scope)?;

        let ty = match self.get_type_of_expr(left) {
            Type::Array(ty, _) => &**ty,
            ty => bail!("at expression expected an array but got {}", ty),
        };

        match self.get_type_of_expr(right) {
            Type::Primitive(_) => (),
            ty => bail!("at expression expected a primitive but got {}", ty),
        }

        self.expr_types.insert(expr.get_hash(), ty.clone());

        Ok(())
    }

    fn check_block(&mut self, block: &BlockStmt, scope: &mut Scope) -> Result<()> {
        for stmt in block {
            self.check_stmt(stmt, scope)?;
        }

        Ok(())
    }

    fn check_stmt(&mut self, stmt: &Statement, scope: &mut Scope) -> Result<()> {
        match stmt {
            Statement::Let { name, ty, value } => self.check_let(name, ty, value.into(), scope),
            Statement::Return { value } => self.check_return(value, scope),
            Statement::If { cond, then, else_then } => self.check_if(cond, then, else_then, scope),
            Statement::Func { .. } => bail!("funcs are only allowed at top level"),
            Statement::Expr { value } => self.check_expr(value, scope),
        }
    }

    fn check_let(&mut self, name: &str, ty: &Option<ast::Type>, value: Option<&Expression>, scope: &mut Scope) -> Result<()> {
        let declared_ty = match ty {
            Some(ty) => Some(self.resolve_type(ty)?),
            None => None,
        };

        let value_ty = match value {
            Some(value) => {
                self.check_expr(value, scope)?;
                Some(self.get_type_of_expr(value).clone())
            }
            None => None,
        };

        match (&declared_ty, &value_ty) {
            (Some(declared), Some(got)) => {
                self.try_coercion(declared, got)?;
            }
            (None, None) => bail!("let statement neither has a type nor an assigned value"),
            _ => (),
        }

        if scope.contains_local(name) {
            bail!("{name} already exists");
        }

        let ty = declared_ty.or(value_ty).unwrap();
        scope.add(name, ty);

        Ok(())
    }

    fn check_return(&mut self, value: &Expression, scope: &mut Scope) -> Result<()> {
        self.check_expr(value, scope)?;
        let value_type = self.expr_types.get(&value.get_hash()).unwrap();

        self.try_coercion(&scope.return_type, value_type)?;

        Ok(())
    }

    fn check_if(&mut self, cond: &Expression, then: &BlockStmt, else_then: &BlockStmt, scope: &mut Scope) -> Result<()> {
        self.check_expr(cond, scope)?;

        match self.get_type_of_expr(cond) {
            Type::Primitive(_) => (),
            ty => bail!("if statement condition must be a primitive but instead got {}", ty),
        }

        self.check_block(then, &mut scope.get_child())?;
        self.check_block(else_then, &mut scope.get_child())?;

        Ok(())
    }

    /**
     * collects every type that codegen will need to define, transitively
     */
    fn collect_used_types(&mut self, scope: &Scope) {
        let types: Vec<Type> = scope.symbols.values().cloned()
            .chain(self.expr_types.values().cloned())
            .chain(self.type_map.values().cloned())
            .collect();

        for ty in types {
            self.record_type(&ty);
        }
    }

    fn record_type(&mut self, ty: &Type) {
        if !self.types_used.insert(ty.clone()) {
            return;
        }

        match ty {
            Type::Func(ty) => {
                self.record_type(&ty.return_type);
                for param in &ty.params { self.record_type(param); }
            }
            Type::Ptr(ty) => self.record_type(ty),
            Type::Array(ty, _) => self.record_type(ty),
            Type::Slice(ty) => self.record_type(ty),
            Type::Void | Type::Primitive(_) => (),
        }
    }
}

fn join_errs(errs: Vec<anyhow::Error>) -> Result<()> {
    if errs.is_empty() {
        Ok(())
    } else {
        bail!(errs.iter()
            .map(|err| format!("{err}"))
            .reduce(|acc, err| format!("{acc}\n{err}"))
            .unwrap_or_default())
    }
}

#[derive(Debug)]
struct Scope<'a> {
    parent: Option<&'a Scope<'a>>,
    symbols: HashMap<String, Type>,
    return_type: Type,
}

#[allow(dead_code)]
impl<'a> Scope<'a> {
    fn new(return_type: Type) -> Self {
        Self {
            parent: None,
            symbols: HashMap::new(),
            return_type,
        }
    }

    fn get_child(&'a self) -> Scope<'a> {
        Self {
            parent: Some(self),
            symbols: HashMap::new(),
            return_type: self.return_type.clone(),
        }
    }

    fn get_child_with(&'a self, return_type: Type) -> Scope<'a> {
        Self {
            parent: Some(self),
            symbols: HashMap::new(),
            return_type,
        }
    }

    fn get_type_of(&self, name: &str) -> Option<Type> {
        self.symbols.get(name).cloned()
            .or_else(|| self.parent
                .map(|parent| parent.get_type_of(name))
                .unwrap_or(None))
    }

    fn contains(&self, name: &str) -> bool {
        self.get_type_of(name).is_some()
    }

    fn contains_local(&self, name: &str) -> bool {
        self.symbols.contains_key(name)
    }

    fn add(&mut self, name: &str, ty: Type) {
        self.symbols.insert(name.into(), ty);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Void,
    Primitive(Primitive),
    Func(Box<FuncType>),
    Ptr(Box<Type>),
    Array(Box<Type>, u64),
    Slice(Box<Type>),
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Void => write!(f, "<void>"),
            Type::Primitive(ty) => write!(f, "{ty}"),
            Type::Func(ty) => write!(f, "{ty}"),
            Type::Ptr(ty) => write!(f, "*{ty}"),
            Type::Array(ty, size) => write!(f, "[{size}]{ty}"),
            Type::Slice(ty) => write!(f, "[]{ty}"),
        }
    }
}

impl Type {
    pub fn get_hash(&self) -> TypeHash {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        TypeHash(hasher.finish())
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct TypeHash(u64);

impl Display for TypeHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FuncType {
    pub return_type: Type,
    pub params: Vec<Type>,
}

impl Display for FuncType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fn({}) -> {}",
            self.params.iter()
                .map(|ty| ty.to_string())
                .reduce(|acc, ty| format!("{acc}, {ty}"))
                .unwrap_or_default(),
            self.return_type,
        )
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
pub enum Primitive {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
}

impl Display for Primitive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Primitive::I8 => "i8",
            Primitive::U8 => "u8",
            Primitive::I16 => "i16",
            Primitive::U16 => "u16",
            Primitive::I32 => "i32",
            Primitive::U32 => "u32",
            Primitive::I64 => "i64",
            Primitive::U64 => "u64",
            Primitive::F32 => "f32",
            Primitive::F64 => "f64",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lexer::Lexer, parser::Parser};

    fn analyze(input: &[u8]) -> Result<()> {
        let lexer = Lexer::new(input.to_vec());
        let parser = Parser::new(lexer)?;
        let file = parser.parse_file()?;
        Analysis::new(&file)?;
        Ok(())
    }

    #[test]
    fn valid_function() {
        analyze(b"fn main() -> i32 { let x: i32 = 1; return x; }").unwrap();
    }

    #[test]
    fn undefined_variable() {
        let result = analyze(b"fn main() -> i32 { let x: i32 = y; return x; }");
        assert!(result.is_err());
        let err = format!("{}", result.unwrap_err());
        assert!(err.contains("does not exist"), "{err}");
    }

    #[test]
    fn duplicate_function() {
        let result = analyze(b"fn foo() -> i32 { return 1; } fn foo() -> i32 { return 2; }");
        assert!(result.is_err());
        let err = format!("{}", result.unwrap_err());
        assert!(err.contains("foo already exists"), "{err}");
    }

    #[test]
    fn argument_count_mismatch() {
        let result = analyze(b"fn foo(x: i32) -> i32 { return x; } fn main() -> i32 { return foo(1, 2); }");
        assert!(result.is_err());
        let err = format!("{}", result.unwrap_err());
        assert!(err.contains("expected"), "{err}");
    }

    #[test]
    fn call_non_function() {
        let result = analyze(b"fn main() -> i32 { let x: i32 = 1; return x(1); }");
        assert!(result.is_err());
        let err = format!("{}", result.unwrap_err());
        assert!(err.contains("not a function"), "{err}");
    }

    #[test]
    fn unknown_type() {
        let result = analyze(b"fn main() -> nope { return 1; }");
        assert!(result.is_err());
        let err = format!("{}", result.unwrap_err());
        assert!(err.contains("is not a type"), "{err}");
    }

    #[test]
    fn unknown_type_in_let() {
        let result = analyze(b"fn main() { let x: nope = 1; }");
        assert!(result.is_err());
        let err = format!("{}", result.unwrap_err());
        assert!(err.contains("is not a type"), "{err}");
    }

    #[test]
    fn decl_then_def_is_ok() {
        analyze(b"fn foo() -> i32; fn foo() -> i32 { return 1; }").unwrap();
    }

    #[test]
    fn shadowing_in_inner_block_is_allowed() {
        analyze(b"fn main() -> i32 { let x: i32 = 1; if 1 { let x: i32 = 2; } return x; }").unwrap();
    }

    #[test]
    fn same_scope_redefinition_errors() {
        let result = analyze(b"fn main() { let x: i32 = 1; let x: i32 = 2; }");
        assert!(result.is_err());
        let err = format!("{}", result.unwrap_err());
        assert!(err.contains("already exists"), "{err}");
    }

    #[test]
    fn local_shadows_function() {
        analyze(b"fn foo() -> i32 { return 1; } fn main() -> i32 { let foo: i32 = 1; return foo; }").unwrap();
    }

    #[test]
    fn param_can_be_named_like_a_function() {
        analyze(b"fn foo() -> i32 { return 1; } fn main(foo: i32) -> i32 { return foo; }").unwrap();
    }

    #[test]
    fn param_cannot_be_shadowed_at_body_top() {
        let result = analyze(b"fn main(x: i32) { let x: i32 = 1; }");
        assert!(result.is_err());
        let err = format!("{}", result.unwrap_err());
        assert!(err.contains("already exists"), "{err}");
    }

    #[test]
    fn param_can_be_shadowed_in_if_block() {
        analyze(b"fn main(x: i32) -> i32 { if 1 { let x: i32 = 2; } return x; }").unwrap();
    }

    #[test]
    fn duplicate_params_error() {
        let result = analyze(b"fn foo(x: i32, x: i32) {}");
        assert!(result.is_err());
        let err = format!("{}", result.unwrap_err());
        assert!(err.contains("already exists"), "{err}");
    }
}
