use std::collections::{HashMap, HashSet};

use anyhow::{Result, anyhow, bail};

use crate::{ast::*, token::Token};

#[derive(Debug)]
pub struct Analysis {
    types: HashMap<String, NamedType>, // named types
    expr_types: HashMap<NodeId, Type>,
    func_decls: HashMap<String, FuncType>,
    func_defs: HashSet<String>, // whether this function has a definition already
}

#[derive(Debug)]
pub enum NamedType {
    Primitive(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FuncType {
    pub return_type: Type,
    pub params: Vec<Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Named(String),
    Func(Box<FuncType>),
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
    pub fn from(file: &FileAST) -> Result<Self> {
        let mut types = HashMap::new();
        types.insert("i32".into(), NamedType::Primitive("int32_t".into()));

        let mut analysis = Self {
            types,
            expr_types: HashMap::new(),
            func_decls: HashMap::new(),
            func_defs: HashSet::new(),
        };

        analysis.check_top_level(file)?;
        let mut errs = vec![];

        for stmt in &file.body {
            let stmt = &file.statements[*stmt];

            match stmt {
                Statement::Func {
                    name,
                    params,
                    body,
                    ..
                } => if let Some(body) = body && let Err(err)
                        = analysis.check_func_body(file, analysis.func_decls.get(name).unwrap().clone(), params, body)
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

    fn does_type_exist(&self, name: &str) -> bool {
        self.types.contains_key(name)
    }

    fn get_type_of(&self, name: &str, scope: Option<&Scope>) -> Option<Type> {
        if let Some(func_type) = self.func_decls.get(name) {
            Some(Type::Func(Box::new(func_type.clone())))
        } else {
            scope.map(|scope| scope.get_type_of(name)).unwrap_or(None)
        }
    }

    fn check_top_level(&mut self, file: &FileAST) -> Result<()> {
        let mut errs = vec![];

        for stmt in &file.body {
            let stmt = &file.statements[*stmt];

            match stmt {
                Statement::Func {
                    name,
                    return_ty,
                    params,
                    body,
                } => match self.check_func(name, return_ty, params, body.is_some()) {
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
        return_type: &str,
        params: &[FuncParam],
        body: bool,
    ) -> Result<()> {
        let mut errs = vec![];

        let func_type = FuncType {
            return_type: Type::Named(return_type.into()),
            params: params.iter()
                .map(|param| Type::Named(param.ty.clone()))
                .collect(),
        };

        // TODO: wtf is this bruh
        if self.types.contains_key(name) || (self.func_decls.contains_key(name) &&
            !(!body && *self.func_decls.get(name).unwrap() == func_type) && // this means that its a decl and the signature is the same
            !(body && *self.func_decls.get(name).unwrap() == func_type &&
                !self.func_defs.contains(name))) // this means that its a def that has only been decl before
        {
            errs.push(anyhow!("{} already exists", name));
        }

        if !self.does_type_exist(return_type) {
            errs.push(anyhow!("{} is not a type", return_type));
        }

        let mut param_names = HashSet::new();

        for param in params {
            if self.does_type_exist(&param.name)
                || self.does_name_exist(&param.name, None)
                || param_names.contains(&param.name)
            {
                errs.push(anyhow!("{} already exists", param.name));
            }
            param_names.insert(param.name.clone());

            if !self.does_type_exist(&param.ty) {
                errs.push(anyhow!("{} is not a type", param.ty));
            }
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
    fn check_func_body(&mut self, file: &FileAST, func_type: FuncType, params: &[FuncParam], body: &BlockStmt) -> Result<()> {
        let return_type = &func_type.return_type;
        let params = params.iter().map(|param| &param.name).zip(func_type.params.iter());
        let mut scope = Scope::new(return_type.clone());

        for (param, ty) in params {
            scope.add(param, ty.clone());
        }

        self.check_block(file, body, &mut scope)?;

        Ok(())
    }

    fn check_expr(&mut self, file: &FileAST, id: NodeId, scope: &Scope) -> Result<()> {
        let expr = &file.expressions[id];

        match expr {
            Expression::Ident { value } => self.check_ident(id, value, scope),
            Expression::Int { .. } => self.check_int(id),
            Expression::String { .. } => todo!("we don't support strings atm"),
            Expression::Unary { op, right } => self.check_unary(file, id, op, *right, scope),
            Expression::Binary { op, left, right } => self.check_binary(file, id, op, *left, *right, scope),
            Expression::Call { func, args } => self.check_call(file, id, *func, args, scope),
        }
    }

    fn check_ident(&mut self, id: NodeId, value: &str, scope: &Scope) -> Result<()> {
        if self.does_type_exist(value) {
            bail!("{} is a type", value);
        }

        if let Some(ty) = self.get_type_of(value, Some(scope)) {
            self.expr_types.insert(id, ty);
        } else {
            bail!("{} does not exist", value);
        }

        Ok(())
    }

    fn check_int(&mut self, id: NodeId) -> Result<()> {
        self.expr_types.insert(id, Type::Named("i32".into()));

        Ok(())
    }

    fn check_unary(&mut self, file: &FileAST, id: NodeId, _op: &Token, right: NodeId, scope: &Scope) -> Result<()> {
        // if let Some(ty) = self.expr_types.get(&right) && Type::
        // here you would have to check if right is a primitive but im too lazy to do it

        self.check_expr(file, right, scope)?;

        self.expr_types.insert(id, self.expr_types.get(&right).unwrap().clone());

        Ok(())
    }

    fn check_binary(&mut self, file: &FileAST, id: NodeId, _op: &Token, left: NodeId, right: NodeId, scope: &Scope) -> Result<()> {
        // the same as unary you have to check if left and right are primitives
        // and also the result should be the highest of them too on a priority list
        // something like i32 < i64 < f32 < f64
        // this has to be checked with the c std as im unaware right now

        self.check_expr(file, left, scope)?;
        self.check_expr(file, right, scope)?;

        self.expr_types.insert(id, self.expr_types.get(&right).unwrap().clone());

        Ok(())
    }

    fn check_call(&mut self, file: &FileAST, id: NodeId, func: NodeId, args: &[NodeId], scope: &Scope) -> Result<()> {
        self.check_expr(file, func, scope)?;

        let Type::Func(ty) = self.expr_types.get(&func).unwrap() else {
            bail!("left of call expr is not a function");
        };

        let ty = *ty.clone();

        if args.len() != ty.params.len() {
            bail!("expected {} args but got {}", ty.params.len(), args.len());
        }

        let mut errs = vec![];
        for (i, (arg, ty)) in args.iter().zip(ty.params.iter()).enumerate() {
            if let Err(err) = self.check_expr(file, *arg, scope) {
                errs.push(err);
            }

            if self.expr_types.get(&arg) != Some(ty) {
                errs.push(anyhow!("arg {} of func does not match type of param", i));
            }
        }

        self.expr_types.insert(id, ty.return_type.clone());

        if !errs.is_empty() {
            bail!(errs.iter()
                .map(|err| format!("{err}"))
                .reduce(|acc, err| format!("{acc}\n{err}"))
                .unwrap_or_default())
        }

        Ok(())
    }

    fn check_block(&mut self, file: &FileAST, block: &BlockStmt, scope: &mut Scope) -> Result<()> {
        let mut errs = vec![];
        let mut scope = scope.get_child();

        for stmt in block {
            if let Err(err) = self.check_stmt(file, *stmt, &mut scope) {
                errs.push(err);
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

    fn check_stmt(&mut self, file: &FileAST, id: NodeId, scope: &mut Scope) -> Result<()> {
        let stmt = &file.statements[id];

        match stmt {
            Statement::Let { name, ty, value } => self.check_let(file, name, ty, *value, scope),
            Statement::Return { value } => self.check_return(file, *value, scope),
            Statement::If { cond, then, else_then } => self.check_if(file, *cond, then, else_then, scope),
            Statement::Func { .. } => bail!("funcs are only allowed at top level"),
            Statement::Expr { value } => self.check_expr(file, *value, scope),
        }
    }

    fn check_let(&mut self, file: &FileAST, name: &str, ty: &str, value: Option<NodeId>, scope: &mut Scope) -> Result<()> {
        if !self.does_type_exist(ty) {
            bail!("{} does not exist", ty);
        }

        let ty = Type::Named(ty.into());

        if let Some(value) = value {
            self.check_expr(file, value, scope)?;
            let value_type = self.expr_types.get(&value).unwrap();

            if *value_type != ty {
                bail!("expected expr of type {:?} but got {:?}", ty, value_type);
            }
        }

        if self.does_name_exist(name, Some(scope)) || self.does_type_exist(name) {
            bail!("{} already exists", name);
        }

        scope.add(name, ty.clone());

        Ok(())
    }

    fn check_return(&mut self, file: &FileAST, value: NodeId, scope: &mut Scope) -> Result<()> {
        self.check_expr(file, value, scope)?;
        let value_type = self.expr_types.get(&value).unwrap();

        if *value_type != scope.return_type {
            bail!("return expected type {:?} but instead found type {:?}", scope.return_type, value_type);
        }

        Ok(())
    }

    #[allow(unused_variables)]
    fn check_if(&mut self, file: &FileAST, cond: NodeId, then: &BlockStmt, else_then: &BlockStmt, scope: &mut Scope) -> Result<()> {
        self.check_expr(file, cond, scope)?;
        let cond_type = self.expr_types.get(&cond).unwrap();
        // check if cond_type is primitive

        self.check_block(file, then, scope)?;
        self.check_block(file, else_then, scope)?;

        Ok(())
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
        Analysis::from(&file)?;
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
