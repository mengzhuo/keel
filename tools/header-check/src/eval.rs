// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

//! Tokenizer and integer-expression parser shared by the C and Rust sides.
//!
//! This is a direct port of the evaluator that used to live in
//! `tests/header_rust_consistency.py`: it understands the subset of C
//! constant expressions the headers use (integer literals with `u`/`l`
//! suffixes, identifiers, parentheses and the arithmetic and bitwise
//! operators) with C's truncating division and Python's floor modulo.

/// A tokenized piece of an integer constant expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Num(i128),
    Ident(String),
    Op(&'static str),
}

/// Why an expression could not be evaluated; the payload names the reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bad(pub &'static str);

pub type Eval = Result<i128, Bad>;

const TWO_CHAR_OPS: [&str; 8] = ["<<", ">>", "<=", ">=", "==", "!=", "&&", "||"];
const ONE_CHAR_OPS: [&str; 14] = [
    "-", "+", "*", "/", "%", "~", "!", "&", "|", "^", "(", ")", "<", ">",
];

/// Split a C constant expression into tokens.  Bytes that do not start a
/// token (spaces, casts, commas, ...) are skipped, matching the Python
/// `re.findall` behaviour this replaced.
pub fn tokenize(expr: &str) -> Vec<Token> {
    let bytes = expr.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_digit() {
            let start = i;
            if c == b'0'
                && i + 1 < bytes.len()
                && (bytes[i + 1] == b'x' || bytes[i + 1] == b'X')
                && i + 2 < bytes.len()
                && bytes[i + 2].is_ascii_hexdigit()
            {
                i += 2;
                while i < bytes.len() && bytes[i].is_ascii_hexdigit() {
                    i += 1;
                }
            } else if c == b'0' && i + 1 < bytes.len() && (b'0'..=b'7').contains(&bytes[i + 1]) {
                i += 1;
                while i < bytes.len() && (b'0'..=b'7').contains(&bytes[i]) {
                    i += 1;
                }
            } else {
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let digits = &expr[start..i];
            while i < bytes.len() && matches!(bytes[i], b'u' | b'U' | b'l' | b'L') {
                i += 1;
            }
            tokens.push(Token::Num(parse_int(digits)));
            continue;
        }
        if c.is_ascii_alphabetic() || c == b'_' {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            tokens.push(Token::Ident(expr[start..i].to_string()));
            continue;
        }
        if let Some(op) = TWO_CHAR_OPS
            .iter()
            .find(|op| bytes[i..].starts_with(op.as_bytes()))
        {
            tokens.push(Token::Op(op));
            i += 2;
            continue;
        }
        if let Some(op) = ONE_CHAR_OPS
            .iter()
            .find(|op| bytes[i..].starts_with(op.as_bytes()))
        {
            tokens.push(Token::Op(op));
            i += 1;
            continue;
        }
        i += 1;
    }
    tokens
}

fn parse_int(text: &str) -> i128 {
    let text = text.trim_end_matches(['u', 'U', 'l', 'L']);
    if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        i128::from_str_radix(hex, 16).expect("hex literal")
    } else if text.len() > 1
        && text.starts_with('0')
        && text[1..].bytes().all(|b| (b'0'..=b'7').contains(&b))
    {
        i128::from_str_radix(&text[1..], 8).expect("octal literal")
    } else {
        text.parse().expect("decimal literal")
    }
}

