//! 解析表达式

use std::cell::Cell;

use chumsky::{input::ValueInput, prelude::*, Parser};
use rust_decimal::Decimal;

use super::tokenizer::Token;
/// 一元运算符
#[derive(Debug, Clone)]
pub enum UnaryOp {
    // 正负号
    Plus,
    Minus,
    // 逻辑非
    Not,
    // ?运算
    Question,
}

/// 二元运算符
#[derive(Debug, Clone)]
pub enum BinaryOp {
    // 基本运算符
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Mod,

    // 逻辑运算
    Equal,
    NotEqual,
    Greater,
    Less,
    GreaterEqual,
    LessEqual,
    And,
    Or,
    Not,

    // 赋值运算符
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    ModAssign,
    PowAssign,

    // 访问运算符
    // 通过索引拿到对象
    Index,
    // 通过key拿到对象
    Key,
    // 拿出所有组件
    Dot,
    // 调用运算符
    Call,
}

/// 对象
#[derive(Debug, Clone)]
pub enum Object<'a> {
    /// 常量
    Constant(Decimal),
    /// 字符串
    Str(&'a str),
    /// 变量(标识符，变量可以是函数名，也可以是变量名)
    Variable(&'a str),
    /// 元组
    Tuple(Vec<Expression<'a>>),
    /// 数组
    Array(Vec<Expression<'a>>),
    /// 字典项(TODO: 未实现)
    DictItem(Box<Expression<'a>>, Box<Expression<'a>>),
    /// 字典(TODO: 未实现)
    Dict(Vec<Expression<'a>>),
}
/// 表达式
#[derive(Debug, Clone)]
pub enum Expression<'a> {
    /// 单独一个对象
    Object(Object<'a>),
    /// 运算
    Unary {
        op: UnaryOp,
        hs: Box<Expression<'a>>,
    },
    /// 双值运算
    Binary {
        op: BinaryOp,
        lhs: Box<Expression<'a>>,
        rhs: Box<Expression<'a>>,
    },
    /// 优先运算符
    Priority(Box<Expression<'a>>),
    /// 查询运算符
    Query {
        with_compoents: Vec<&'a str>,
        without_compoents: Vec<&'a str>,
    },
}

/// 分支
#[derive(Debug, Clone)]
pub struct Branch<'a> {
    /// 条件
    pub condition: Expression<'a>,
    /// 指令
    pub commands: Vec<Command<'a>>,
}

/// 指令
#[derive(Debug, Clone)]
pub enum Command<'a> {
    /// 表达式
    Expression(Expression<'a>),
    /// 条件表达式
    If {
        if_branch: Vec<Branch<'a>>,
        else_branch: Option<Vec<Command<'a>>>,
    },
    /// 循环表达式
    While {
        condition: Box<Expression<'a>>,
        command: Vec<Command<'a>>,
    },
    /// 函数定义
    Function {
        name: &'a str,
        args: Vec<&'a str>,
        commands: Vec<Command<'a>>,
    },
    /// 占位行
    NewLine,
}

/// 方便进行indent增加
macro add_indent($indent_count:expr) {
    |s| {
        $indent_count.set($indent_count.get() + 1);
        s
    }
}

/// 方便进行indent减少
macro sub_indent($indent_count:expr) {
    $indent_count.set($indent_count.get() - 1);
}

/// Indent解析
pub struct Indent(usize);
pub fn build_ast<'s, I>(
    indent_count: &'s Cell<usize>,
) -> impl Parser<'s, I, Vec<Command<'s>>, extra::Err<Rich<'s, Token<'s>>>> + Clone
where
    I: ValueInput<'s, Token = Token<'s>, Span = SimpleSpan>,
{
    recursive(|ast| {
        // 解析行
        let parse_empty = just(Token::Line).to(Command::NewLine);

        // 解析表达式
        let parse_expression = recursive(|expression| {
            // 解析基础对象(必须在parse_key_value之后，否则会覆盖parse_key_value的匹配)
            let parse_base_object = select! {
                Token::Number(num) => Expression::Object(Object::Constant(num)),
                Token::Str(s) => Expression::Object(Object::Str(s)),
                Token::Ident(s) => Expression::Object(Object::Variable(s)),
            };

            // 解析键值对
            let parse_key_value = parse_base_object
                .then_ignore(just(Token::Colon))
                .then(expression.clone())
                .map(|(k, v)| Expression::Object(Object::DictItem(Box::new(k), Box::new(v))))
                .boxed();

            // 解析数组
            let parse_array = expression
                .clone()
                .separated_by(just(Token::Comma))
                .allow_trailing()
                .collect()
                .delimited_by(just(Token::LeftBracket), just(Token::RightBracket))
                .map(|e| Expression::Object(Object::Array(e)))
                .boxed();

            // 解析字典
            let parse_dict = expression
                .clone()
                .separated_by(just(Token::Comma))
                .allow_trailing()
                .collect()
                .delimited_by(just(Token::LeftBrace), just(Token::RightBrace))
                .map(|e| Expression::Object(Object::Dict(e)))
                .boxed();

            // 解析元组
            let parse_tuple = expression
                .clone()
                .separated_by(just(Token::Comma))
                .allow_trailing()
                .collect()
                .delimited_by(just(Token::LeftParen), just(Token::RightParen))
                .map(|e| Expression::Object(Object::Tuple(e)))
                .boxed();

            // 解析查询运算符
            let parse_query_single = {
                let parse_query = just(Token::And).or(just(Token::Or)).then(select! {
                    Token::Ident(s) => s
                });

                just(Token::Query)
                    .ignore_then(
                        just(Token::Less)
                            .ignore_then(
                                select! {
                                    Token::Ident(s) => s
                                }
                                .then(
                                    parse_query
                                        .clone()
                                        .repeated()
                                        .collect::<Vec<(Token, &str)>>(),
                                )
                                .or_not(),
                            )
                            .then_ignore(just(Token::Greater))
                            .or_not(),
                    )
                    .map(|e| {
                        if let Some(Some((first, e))) = e {
                            let mut with_compoents = vec![first];
                            let mut without_compoents = vec![];
                            for (logic, compoent) in e {
                                match logic {
                                    Token::Or => {
                                        with_compoents.push(compoent);
                                    }
                                    Token::And => {
                                        without_compoents.push(compoent);
                                    }
                                    token => panic!("unexpected token:{token:?}"),
                                }
                            }
                            Expression::Query {
                                with_compoents,
                                without_compoents,
                            }
                        } else {
                            Expression::Query {
                                with_compoents: vec![],
                                without_compoents: vec![],
                            }
                        }
                    })
            }
            .boxed();

            // 解析值
            let parse_value = parse_key_value
                .or(parse_base_object)
                .or(parse_array.clone())
                .or(parse_dict.clone())
                .or(parse_tuple.clone())
                .or(parse_query_single.clone());
            // 解析左运算符
            let parse_left_op = select! {
                Token::Add=>UnaryOp::Plus,
                Token::Sub=>UnaryOp::Minus,
                Token::Not=>UnaryOp::Not,
            };

            // 解析左表达式基本值
            let parse_left_value = parse_left_op
                .then(parse_value.clone())
                .map(|(op, hs)| Expression::Unary {
                    op,
                    hs: Box::new(hs),
                })
                .boxed();
            // 解析问号运算基本值
            let parse_left_question = parse_value
                .clone()
                .then(just(Token::Question))
                .map(|(hs, _)| Expression::Unary {
                    op: UnaryOp::Question,
                    hs: Box::new(hs),
                })
                .boxed();

            // 解析运算符
            let parse_binary_op = select! {
                Token::Add => BinaryOp::Add,
                Token::Sub => BinaryOp::Sub,
                Token::Mul => BinaryOp::Mul,
                Token::Div => BinaryOp::Div,
                Token::Equal => BinaryOp::Equal,
                Token::NotEqual => BinaryOp::NotEqual,
                Token::Greater => BinaryOp::Greater,
                Token::Less => BinaryOp::Less,
                Token::GreaterEqual => BinaryOp::GreaterEqual,
                Token::LessEqual => BinaryOp::LessEqual,
                Token::And => BinaryOp::And,
                Token::Or => BinaryOp::Or,
                Token::Pow => BinaryOp::Pow,
                Token::Mod => BinaryOp::Mod,
                Token::Not => BinaryOp::Not,
                Token::Dot => BinaryOp::Dot,
                Token::Assign=>BinaryOp::Assign,
                Token::AddAssign=>BinaryOp::AddAssign,
                Token::SubAssign=>BinaryOp::SubAssign,
                Token::MulAssign=>BinaryOp::MulAssign,
                Token::DivAssign=>BinaryOp::DivAssign,
                Token::ModAssign=>BinaryOp::ModAssign,
                Token::PowAssign=>BinaryOp::PowAssign,
            };
            // 解析基础运算符
            let parse_base_binary_op_value = parse_binary_op.then(expression.clone()).boxed();
            // call运算符，拿到call的数据
            let parse_call_binary_op_value =
                parse_tuple.clone().map(|e| (BinaryOp::Call, e)).boxed();
            // index运算符，拿到index数据
            let parse_index_binary_op_value =
                parse_array.clone().map(|e| (BinaryOp::Index, e)).boxed();
            // 取表运算符，拿到表数据
            let parse_table_binary_op_value =
                parse_dict.clone().map(|e| (BinaryOp::Key, e)).boxed();
            // 基本表达式
            let parse_base_value = parse_left_question
                .clone()
                .or(parse_value.clone())
                .or(parse_left_value.clone());
            // 运算表达式
            let parse_op_value = parse_base_binary_op_value
                .clone()
                .or(parse_call_binary_op_value.clone())
                .or(parse_index_binary_op_value.clone())
                .or(parse_table_binary_op_value.clone());

            // 解析表达式
            parse_base_value
                .then(parse_op_value.clone().or_not())
                .map(|(hs, op)| {
                    if let Some((op, ts)) = op {
                        Expression::Binary {
                            op,
                            lhs: Box::new(hs),
                            rhs: Box::new(ts),
                        }
                    } else {
                        hs
                    }
                })
        })
        .then_ignore(just(Token::Line).or_not());
        // 忽略tab
        let parse_ignored_tab = just(Token::Tab)
            .repeated()
            .configure(|repeated, _| repeated.exactly(indent_count.get()));

        // elif解析器
        let parse_elif = just(Token::Elif)
            .ignore_then(parse_expression.clone())
            .then_ignore(just(Token::Colon).then(just(Token::Line)))
            .map(add_indent!(indent_count))
            .then(ast.clone().repeated().collect())
            .map(|(condition, commands)| {
                sub_indent!(indent_count);
                Branch {
                    condition,
                    commands,
                }
            });
        // else解析器
        let parse_else = just(Token::Else)
            .ignore_then(just(Token::Colon).then(just(Token::Line)))
            .map(add_indent!(indent_count))
            .then(ast.clone().repeated().collect::<Vec<Command>>())
            .map(|(_, commands)| {
                sub_indent!(indent_count);
                commands
            });
        // if解析器
        let parse_if = just(Token::If)
            .ignore_then(parse_expression.clone())
            .then_ignore(just(Token::Colon).then(just(Token::Line)))
            .map(add_indent!(indent_count))
            .then(ast.clone().repeated().collect())
            .map(|(condition, commands)| {
                sub_indent!(indent_count);
                Branch {
                    condition,
                    commands,
                }
            })
            .then(parse_elif.clone().repeated().collect::<Vec<Branch>>())
            .then(parse_else.clone().or_not())
            .map(|((branch, mut elifs), else_branch)| {
                elifs.insert(0, branch);
                Command::If {
                    if_branch: elifs,
                    else_branch,
                }
            });

        parse_ignored_tab
            .ignore_then(parse_empty.or(parse_expression.map(Command::Expression).or(parse_if)))
    })
    .repeated()
    .collect()
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use chumsky::{
        input::{Input, Stream},
        Parser,
    };
    use logos::Logos;

    use crate::parser::tokenizer::Token;

    use super::build_ast;

    #[test]
    fn test_build_ast() {
        let lex = r#"
if Query<Dog|Cat&Pig>:
    1+2
elif 1:
    1

"#;
        let token_sequence = Token::lexer(lex)
            .spanned()
            .map(|(token_result, span)| {
                let token = token_result.unwrap();
                print!("{:?}\t", token);
                (token, span.into())
            })
            .collect::<Vec<_>>();

        // Construct a token stream suitable for the parser
        let end_pos = lex.len();
        let token_stream = Stream::from_iter(token_sequence).spanned((end_pos..end_pos).into());
        let indent_count = Cell::new(0);
        // Attempt to parse the token stream into an abstract syntax tree (AST)
        let ast = build_ast(&indent_count)
            .parse(token_stream)
            .into_result()
            .map_err(|parse_errors| format!("Parsing error: {:?}", parse_errors));
        println!("{:#?}", ast);
    }
}

#[cfg(test)]
mod test22 {
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    use chumsky::{
        error::{EmptyErr, Rich, Simple},
        extra::{self, State},
        prelude::{just, one_of},
        select, text, ConfigIterParser, ConfigParser, IterParser, Parser,
    };
    #[test]
    fn hahaha() {
        let indent_count = Cell::new(0);
        let generic = just(b'0')
            .repeated()
            .configure(|repeated, _| repeated.exactly(indent_count.get()));
        let parse = just::<_, _, extra::Default>(b'b')
            .map(|e| {
                indent_count.set(2);
                e
            })
            .or_not()
            .then(generic.with_ctx(()));

        println!("{:#?}", parse.parse(b"b00").into_result());
    }
}
