//! 从文件中解析出来token

use logos::Logos;
use rust_decimal::Decimal;
use std::str::FromStr;

#[derive(Logos, Debug, Eq, PartialEq, Clone)]
pub enum Token<'a> {
    #[regex("@[a-zA-Z_][a-zA-Z0-9_]*", |lex| &lex.slice()[1..])]
    State(&'a str),
    #[token("(")]
    LeftParen,
    #[token(")")]
    RightParen,
    #[token("{")]
    LeftBrace,
    #[token("}")]
    RightBrace,
    #[token("[")]
    LeftBracket,
    #[token("]")]
    RightBracket,
    #[token("+")]
    Add,
    #[token("++")]
    PlusOne,
    #[token("-")]
    Sub,
    #[token("--")]
    MinusOne,
    #[token("*")]
    Mul,
    #[token("/")]
    Div,
    #[token("%")]
    Mod,
    #[token("^")]
    Pow,
    #[token("=")]
    Assign,
    #[token("+=")]
    AddAssign,
    #[token("-=")]
    SubAssign,
    #[token("*=")]
    MulAssign,
    #[token("/=")]
    DivAssign,
    #[token("%=")]
    ModAssign,
    #[token("^=")]
    PowAssign,
    #[token("==")]
    Equal,
    #[token("!=")]
    NotEqual,
    #[token(">")]
    Greater,
    #[token("<")]
    Less,
    #[token(">=")]
    GreaterEqual,
    #[token("<=")]
    LessEqual,
    #[token("&&")]
    #[token("&")]
    #[token("and")]
    And,
    #[token("||")]
    #[token("|")]
    #[token("or")]
    Or,
    #[token("!")]
    Not,
    #[token("?")]
    Question,
    #[token(":")]
    Colon,
    #[token("if")]
    If,
    #[token("elif")]
    Elif,
    #[token("else")]
    Else,
    #[token("while")]
    While,
    #[token("for")]
    For,
    #[token("pub")]
    Pub,
    #[token("fn")]
    Fn,
    #[token("Query")]
    Query,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token("\n")]
    Line,
    #[regex(r#""[^"]*""# , |lex|{
        let slice=lex.slice();
        &slice[0..slice.len()-1]
    })]
    #[regex(r#"'[^']*'"# , |lex|{
        let slice=lex.slice();
        &slice[1..slice.len()-1]
    })]
    Str(&'a str),
    #[regex(r"[0-9]*\.?[0-9]+([eE][-+]?[0-9]+)?", |lex| Decimal::from_str(lex.slice()).unwrap())]
    Number(Decimal),
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice())]
    Ident(&'a str),
    #[regex(r"#[^\n]*", logos::skip)]
    Comment,
    #[token("    ")]
    #[token("\t")]
    Tab,
    #[regex(r" ", logos::skip)]
    Whitespace,
}

#[cfg(test)]
mod tests {
    use logos::Logos;

    use super::Token;

    #[test]
    fn token_hello_world() {
        let lex = r#"
            print('Hello,World!')
        "#;
        let tokens = Token::lexer(lex).collect::<Vec<_>>();
        for token in tokens {
            println!("{:?}", token);
        }
    }
}
