// Generated from tko.org. Do not edit by hand.

use crate::storage::Ticket;
use std::error::Error;
use std::fmt;

pub type Result<T> = std::result::Result<T, QueryError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryError {
    message: String,
}

impl QueryError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for QueryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl Error for QueryError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Predicate {
    expr: Expr,
}

impl Predicate {
    pub fn parse(input: &str) -> Result<Self> {
        let tokens = tokenize(input)?;
        let mut parser = Parser { tokens, pos: 0 };
        let expr = parser.parse_expr()?;
        if parser.peek().is_some() {
            return Err(QueryError::new("unexpected token after predicate"));
        }
        Ok(Self { expr })
    }

    pub fn matches(&self, ticket: &Ticket) -> Result<bool> {
        self.expr.eval(ticket)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Expr {
    Compare {
        field: String,
        op: CompareOp,
        value: String,
    },
    Contains {
        field: String,
        value: String,
    },
    In {
        field: String,
        values: Vec<String>,
    },
    Presence {
        field: String,
        expected: bool,
    },
    Not(Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
}

impl Expr {
    fn eval(&self, ticket: &Ticket) -> Result<bool> {
        match self {
            Expr::Compare { field, op, value } => compare(ticket, field, *op, value),
            Expr::Contains { field, value } => Ok(plural_field(ticket, field)?.contains(value)),
            Expr::In { field, values } => Ok(values.contains(&scalar_field(ticket, field)?)),
            Expr::Presence { field, expected } => Ok(field_present(ticket, field)? == *expected),
            Expr::Not(expr) => Ok(!expr.eval(ticket)?),
            Expr::And(left, right) => Ok(left.eval(ticket)? && right.eval(ticket)?),
            Expr::Or(left, right) => Ok(left.eval(ticket)? || right.eval(ticket)?),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompareOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Word(String),
    Op(CompareOp),
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn parse_expr(&mut self) -> Result<Expr> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr> {
        let mut expr = self.parse_and()?;
        while self.consume_word("or") {
            let right = self.parse_and()?;
            expr = Expr::Or(Box::new(expr), Box::new(right));
        }
        Ok(expr)
    }

    fn parse_and(&mut self) -> Result<Expr> {
        let mut expr = self.parse_not()?;
        while self.consume_word("and") {
            let right = self.parse_not()?;
            expr = Expr::And(Box::new(expr), Box::new(right));
        }
        Ok(expr)
    }

    fn parse_not(&mut self) -> Result<Expr> {
        if self.consume_word("not") {
            Ok(Expr::Not(Box::new(self.parse_not()?)))
        } else {
            self.parse_primary()
        }
    }

    fn parse_primary(&mut self) -> Result<Expr> {
        if self.consume(&Token::LParen) {
            let expr = self.parse_expr()?;
            self.expect(Token::RParen)?;
            return Ok(expr);
        }

        if self.consume_word("has") {
            return Ok(Expr::Presence {
                field: self.expect_word()?,
                expected: true,
            });
        }

        if self.consume_word("no") {
            return Ok(Expr::Presence {
                field: self.expect_word()?,
                expected: false,
            });
        }

        let field = self.expect_word()?;
        if self.consume_word("contain") {
            return Ok(Expr::Contains {
                field,
                value: self.expect_word()?,
            });
        }
        if self.consume_word("in") {
            return Ok(Expr::In {
                field,
                values: self.parse_list()?,
            });
        }
        let op = self.expect_op()?;
        Ok(Expr::Compare {
            field,
            op,
            value: self.expect_word()?,
        })
    }

    fn parse_list(&mut self) -> Result<Vec<String>> {
        self.expect(Token::LBracket)?;
        let mut values = Vec::new();
        loop {
            values.push(self.expect_word()?);
            if self.consume(&Token::Comma) {
                continue;
            }
            self.expect(Token::RBracket)?;
            break;
        }
        Ok(values)
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn consume(&mut self, token: &Token) -> bool {
        if self.peek() == Some(token) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn consume_word(&mut self, expected: &str) -> bool {
        if matches!(self.peek(), Some(Token::Word(word)) if word == expected) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, expected: Token) -> Result<()> {
        if self.consume(&expected) {
            Ok(())
        } else {
            Err(QueryError::new(format!("expected {expected:?}")))
        }
    }

    fn expect_word(&mut self) -> Result<String> {
        match self.peek() {
            Some(Token::Word(word)) => {
                let word = word.clone();
                self.pos += 1;
                Ok(word)
            }
            _ => Err(QueryError::new("expected bare token")),
        }
    }

    fn expect_op(&mut self) -> Result<CompareOp> {
        match self.peek() {
            Some(Token::Op(op)) => {
                let op = *op;
                self.pos += 1;
                Ok(op)
            }
            _ => Err(QueryError::new("expected comparison operator")),
        }
    }
}

fn tokenize(input: &str) -> Result<Vec<Token>> {
    let mut chars = input.chars().peekable();
    let mut tokens = Vec::new();

    while let Some(ch) = chars.peek().copied() {
        match ch {
            c if c.is_whitespace() => {
                chars.next();
            }
            '(' => {
                chars.next();
                tokens.push(Token::LParen);
            }
            ')' => {
                chars.next();
                tokens.push(Token::RParen);
            }
            '[' => {
                chars.next();
                tokens.push(Token::LBracket);
            }
            ']' => {
                chars.next();
                tokens.push(Token::RBracket);
            }
            ',' => {
                chars.next();
                tokens.push(Token::Comma);
            }
            '=' => {
                chars.next();
                tokens.push(Token::Op(CompareOp::Eq));
            }
            '!' => {
                chars.next();
                if chars.next() == Some('=') {
                    tokens.push(Token::Op(CompareOp::Ne));
                } else {
                    return Err(QueryError::new("expected != operator"));
                }
            }
            '<' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::Op(CompareOp::Le));
                } else {
                    tokens.push(Token::Op(CompareOp::Lt));
                }
            }
            '>' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::Op(CompareOp::Ge));
                } else {
                    tokens.push(Token::Op(CompareOp::Gt));
                }
            }
            _ => {
                let mut word = String::new();
                while let Some(next) = chars.peek().copied() {
                    if next.is_whitespace() || "()[],=!<>".contains(next) {
                        break;
                    }
                    word.push(next);
                    chars.next();
                }
                if word.is_empty() {
                    return Err(QueryError::new("invalid token"));
                }
                tokens.push(Token::Word(word));
            }
        }
    }

    if tokens.is_empty() {
        return Err(QueryError::new("empty predicate"));
    }

    Ok(tokens)
}

fn compare(ticket: &Ticket, field: &str, op: CompareOp, value: &str) -> Result<bool> {
    if field == "priority" {
        let rhs: u8 = value
            .parse()
            .map_err(|_| QueryError::new(format!("priority value is not numeric: {value}")))?;
        return Ok(compare_order(ticket.properties.priority, op, rhs));
    }

    let lhs = scalar_field(ticket, field)?;
    Ok(match op {
        CompareOp::Eq => lhs == value,
        CompareOp::Ne => lhs != value,
        CompareOp::Lt => lhs.as_str() < value,
        CompareOp::Le => lhs.as_str() <= value,
        CompareOp::Gt => lhs.as_str() > value,
        CompareOp::Ge => lhs.as_str() >= value,
    })
}

fn compare_order<T: Ord>(lhs: T, op: CompareOp, rhs: T) -> bool {
    match op {
        CompareOp::Eq => lhs == rhs,
        CompareOp::Ne => lhs != rhs,
        CompareOp::Lt => lhs < rhs,
        CompareOp::Le => lhs <= rhs,
        CompareOp::Gt => lhs > rhs,
        CompareOp::Ge => lhs >= rhs,
    }
}

fn scalar_field(ticket: &Ticket, field: &str) -> Result<String> {
    match field {
        "id" => Ok(ticket.id.clone()),
        "status" => Ok(ticket.properties.status.clone()),
        "type" => Ok(ticket.properties.ticket_type.clone()),
        "assignee" => Ok(ticket.properties.assignee.clone().unwrap_or_default()),
        "external-ref" => Ok(ticket.properties.external_ref.clone().unwrap_or_default()),
        "parent" => Ok(ticket.properties.parent.clone().unwrap_or_default()),
        "created" => Ok(ticket.properties.created.clone().unwrap_or_default()),
        "title" => Ok(ticket.title.clone()),
        "priority" => Ok(ticket.properties.priority.to_string()),
        _ => Err(QueryError::new(format!("unknown scalar field: {field}"))),
    }
}

fn plural_field<'a>(ticket: &'a Ticket, field: &str) -> Result<&'a [String]> {
    match field {
        "deps" => Ok(&ticket.properties.deps),
        "links" => Ok(&ticket.properties.links),
        "tags" => Ok(&ticket.properties.tags),
        _ => Err(QueryError::new(format!("unknown plural field: {field}"))),
    }
}

fn field_present(ticket: &Ticket, field: &str) -> Result<bool> {
    if matches!(field, "deps" | "links" | "tags") {
        return Ok(!plural_field(ticket, field)?.is_empty());
    }
    Ok(!scalar_field(ticket, field)?.is_empty())
}
