use std::collections::{HashMap, HashSet};

use anyhow::{Result, anyhow, bail};

use crate::{ast::*, token::Token};

#[derive(Debug)]
pub struct Analysis {
    pub expr_types: HashMap<ExprHash, Type>,
    pub func_decls: HashMap<String, FuncType>,
    pub func_defs: HashSet<String>, // whether this function has a definition already
    pub types_used: HashSet<Type>,
}

struct Scope<'a> {
    parent: Option<&'a Scope<'a>>,
    symbols: HashMap<String, Type>,
    return_type: Type,
}

/**
 * so check functions do type checking and add types to all expressions
 */
impl Analysis {
    pub fn new(file: &File) -> Result<Self> {
        let mut analysis = Self {
            expr_types: HashMap::new(),
            func_decls: HashMap::new(),
            func_defs: HashSet::new(),
            types_used: HashSet::new(),
        };

        analysis.check_top_level(file)?;
        let mut errs = vec![];

        for stmt in &file.body {
            match stmt {
                Statement::Func {
                    name,
                    params,
                    body,
                    ..
                } => if let Some(body) = body && let Err(err)
                        = analysis.check_func_body(analysis.func_decls.get(name).unwrap().clone(), params, body)
                {
                    errs.push(err);
                }
                _ => unreachable!("already checked"),
            }
        }

        if !errs.is_empty() {
            bail!(errs.iter()
                .map(|err| format!("{err}"))
                .reduce(|acc, err| format!("{acc}\n{err}"))
                .unwrap_or_default())
        }

        Ok(analysis)
    }

    fn does_name_exist(&self, name: &str, scope: Option<&Scope>) -> bool {
        self.func_decls.contains_key(name)
            || scope.map_or(false, |scope| scope.contains(name))
    }

    fn get_type_of(&self, name: &str, scope: Option<&Scope>) -> Option<Type> {
        if let Some(func_type) = self.func_decls.get(name) {
            Some(Type::Func(Box::new(func_type.clone())))
        } else {
            scope.map(|scope| scope.get_type_of(name)).unwrap_or(None)
        }
    }

    /**
     * panics if expression has not been checked
     */
    pub fn get_type_of_expr(&self, expr: &Expression) -> &Type {
        self.expr_types.get(&expr.get_hash()).unwrap()
    }

    fn check_top_level(&mut self, file: &File) -> Result<()> {
        let mut errs = vec![];

        for stmt in &file.body {
            match stmt {
                Statement::Func {
                    name,
                    return_type,
                    params,
                    body,
                } => match self.check_func(name, return_type, params, body.is_some()) {
                    Ok(()) => (),
                    Err(err) => errs.push(err),
                },
                _ => errs.push(anyhow!("only functions are allowed at top level")),
            }
        }

        if !errs.is_empty() {
            bail!(errs.iter()
                .map(|err| format!("{err}"))
                .reduce(|acc, err| format!("{acc}\n{err}"))
                .unwrap_or_default())
        }

        Ok(())
    }

    fn check_func(
        &mut self,
        name: &str,
        return_type: &Type,
        params: &[FuncParam],
        body: bool,
    ) -> Result<()> {
        let mut errs = vec![];

        let func_type = FuncType {
            return_type: return_type.clone(),
            params: params.iter()
                .map(|param| param.ty.clone())
                .collect(),
        };

        self.check_type(&Type::Func(Box::new(func_type.clone())));

        // TODO: wtf is this bruh
        if self.func_decls.contains_key(name) &&
            !(!body && *self.func_decls.get(name).unwrap() == func_type) && // this means that its a decl and the signature is the same
            !(body && *self.func_decls.get(name).unwrap() == func_type &&
                !self.func_defs.contains(name)) // this means that its a def that has only been decl before
        {
            errs.push(anyhow!("{} already exists", name));
        }

        // if !self.does_type_exist(return_type) {
        //     errs.push(anyhow!("{} is not a type", return_type));
        // }

        let mut param_names = HashSet::new();

        for param in params {
            if self.does_name_exist(&param.name, None)
                || param_names.contains(&param.name)
            {
                errs.push(anyhow!("{} already exists", param.name));
            }
            param_names.insert(param.name.clone());

            // if !self.does_type_exist(&param.ty) {
            //     errs.push(anyhow!("{} is not a type", param.ty));
            // }
        }

        if !errs.is_empty() {
            bail!(errs.iter()
                .map(|err| format!("{err}"))
                .reduce(|acc, err| format!("{acc}\n{err}"))
                .unwrap_or_default())
        }

        self.func_decls.insert(name.into(), func_type);
        if body { self.func_defs.insert(name.into()); }

        Ok(())
    }

