use anyhow::Result;
use anyhow::bail;

use crate::lexer::*;
use crate::token::*;
use crate::ast::*;

pub struct Parser {
    lexer: Lexer,
    peek_token: Token,
}

#[derive(PartialEq, PartialOrd)]
pub enum BindingPower {
    Lowest,
    Assign,
    Equals,
    Sum,
    Product,
    Unary,
    Call,
}

impl BindingPower {
    fn get(token: &Token) -> BindingPower {
        use BindingPower::*;

        match token {
            Token::Equal | Token::NotEqual | Token::Lt | Token::Gt
            | Token::Lte | Token::Gte => Equals,
            Token::Plus | Token::Minus => Sum,
            Token::Asterisk | Token::Slash => Product,
            Token::Assign => Assign,
            Token::LParen => Call,
            _ => Lowest,
        }
    }
}

impl Parser {
    pub fn new(mut lexer: Lexer) -> Result<Self> {
        Ok(Self {
            peek_token: lexer.next_token()?,
            lexer,
        })
    }

    fn next_token(&mut self) -> Result<Token> {
        Ok(std::mem::replace(&mut self.peek_token, self.lexer.next_token()?))
    }

    fn expect_peek(&mut self, token: &Token) -> Result<()> {
        if self.peek_token == *token {
            self.next_token()?;
            Ok(())
        } else {
            anyhow::bail!("expected {} but got {}", token, self.peek_token)
        }
    }

    fn expect_ident(&mut self) -> Result<String> {
        match self.next_token()? {
            Token::Ident(value) => Ok(value),
            token => anyhow::bail!("expected identifier but got {}", token)
        }
    }

    fn get_peek_binding_power(&self) -> BindingPower {
        BindingPower::get(&self.peek_token)
    }

    fn parse_expression(&mut self, bpow: BindingPower) -> Result<Expression> {
        let mut left = match self.next_token()? {
            Token::Ident(name) => self.parse_ident(name),
            Token::Int(lit) => self.parse_int(lit),
            Token::String(lit) => self.parse_string(lit),
            op @ (Token::Minus | Token::Bang) => self.parse_unary_expression(op)?,
            token => bail!("invalid prefix operator {}", token),
        };

        while bpow < self.get_peek_binding_power() {
            left = match self.peek_token {
                Token::Equal | Token::NotEqual | Token::Lt | Token::Lte
                | Token::Gt | Token::Gte | Token::Plus | Token::Minus
                | Token::Asterisk | Token::Slash
                | Token::Assign => self.parse_binary_expression(left)?,
                Token::LParen => self.parse_call_expression(left)?,
                _ => return Ok(left),
            }
        }

        Ok(left)
    }

    fn parse_ident(&mut self, name: String) -> Expression {
        Expression::Ident { value: name }
    }

    fn parse_int(&mut self, lit: String) -> Expression {
        Expression::Int { value: lit }
    }

    fn parse_string(&mut self, lit: String) -> Expression {
        Expression::String { value: lit }
    }

    fn parse_unary_expression(&mut self, op: Token) -> Result<Expression> {
        let right = self.parse_expression(BindingPower::Unary)?;
        Ok(Expression::Unary { op, right: right.into() })
    }

    fn parse_binary_expression(&mut self, left: Expression) -> Result<Expression> {
        let op = self.next_token()?;
        let bpow = BindingPower::get(&op);
        let right = self.parse_expression(bpow)?;

        Ok(Expression::Binary { op, left: left.into(), right: right.into() })
    }

    fn parse_call_expression(&mut self, left: Expression) -> Result<Expression> {
        let args = self.parse_call_arguments()?;
        Ok(Expression::Call { func: left.into(), args })
    }

    fn parse_call_arguments(&mut self) -> Result<Vec<Expression>> {
        self.next_token()?; // (
        let mut args = vec![];

        if self.peek_token == Token::RParen {
            self.next_token()?;
            return Ok(args);
        }

        loop {
            args.push(self.parse_expression(BindingPower::Lowest)?);
            if self.peek_token != Token::Comma { break }
            self.next_token()?;
        }

        self.expect_peek(&Token::RParen)?;

        Ok(args)
    }

    fn parse_statement(&mut self) -> Result<Statement> {
        match self.peek_token {
            Token::Let => self.parse_let_statement(),
            Token::Return => self.parse_return_statement(),
            Token::If => self.parse_if_statement(),
            Token::Fn => self.parse_func_statement(),
            _ => self.parse_expr_statement(),
        }
    }