/// Recursive-descent parser for the C operator precedence table.
pub struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Parse the whole expression, resolving identifiers through `resolve`.
    pub fn parse(&mut self, resolve: &mut dyn FnMut(&str) -> Eval) -> Eval {
        let value = self.or(resolve)?;
        if self.pos != self.tokens.len() {
            return Err(Bad("trailing"));
        }
        Ok(value)
    }

    fn peek(&self) -> Option<&'a Token> {
        self.tokens.get(self.pos)
    }

    fn take(&mut self) -> Option<&'a Token> {
        let token = self.tokens.get(self.pos);
        self.pos += 1;
        token
    }

    fn eat(&mut self, op: &str) -> bool {
        if matches!(self.peek(), Some(Token::Op(found)) if *found == op) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn or(&mut self, resolve: &mut dyn FnMut(&str) -> Eval) -> Eval {
        let mut value = self.xor(resolve)?;
        while self.eat("|") {
            value |= self.xor(resolve)?;
        }
        Ok(value)
    }

    fn xor(&mut self, resolve: &mut dyn FnMut(&str) -> Eval) -> Eval {
        let mut value = self.and(resolve)?;
        while self.eat("^") {
            value ^= self.and(resolve)?;
        }
        Ok(value)
    }

    fn and(&mut self, resolve: &mut dyn FnMut(&str) -> Eval) -> Eval {
        let mut value = self.shift(resolve)?;
        while self.eat("&") {
            value &= self.shift(resolve)?;
        }
        Ok(value)
    }

    fn shift(&mut self, resolve: &mut dyn FnMut(&str) -> Eval) -> Eval {
        let mut value = self.add(resolve)?;
        loop {
            if self.eat("<<") {
                value = checked_shl(value, self.add(resolve)?)?;
            } else if self.eat(">>") {
                value = checked_shr(value, self.add(resolve)?)?;
            } else {
                return Ok(value);
            }
        }
    }

    fn add(&mut self, resolve: &mut dyn FnMut(&str) -> Eval) -> Eval {
        let mut value = self.mul(resolve)?;
        loop {
            if self.eat("+") {
                value = value.wrapping_add(self.mul(resolve)?);
            } else if self.eat("-") {
                value = value.wrapping_sub(self.mul(resolve)?);
            } else {
                return Ok(value);
            }
        }
    }

    fn mul(&mut self, resolve: &mut dyn FnMut(&str) -> Eval) -> Eval {
        let mut value = self.unary(resolve)?;
        loop {
            if self.eat("*") {
                value = value.wrapping_mul(self.unary(resolve)?);
            } else if self.eat("/") {
                let divisor = self.unary(resolve)?;
                if divisor == 0 {
                    return Err(Bad("div0"));
                }
                value /= divisor;
            } else if self.eat("%") {
                let divisor = self.unary(resolve)?;
                if divisor == 0 {
                    return Err(Bad("mod0"));
                }
                value = floor_mod(value, divisor);
            } else {
                return Ok(value);
            }
        }
    }

    fn unary(&mut self, resolve: &mut dyn FnMut(&str) -> Eval) -> Eval {
        if self.eat("+") {
            return self.unary(resolve);
        }
        if self.eat("-") {
            return Ok(self.unary(resolve)?.wrapping_neg());
        }
        if self.eat("~") {
            return Ok(!self.unary(resolve)?);
        }
        if self.eat("!") {
            return Ok(if self.unary(resolve)? == 0 { 1 } else { 0 });
        }
        self.atom(resolve)
    }

    fn atom(&mut self, resolve: &mut dyn FnMut(&str) -> Eval) -> Eval {
        if self.eat("(") {
            let value = self.or(resolve)?;
            if !self.eat(")") {
                return Err(Bad("paren"));
            }
            return Ok(value);
        }
        match self.take() {
            None => Err(Bad("eof")),
            Some(Token::Num(value)) => Ok(*value),
            Some(Token::Ident(name)) => resolve(name),
            Some(Token::Op(_)) => Err(Bad("token")),
        }
    }
}

fn checked_shl(value: i128, amount: i128) -> Eval {
    if !(0..128).contains(&amount) {
        return Err(Bad("shift"));
    }
    Ok(value.wrapping_shl(amount as u32))
}

fn checked_shr(value: i128, amount: i128) -> Eval {
    if !(0..128).contains(&amount) {
        return Err(Bad("shift"));
    }
    Ok(value.wrapping_shr(amount as u32))
}

/// Python's `%` (the sign follows the divisor), unlike Rust's truncating `%`.
fn floor_mod(value: i128, divisor: i128) -> i128 {
    let rem = value % divisor;
    if rem != 0 && (rem < 0) != (divisor < 0) {
        rem + divisor
    } else {
        rem
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(expr: &str) -> i128 {
        let tokens = tokenize(expr);
        let mut parser = Parser::new(&tokens);
        parser
            .parse(&mut |name| {
                Ok(match name {
                    "X" => 6,
                    _ => return Err(Bad("missing")),
                })
            })
            .unwrap()
    }

    #[test]
    fn literals_and_suffixes() {
        assert_eq!(eval("0x10UL"), 16);
        assert_eq!(eval("010"), 8);
        assert_eq!(eval("42l"), 42);
    }

    #[test]
    fn precedence_and_operators() {
        assert_eq!(eval("1 | 2 ^ 3 & 4"), 3);
        assert_eq!(eval("1 + 2 * 3 << 1"), 14);
        assert_eq!(eval("(X << 2) - 4"), 20);
        assert_eq!(eval("~0U >> 1"), -1);
    }

    #[test]
    fn division_and_modulo() {
        assert_eq!(eval("-7 / 2"), -3);
        assert_eq!(eval("-7 % 2"), 1);
        assert_eq!(eval("7 % -2"), -1);
    }

    #[test]
    fn unknown_identifiers_are_errors() {
        let tokens = tokenize("NOPE + 1");
        let mut parser = Parser::new(&tokens);
        assert_eq!(
            parser.parse(&mut |_| Err(Bad("missing"))),
            Err(Bad("missing"))
        );
    }
}