    /**
     * this assumes that param types and names and function return type have been already checked
     */
    fn check_func_body(&mut self, func_type: FuncType, params: &[FuncParam], body: &BlockStmt) -> Result<()> {
        let return_type = &func_type.return_type;
        let params = params.iter().map(|param| &param.name).zip(func_type.params.iter());
        let mut scope = Scope::new(return_type.clone());

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
        // if self.does_type_exist(value) {
        //     bail!("{} is a type", value);
        // }

        if let Some(ty) = self.get_type_of(value, Some(scope)) {
            self.expr_types.insert(expr.get_hash(), ty);
        } else {
            bail!("{} does not exist", value);
        }

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

        self.check_type(&ty);
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

        self.check_type(&return_ty);
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

        if !errs.is_empty() {
            bail!(errs.iter()
                .map(|err| format!("{err}"))
                .reduce(|acc, err| format!("{acc}\n{err}"))
                .unwrap_or_default())
        }

        Ok(())
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
        let mut scope = scope.get_child();

        for stmt in block {
            self.check_stmt(stmt, &mut scope)?;
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

    fn check_let(&mut self, name: &str, ty: &Option<Type>, value: Option<&Expression>, scope: &mut Scope) -> Result<()> {
        // if !self.does_type_exist(ty) {
        //     bail!("{} does not exist", ty);
        // }

        if let Some(value) = value {
            self.check_expr(value, scope)?;
            let value_type = self.expr_types.get(&value.get_hash()).unwrap();

            if let Some(ty) = ty {
                self.try_coercion(ty, value_type)?;
            }
        } else if ty.is_none() {
            bail!("let statement neither has a type nor an assigned value");
        }

        if self.does_name_exist(name, Some(scope)) {
            bail!("{} already exists", name);
        }

        if let Some(ty) = ty {
            self.check_type(ty);
        } else {
            self.check_type(&self.get_type_of_expr(value.unwrap()).clone());
        }

        scope.add(name, match ty {
            Some(ty) => ty.clone(),
            None => self.get_type_of_expr(value.unwrap()).clone(),
        });

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

        self.check_block(then, scope)?;
        self.check_block(else_then, scope)?;

        Ok(())
    }

    fn check_type(&mut self, ty: &Type) {
        self.types_used.insert(ty.clone());

        match ty {
            Type::Func(ty) => {
                self.check_type(&ty.return_type);
                for param in &ty.params { self.check_type(param); }
            }
            Type::Ptr(ty) => self.check_type(ty),
            Type::Array(ty, _) => self.check_type(ty),
            Type::Slice(ty) => self.check_type(ty),
            Type::Void | Type::Primitive(_) => (),
        }
    }
}

impl<'a> Scope<'a> {
    fn new(return_type: Type) -> Self {
        Self {
            parent: None,
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

    fn add(&mut self, name: &str, ty: Type) {
        self.symbols.insert(name.into(), ty);
    }

    fn get_child(&'a self) -> Scope<'a> {
        Self {
            parent: Some(self),
            symbols: HashMap::new(),
            return_type: self.return_type.clone(),
        }
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
        analyze(b"fn main() -> i32 { let x: i32 = 1 return x }").unwrap();
    }

    #[test]
    fn undefined_variable() {
        let result = analyze(b"fn main() -> i32 { let x: i32 = y return x }");
        assert!(result.is_err());
        let err = format!("{}", result.unwrap_err());
        assert!(err.contains("does not exist"), "{err}");
    }

    #[test]
    fn duplicate_function() {
        let result = analyze(b"fn foo() -> i32 { return 1 } fn foo() -> i32 { return 2 }");
        assert!(result.is_err());
        let err = format!("{}", result.unwrap_err());
        assert!(err.contains("foo already exists"), "{err}");
    }

    #[test]
    fn argument_count_mismatch() {
        let result = analyze(b"fn foo(x: i32) -> i32 { return x } fn main() -> i32 { return foo(1, 2) }");
        assert!(result.is_err());
        let err = format!("{}", result.unwrap_err());
        assert!(err.contains("expected"), "{err}");
    }

    #[test]
    fn call_non_function() {
        let result = analyze(b"fn main() -> i32 { let x: i32 = 1 return x(1) }");
        assert!(result.is_err());
        let err = format!("{}", result.unwrap_err());
        assert!(err.contains("not a function"), "{err}");
    }
}
