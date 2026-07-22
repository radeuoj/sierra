use std::collections::{HashMap, HashSet};

use anyhow::{Result, anyhow, bail};

use crate::ast::*;

pub struct Analysis {
    types: HashMap<String, NamedType>, // named types
    expr_types: HashMap<NodeId, Type>,
    func_decls: HashMap<String, FuncType>,
    func_defs: HashSet<String>, // whether this function has a definition already
}

pub enum NamedType {
    Primitive(String),
}

#[derive(PartialEq)]
pub struct FuncType {
    pub return_type: Type,
    pub params: Vec<Type>,
}

#[derive(PartialEq)]
pub enum Type {
    Named(String),
}

impl Analysis {
    pub fn from(file: &FileAST) -> Self {
        let mut types = HashMap::new();
        types.insert("i32".into(), NamedType::Primitive("int32_t".into()));

        let analysis = Self {
            types,
            expr_types: HashMap::new(),
            func_decls: HashMap::new(),
            func_defs: HashSet::new(),
        };

        analysis
    }

    fn check_name(&self, name: &str) -> bool {
        self.types.get(name).is_some() || self.func_decls.get(name).is_some()
    }

    fn check_type(&self, name: &str) -> bool {
        self.types.get(name).is_some()
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

        if self.check_name(name) &&
            !(!body && *self.func_decls.get(name).unwrap() == func_type) && // this means that its a decl and the signature is the same
            !(body && *self.func_decls.get(name).unwrap() == func_type &&
                !self.func_defs.contains(name)) // this means that its a def that has only been decl before
        {
            errs.push(anyhow!("{} already exists", name));
        }

        if !self.check_type(return_type) {
            errs.push(anyhow!("{} is not a type", return_type));
        }

        let mut param_names = HashSet::new();

        for param in params {
            if self.check_name(&param.name) || param_names.contains(&param.name) {
                errs.push(anyhow!("{} already exists", name));
            }
            param_names.insert(param.name.clone());

            if !self.check_type(&param.ty) {
                errs.push(anyhow!("{} is not a type", param.ty));
            }
        }

        self.func_decls.insert(name.into(), func_type);
        if body { self.func_defs.insert(name.into()); }

        if !errs.is_empty() {
            bail!(errs.iter()
                .map(|err| format!("{err}"))
                .reduce(|acc, err| format!("{acc}\n{err}"))
                .unwrap_or_default())
        }

        Ok(())
    }
}