    fn parse_let_statement(&mut self) -> Result<Statement> {
        self.next_token()?; // let
        let name = self.expect_ident()?;

        self.expect_peek(&Token::Colon)?;
        let ty = self.expect_ident()?;

        let value = match self.peek_token {
            Token::Assign => {
                self.next_token()?; // =
                Some(self.parse_expression(BindingPower::Lowest)?)
            }
            _ => None,
        };

        Ok(Statement::Let { name, ty, value })
    }

    fn parse_return_statement(&mut self) -> Result<Statement> {
        self.next_token()?; // return
        let value = self.parse_expression(BindingPower::Lowest)?;
        Ok(Statement::Return { value })
    }

    fn parse_if_statement(&mut self) -> Result<Statement> {
        self.next_token()?; // if
        let cond = self.parse_expression(BindingPower::Lowest)?;
        let then = self.parse_block_statement()?;

        let else_then = if self.peek_token == Token::Else {
            self.next_token()?;
            self.parse_block_statement()?
        } else {
            vec![]
        };

        Ok(Statement::If { cond, then, else_then })
    }

    fn parse_block_statement(&mut self) -> Result<BlockStmt> {
        self.expect_peek(&Token::LBrace)?;
        let mut body = vec![];

        while ![Token::Eof, Token::RBrace].contains(&self.peek_token) {
             body.push(self.parse_statement()?);
        }

        self.expect_peek(&Token::RBrace)?;

        Ok(body)
    }

    fn parse_func_statement(&mut self) -> Result<Statement> {
        self.next_token()?; // fn
        let name = self.expect_ident()?;

        let params = self.parse_func_params()?;

        self.expect_peek(&Token::Arrow)?;
        let return_type = self.expect_ident()?;

        let body = if self.peek_token == Token::LBrace {
            Some(self.parse_block_statement()?)
        } else {
            None
        };

        Ok(Statement::Func { name, return_type, params, body })
    }

    fn parse_func_params(&mut self) -> Result<Vec<FuncParam>> {
        self.expect_peek(&Token::LParen)?;
        let mut params = vec![];

        if self.peek_token == Token::RParen {
            self.next_token()?;
            return Ok(params);
        }

        loop {
            params.push(self.parse_func_param()?);
            if self.peek_token != Token::Comma { break; }
            self.next_token()?;
        }

        self.expect_peek(&Token::RParen)?;

        Ok(params)
    }

    fn parse_func_param(&mut self) -> Result<FuncParam> {
        let name = self.expect_ident()?;

        self.expect_peek(&Token::Colon)?;
        let ty = self.expect_ident()?;

        Ok(FuncParam { name, ty })
    }

    fn parse_expr_statement(&mut self) -> Result<Statement> {
        let value = self.parse_expression(BindingPower::Lowest)?;
        Ok(Statement::Expr { value })
    }

    pub fn parse_file(mut self) -> Result<File> {
        let mut body = vec![];

        while self.peek_token != Token::Eof {
            body.push(self.parse_statement()?);
        }

        Ok(File { body })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expect_peek() -> Result<()> {
        let input = b"let a: int32 = 10;";
        let lexer = Lexer::new(input.to_vec());
        let mut parser = Parser::new(lexer)?;

        use Token::*;
        parser.expect_peek(&Let)?;
        parser.expect_peek(&Ident("a".to_string()))?;
        parser.expect_peek(&Colon)?;
        parser.expect_peek(&Ident("int32".to_string()))?;
        parser.expect_peek(&Assign)?;
        parser.expect_peek(&Int("10".to_string()))?;
        parser.expect_peek(&Semicolon)?;
        parser.expect_peek(&Eof)?;

        Ok(())
    }

    #[test]
    fn test_parse_program() -> Result<()> {
        let input = br#"
            fn main() -> i32 {
                printf("salutare")
                let a: i32 = 1 + 1
                let b: bool = !false
            }
        "#;

        let lexer = Lexer::new(input.to_vec());
        let parser = Parser::new(lexer)?;
        let program = parser.parse_file()?;

        println!("{:#?}", program);

        // assert_eq!(program, Program {
        //     body: vec![
        //         Statement::Func {
        //             name: "main",
        //             return_ty: "i32", params: (), body: () }
        //     ],
        // });

        Ok(())
    }
}
