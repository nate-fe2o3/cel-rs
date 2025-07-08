use anyhow::{Context, Result, anyhow};
use itertools::{Itertools, PeekNth, peek_nth};
use owo_colors::OwoColorize;
use proc_macro2::{Delimiter, Group, Span, TokenStream, TokenTree};
use quote::{TokenStreamExt, quote, quote_spanned};
use std::mem::discriminant;

use crate::tokens::*;

fn peek<T: Iterator<Item = TokenTree>>(input: &mut PeekNth<T>) -> Result<&TokenTree> {
    let t = input.peek().context("Unexpected Eof")?;
    Ok(t)
}

fn peek_n<T: Iterator<Item = TokenTree>>(input: &mut PeekNth<T>, n: usize) -> Result<&TokenTree> {
    let t = input.peek_nth(n).context("Unexpected Eof")?;
    Ok(t)
}

fn next<T: Iterator<Item = TokenTree>>(input: &mut PeekNth<T>) -> Result<TokenTree> {
    let t = input.next().context("Unexpected EoF")?;
    Ok(t)
}

fn expect_token_type<T: Iterator<Item = TokenTree>>(
    input: &mut PeekNth<T>,
    expected: &TokenTree,
) -> Result<TokenTree> {
    let t = peek(input)?;
    if discriminant(t) == discriminant(expected) {
        return next(input);
    }
    Err(anyhow!("Unexpected token: Expected {expected}, found {t}"))
}

fn expect_token<T: Iterator<Item = TokenTree>>(
    input: &mut PeekNth<T>,
    expected: &TokenTree,
) -> Result<TokenTree> {
    let t = peek(input)?;
    if match_token(t, expected) {
        next(input)
    } else {
        Err(anyhow!("Unexpected token: Expected {expected}, found {t}"))
    }
}

fn match_token_type(input: &TokenTree, expected: &TokenTree) -> bool {
    discriminant(input) == discriminant(expected)
}

fn match_any_token(input: &TokenTree, expected: &[TokenTree]) -> bool {
    expected.iter().any(|x| match_token(input, x))
}

fn match_token(input: &TokenTree, expected: &TokenTree) -> bool {
    match (expected, input) {
        (TokenTree::Group(a), TokenTree::Group(b)) => a.delimiter() == b.delimiter(),
        (TokenTree::Ident(a), TokenTree::Ident(b)) => a == b,
        (TokenTree::Punct(a), TokenTree::Punct(b)) => {
            (a.as_char(), a.spacing()) == (b.as_char(), b.spacing())
        }
        (TokenTree::Literal(a), TokenTree::Literal(b)) => a.to_string() == b.to_string(),
        _ => false,
    }
}

fn get_group(t: TokenTree) -> Result<Group> {
    let TokenTree::Group(g) = t else {
        return Err(anyhow!("Not a group"));
    };
    Ok(g)
}

// expression = or_expression ["?" expression ":" expression].
// TODO: confirm re-add of error on leftover tokens
fn parse_expression<T: Iterator<Item = TokenTree>>(input: &mut PeekNth<T>) -> Result<TokenStream> {
    println!("expr");
    let mut output = TokenStream::new();
    let or_expr = parse_passthrough_expression(input)?;
    if let Ok(token) = peek(input)
        && match_token(token, &QUESTION)
    {
        next(input)?;
        let expr1 = parse_expression(input)?;
        expect_token(input, &COLON)?;
        let expr2 = parse_expression(input)?;
        output.extend(quote! {if #or_expr {#expr1} else {#expr2}});
    } else {
        output.extend(or_expr);
    }
    Ok(output)
}

fn parse_passthrough_expression<T: Iterator<Item = TokenTree>>(
    input: &mut PeekNth<T>,
) -> Result<TokenStream> {
    println!("passthru");
    let mut output = parse_postfix_expression(input)?;
    let pass_tokens = vec![
        LT.clone(),
        GT.clone(),
        BITOR.clone(),
        BITAND.clone(),
        BITXOR.clone(),
        ADD.clone(),
        SUB.clone(),
        MUL.clone(),
        DIV.clone(),
        MOD.clone(),
        NOT.clone(),
    ];
    loop {
        let Ok(t) = peek(input) else {
            break;
        };
        let token = &t.clone();
        let Ok(t2) = peek_n(input, 1) else { break };
        let token2 = &t2.clone();
        //single
        if match_any_token(token, &pass_tokens) {
            let tree = next(input)?;
            output.append(tree);
            // double
            if match_token(token, &OR_J) && match_token(token2, &BITOR)
                || match_token(token, &AND_J) && match_token(token2, &BITAND)
                || match_token(token, &EQ_J) && match_token(token2, &ASSIGN)
                || match_token(token, &NOT_J) && match_token(token2, &ASSIGN)
                || match_token(token, &GT_J) && match_token(token2, &ASSIGN)
                || match_token(token, &LT_J) && match_token(token2, &ASSIGN)
                || match_token(token, &LT_J) && match_token(token2, &LT)
                || match_token(token, &GT_J) && match_token(token2, &GT)
            {
                output.append(next(input)?);
                output.append(next(input)?);
            }
        } else {
            break;
        }
        output.extend(parse_postfix_expression(input));
    }
    Ok(output)
}

// postfix_expression = primary_expression { ("[" expression "]") | ("." identifier) }.
fn parse_postfix_expression<T: Iterator<Item = TokenTree>>(
    input: &mut PeekNth<T>,
) -> Result<TokenStream> {
    println!("pf");
    let mut output = parse_primary_expression(input)?;
    while let Ok(token) = peek(input) {
        if match_token(token, &SQUARE_GROUP) {
            let tree = next(input)?;
            let group = get_group(tree)?.stream().into_iter();
            let sub_expr = parse_expression(&mut peek_nth(group))?;
            output.extend(quote! {.get(#sub_expr).unwrap()});
        } else if match_token(token, &DOT) {
            next(input)?;
            let t = expect_token_type(input, &IDENT)?;
            output.extend(quote! {.get(#t).unwrap()});
        } else {
            break;
        }
    }
    Ok(output)
}

fn parse_primary_expression<T: Iterator<Item = TokenTree>>(
    input: &mut PeekNth<T>,
) -> Result<TokenStream> {
    let mut output = TokenStream::new();
    println!("primary");

    // Names
    if match_token(peek(input)?, &AT) {
        next(input)?;
        let ident = expect_token_type(input, &IDENT)?;
        output.append(ident);
    }
    // Literals: strings, characters, integers, floats
    else if match_token_type(peek(input)?, &LIT) {
        output.append(next(input)?);
    }
    // Idents: booleans, variable names, keywords, functions
    // TODO: filter out rust keywords that do not belong
    else if match_token_type(peek(input)?, &IDENT) {
        output.append(next(input)?);
        //function signature
        if match_token(peek(input)?, &PARENS_GROUP) {
            let parsed_group = TokenTree::Group(Group::new(
                Delimiter::Parenthesis,
                parse_argument_expression_list(input)?,
            ));

            output.append(parsed_group);
        }
    }
    // Arrays
    else if match_token(peek(input)?, &SQUARE_GROUP) {
        output.extend(parse_array_literal(input)?);
    }
    // Dictionaries
    else if match_token(peek(input)?, &CURLY_GROUP) {
        output.extend(parse_dictionary_literal(input)?);
    } else {
        return Err(anyhow!(
            "Unexpected token: no primary definitions found for {}",
            peek(input)?
        ));
    }
    Ok(output)
}

fn parse_argument_expression_list<T: Iterator<Item = TokenTree>>(
    input: &mut PeekNth<T>,
) -> Result<TokenStream> {
    let mut output = TokenStream::new();
    let group_token = next(input)?;
    let arg_stream = get_group(group_token)?.stream();
    if !arg_stream.is_empty() {
        let mut nested = peek_nth(arg_stream);
        let first = peek(&mut nested)?;
        if let Ok(maybe_colon) = peek_n(&mut nested, 1)
            && match_token(maybe_colon, &COLON)
        {
            output.extend(parse_named_argument_list(input)?);
        } else {
            output.extend(parse_argument_list(input)?);
        }
    }
    Ok(output)
}

// argument_list = expression { "," expression }.
fn parse_argument_list<T: Iterator<Item = TokenTree>>(
    input: &mut PeekNth<T>,
) -> Result<TokenStream> {
    let mut output = parse_expression(input)?;
    while let Ok(token) = peek(input)
        && match_token(token, &COMMA)
    {
        output.append(COMMA.clone());
        output.extend(parse_expression(input)?);
    }
    Ok(output)
}

// named_argument_list = named_argument { "," named_argument }.
fn parse_named_argument_list<T: Iterator<Item = TokenTree>>(
    input: &mut PeekNth<T>,
) -> Result<TokenStream> {
    let mut output = parse_named_argument(input)?;
    while let Ok(token) = peek(input)
        && match_token(token, &COMMA)
    {
        output.append(COMMA.clone());
        output.extend(parse_named_argument(input)?);
    }
    Ok(output)
}

// named_argument = identifier ":" expression.
fn parse_named_argument<T: Iterator<Item = TokenTree>>(
    input: &mut PeekNth<T>,
) -> Result<TokenStream> {
    let mut output = TokenStream::new();
    output.append(expect_token_type(input, &IDENT)?);
    output.append(expect_token(input, &COLON)?);
    output.extend(parse_expression(input)?);
    Ok(output)
}

// array = "[" [argument_list] "]".
fn parse_array_literal<T: Iterator<Item = TokenTree>>(
    input: &mut PeekNth<T>,
) -> Result<TokenStream> {
    let mut output = TokenStream::new();
    let group = expect_token(input, &SQUARE_GROUP)?;
    let arg_stream = get_group(group)?.stream();
    if !arg_stream.is_empty() {
        let mut nested = peek_nth(arg_stream);
        output.extend(parse_argument_list(&mut nested)?);
    }
    Ok(quote! {vec![#output]})
}

// dictionary = "{" [named_argument_list] "}".
fn parse_dictionary_literal<T: Iterator<Item = TokenTree>>(
    input: &mut PeekNth<T>,
) -> Result<TokenStream> {
    let mut output = TokenStream::new();

    let group = expect_token(input, &CURLY_GROUP)?;
    let arg_stream = get_group(group)?.stream();
    if !arg_stream.is_empty() {
        let mut nested = peek_nth(arg_stream);
        output.extend(parse_named_argument_list(&mut nested)?);
    }
    Ok(quote! {(#output).into_tuple_list()})
}

pub struct CELParser2<I: Iterator<Item = TokenTree>> {
    tokens: PeekNth<I>,
    output: TokenStream,
}
impl<I: Iterator<Item = TokenTree> + Clone> CELParser2<I> {
    pub fn extract_error_message(&self) -> Option<String> {
        let output_str = self.output.to_string();

        // Look for compile_error ! ("message") - note the spaces
        if let Some(start) = output_str.find("compile_error ! (\"") {
            let start = start + "compile_error ! (\"".len();
            if let Some(end) = output_str[start..].find("\")") {
                return Some(output_str[start..start + end].to_string());
            }
        }
        None
    }

    /// https://github.com/rust-lang/rustc-dev-guide/blob/master/src/diagnostics.md
    pub fn format_error(
        &self,
        source_code: &str,
        filename: &str,
        start_line: u32,
    ) -> Option<String> {
        if let Some(error_msg) = self.extract_error_message() {
            if let Some(span) = self.get_error_span() {
                return Some(self.format_rustc_style(
                    &error_msg,
                    span,
                    source_code,
                    filename,
                    start_line,
                ));
            }
        }
        None
    }

    fn format_rustc_style(
        &self,
        message: &str,
        span: Span,
        source: &str,
        filename: &str,
        start_line: u32,
    ) -> String {
        let start = span.start();
        let end = span.end();

        let lines: Vec<&str> = source.lines().collect();

        let mut output = String::new();

        // Calculate offset line numbers (start_line is 1-based)
        let error_line = start_line + (start.line as u32) - 1;
        let error_column = start.column + 1; // +1 because the column is 0-based but the error is 1-based

        // Calculate the width needed for line numbers
        // end.line is the last line within the source span (1-based)
        // start_line is the offset to get actual file line numbers
        // The maximum displayed line number will be: start_line + end.line - 1
        let max_line_num = start_line + (end.line as u32) - 1;
        let line_width = max_line_num.to_string().len();

        // Error header with red and bold "error:"
        output.push_str(&format!("{}: {}\n", "error".red().bold(), message));
        output.push_str(&format!(
            " {} {}:{}:{}\n",
            "-->".blue().bold(),
            filename.blue(),
            error_line.to_string().blue(),
            error_column.to_string().blue()
        ));
        output.push_str(&format!(
            "{:width$} {}\n",
            "",
            "|".blue().bold(),
            width = line_width
        ));

        // Show the problematic line(s)
        for line_num in start.line..=end.line {
            if let Some(line_content) = lines.get(line_num.saturating_sub(1)) {
                let display_line_num = start_line + (line_num as u32) - 1;
                output.push_str(&format!(
                    "{} {} {}\n",
                    display_line_num.to_string().blue().bold(),
                    "|".blue().bold(),
                    line_content
                ));

                // Add caret indicators
                if line_num == start.line {
                    output.push_str(&format!(
                        "{:width$} {} ",
                        "",
                        "|".blue().bold(),
                        width = line_width
                    ));

                    // Add spaces up to start column
                    output.push_str(&" ".repeat(start.column));

                    // Add carets in red
                    let caret_len = if start.line == end.line {
                        end.column.saturating_sub(start.column).max(1)
                    } else {
                        line_content
                            .len()
                            .saturating_sub(start.column.saturating_sub(1))
                    };

                    output.push_str(&"^".repeat(caret_len).red().bold().to_string());
                    output.push('\n');
                }
            }
        }

        output
    }

    fn get_error_span(&self) -> Option<Span> {
        // The compile_error! TokenStream structure is:
        // TokenTree::Ident("compile_error") - with the span we want
        // TokenTree::Punct('!')
        // TokenTree::Group(...) - containing the message, also with the span

        let mut tokens = self.output.clone().into_iter();

        // Look for the first token (should be "compile_error" ident)
        if let Some(first_token) = tokens.next() {
            match first_token {
                TokenTree::Ident(ident) if ident == "compile_error" => {
                    return Some(ident.span());
                }
                _ => {
                    // Fallback: try to get span from any token in the stream
                    return Some(first_token.span());
                }
            }
        }
        None
    }

    pub fn new(tokens: I) -> Self {
        let output = TokenStream::new();
        CELParser2 {
            tokens: peek_nth(tokens),
            output,
        }
    }

    pub fn run(&mut self) -> Result<()> {
        let tokens = parse_expression(&mut self.tokens)?;
        self.output.extend(tokens);
        Ok(())
    }

    pub fn get_output(&self) -> &TokenStream {
        &self.output
    }

    pub fn report_error(&mut self, message: &str) -> bool {
        let span = self
            .tokens
            .peek()
            .map_or_else(proc_macro2::Span::call_site, |token| token.span());
        self.output = quote_spanned!(span => compile_error!(#message));
        false
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_simple_number() {
        let t = TokenStream::from_str("123 + 52").unwrap();
        let mut p = CELParser2::new(t.into_iter());
        p.run().unwrap();
        dbg!(p.get_output());
        assert_eq!(1, 2)
    }

    //     #[test]
    //     fn test_simple_string() {
    //         let (mut p, e) = parse_str("\"hello\"");
    //         assert!(e.is_ok());
    //         assert_eq!(p.seg.pop(), Value::String("hello".into()));
    //     }
    //
    //     #[test]
    //     fn test_boolean_true() {
    //         let (mut p, e) = parse_str("true");
    //         assert!(e.is_ok());
    //         assert_eq!(p.seg.pop(), Value::Boolean(true));
    //     }
    //
    //     #[test]
    //     fn test_multipeek() {
    //         let input = "hey there";
    //         let mut lexer = Lexer::new(input);
    //         lexer.lex().unwrap();
    //         println!("{:?}", lexer.tokens);
    //         let mut parser = Parser::new(lexer.tokens);
    //         assert_eq!(
    //             Token::Identifier("hey".into()),
    //             parser.peek_multi().unwrap().value
    //         );
    //         assert_eq!(
    //             Token::Identifier("there".into()),
    //             parser.peek_multi().unwrap().value
    //         );
    //     }
    //
    //     #[test]
    //     fn test_boolean_false() {
    //         let (mut p, e) = parse_str("false");
    //         assert!(e.is_ok());
    //         assert_eq!(p.seg.pop(), Value::Boolean(false));
    //     }
    //
    //     // #[test]
    //     // fn test_stuff() {
    //     //     assert_eq!(
    //     //         parse_str("5 > 3 ? x = 2 : x = 3 "),
    //     //         Ok(Expression::Literal(Token::False))
    //     //     );
    //     // }
    //
    //     // #[test]
    //     // fn test_empty_keyword() {
    //     //     assert_eq!(parse_str("empty"), Ok(Expression::Literal(Token::Empty)));
    //     // }
    //
    //     #[test]
    //     fn test_parenthesized_expression() {
    //         let (mut p, e) = parse_str("(123)");
    //         assert!(e.is_ok());
    //         assert_eq!(p.seg.pop(), Value::Num(123.));
    //     }
    //
    //     // #[test]
    //     // fn test_name_expression() {
    //     //     assert_eq!(
    //     //         parse_str("@my_var"),
    //     //         Ok(Expression::Literal(Token::Name("my_var".to_string())))
    //     //     );
    //     // }
    //
    //     // #[test]
    //     // fn test_name_with_keyword() {
    //     //     assert_eq!(
    //     //         parse_str("@true"),
    //     //         Ok(Expression::Literal(Token::Name("true".to_string())))
    //     //     );
    //     // }
    //
    //     // #[test]
    //     // fn test_simple_variable() {
    //     //     let mut p = parse_str("myVar");
    //     //     println!("{:?}", p.seg.stack);
    //     //     assert_eq!(p.seg.pop(), Value::String("myVar".into()));
    //     // }
    //
    //     #[test]
    //     fn test_simple_function_call_no_args() {
    //         let mut l = Lexer::new("myFunc()");
    //         l.lex().unwrap();
    //         let mut p = Parser::new(l.tokens);
    //         p.seg.register0("myFunc", || Value::Num(500.));
    //         p.parse_expression().unwrap();
    //         println!("WTF: {:?}", p.seg.stack);
    //         assert_eq!(p.seg.pop(), Value::Num(500.))
    //     }
    //
    //     #[test]
    //     fn test_function_call_one_arg() {
    //         let (mut p, e) = parse_str("neg(5)");
    //         assert!(e.is_ok());
    //         assert_eq!(p.seg.pop(), Value::Num(-5.))
    //     }
    //
    //     #[test]
    //     fn test_function_call_multiple_args() {
    //         let (mut p, e) = parse_str("add(\"This is a \", \"test\")");
    //         assert!(e.is_ok());
    //         assert_eq!(p.seg.pop(), Value::String("This is a test".into()));
    //     }
    //
    //     // #[test]
    //     // fn test_function_call_one_named_arg() {
    //     //     assert_eq!(
    //     //         parse_str("config(mode: \"fast\")"),
    //     //         Ok(Expression::FunctionCall {
    //     //             name: "config".to_string(),
    //     //             arguments: vec![], // No positional if only named were parsed this way
    //     //             named_arguments: vec![(
    //     //                 "mode".to_string(),
    //     //                 Expression::Literal(Token::String("fast".to_string()))
    //     //             )]
    //     //         })
    //     //     );
    //     // }
    //
    //     // #[test]
    //     // fn test_function_call_mixed_args_fail_current_logic() {
    //     //     // Current parse_argument_expression_list commits to either full positional or full named.
    //     //     // This test should fail as `num:1` would be seen, then `2` would be an error.
    //     //     let result = parse_str("setup(num:1, 2)");
    //     //     assert!(result.is_err());
    //     // }
    //
    //     #[test]
    //     fn test_empty_array_literal() {
    //         let (mut p, e) = parse_str("[]");
    //         assert!(e.is_ok());
    //         assert_eq!(p.seg.pop(), Value::Vec(vec![]));
    //     }
    //
    //     #[test]
    //     fn test_array_literal_one_element() {
    //         let (mut p, e) = parse_str("[123]");
    //         assert!(e.is_ok());
    //         assert_eq!(p.seg.pop(), Value::Vec(vec![Value::Num(123.)]));
    //     }
    //
    //     #[test]
    //     fn test_array_literal_multiple_elements() {
    //         let (mut p, e) = parse_str("[1, \"two\"]");
    //         assert!(e.is_ok());
    //         assert_eq!(
    //             p.seg.pop(),
    //             Value::Vec(vec![Value::Num(1.), Value::String("two".into())])
    //         );
    //     }
    //
    //     // #[test]
    //     // fn test_empty_dictionary_literal() {
    //     //     assert_eq!(parse_str("{}"), Ok(Expression::DictionaryLiteral(vec![])));
    //     // }
    //
    //     // #[test]
    //     // fn test_dictionary_literal_one_member() {
    //     //     assert_eq!(
    //     //         parse_str("{key: \"value\"}"),
    //     //         Ok(Expression::DictionaryLiteral(vec![(
    //     //             "key".to_string(),
    //     //             Expression::Literal(Token::String("value".to_string()))
    //     //         )]))
    //     //     );
    //     // }
    //
    //     #[test]
    //     fn test_dictionary_literal_multiple_members() {
    //         let (mut p, e) = parse_str("{a: 1, b: true}");
    //         assert!(e.is_ok());
    //         assert_eq!(
    //             p.seg.pop(),
    //             Value::Dict(HashMap::from([
    //                 ("a".into(), Value::Num(1.)),
    //                 ("b".into(), Value::Boolean(true))
    //             ]))
    //         );
    //     }
    //
    //     #[test]
    //     fn test_addition() {
    //         let (mut p, e) = parse_str("1 + 2");
    //         assert!(e.is_ok());
    //         println!("{:?}", p.seg.stack);
    //         assert_eq!(p.seg.pop(), Value::Num(3.));
    //     }
    //
    //     #[test]
    //     fn test_ternary_operator() {
    //         let (mut p, e) = parse_str("true ? 1 : 2");
    //         assert!(e.is_ok());
    //         assert_eq!(p.seg.pop(), Value::Num(1.));
    //     }
    //
    //     #[test]
    //     fn test_unary_minus() {
    //         let (mut p, e) = parse_str("-5");
    //         assert!(e.is_ok());
    //         assert_eq!(p.seg.pop(), Value::Num(-5.));
    //     }
    //
    //     #[test]
    //     fn test_unary_not() {
    //         let (mut p, e) = parse_str("!true");
    //         assert!(e.is_ok());
    //         assert_eq!(p.seg.pop(), Value::Boolean(false));
    //     }
    //
    //     //TODO: why should this be an error in a stack based language
    //     // #[test]
    //     // fn test_incomplete_expression() {
    //     //     let (mut p, e) = parse_str("10 + 25 25");
    //     //     assert!(e.is_err());
    //     //     println!("{e:?}");
    //     //     // assert_eq!(
    //     //     //     e,
    //     //     //     vec![Value::String(
    //     //     //         "compile_error ! (\"Unexpected token\")".into()
    //     //     //     )]
    //     //     // );
    //     // }
    //
    //     #[test]
    //     fn test_arithmetic_expression() {
    //         let (mut p, e) = parse_str("10 + 20 * 30");
    //         assert!(e.is_ok());
    //         assert_eq!(p.seg.pop(), Value::Num(610.));
    //     }
    //
    //     #[test]
    //     fn test_parenthesized_expression_2() {
    //         let (mut p, e) = parse_str("(10 + 20) * 30");
    //         assert!(e.is_ok());
    //         assert_eq!(p.seg.pop(), Value::Num(900.))
    //     }
    //
    //     #[test]
    //     fn test_complex_expression() {
    //         let (mut p, e) = parse_str("10 + 20 * (30 - 5) / 2");
    //         assert!(e.is_ok());
    //         assert_eq!(p.seg.pop(), Value::Num(260.))
    //     }
    //
    //     // TODO: how are we resolving all of these
    //     // #[test]
    //     // fn test_logical_expression() {
    //     //     let mut p = parse_str("a && b || c");
    //     //     assert!(true);
    //     // }
    //     //
    //     // #[test]
    //     // fn test_comparison_expression() {
    //     //     let input = TokenStream::from_str("a == b && c > d").unwrap();
    //     //     let mut parser = CELParser2::new(input.into_iter());
    //     //     assert!(parser.is_expression());
    //     // }
    //     //
    //     // #[test]
    //     // fn test_bitwise_expression() {
    //     //     let input = TokenStream::from_str("a | b & c ^ d").unwrap();
    //     //     let mut parser = CELParser2::new(input.into_iter());
    //     //     assert!(parser.is_expression());
    //     // }
    //     //
    //     // #[test]
    //     // fn test_shift_expression() {
    //     //     let input = TokenStream::from_str("a << 2 + b >> 1").unwrap();
    //     //     let mut parser = CELParser2::new(input.into_iter());
    //     //     assert!(parser.is_expression());
    //     // }
    //     //
    //     // #[test]
    //     // fn test_unary_expression() {
    //     //     let input = TokenStream::from_str("-a + !b").unwrap();
    //     //     let mut parser = CELParser2::new(input.into_iter());
    //     //     assert!(parser.is_expression());
    //     // }
    //     //
    //     // #[test]
    //     // fn test_chained_unary_expression() {
    //     //     let input = TokenStream::from_str("!!a + --b").unwrap();
    //     //     let mut parser = CELParser2::new(input.into_iter());
    //     //     assert!(parser.is_expression());
    //     // }
    //
    //     #[test]
    //     fn test_invalid_expression() {
    //         let (mut p, e) = parse_str("+");
    //         assert!(e.is_err());
    //         assert_eq!(
    //             e.unwrap_err(),
    //             ParseError::UnexpectedToken {
    //                 expected: Some("primary expression type (@, literal, [, {, identifier, (".into()),
    //                 found: Token::Eof,
    //                 line: 1,
    //                 column: 2
    //             }
    //         );
    //     }
    //
    //     // #[test]
    //     // fn test_error_formatting() {
    //     //     let source = "10 + 20 30"; // Missing operator between 20 and 30
    //     //     let input = TokenStream::from_str(source).unwrap();
    //     //     let mut parser = CELParser2::new(input.into_iter());
    //     //
    //     //     // This should fail parsing
    //     //     assert!(!parser.is_expression());
    //     //
    //     //     // Test error message extraction
    //     //     let error_msg = parser.extract_error_message();
    //     //     assert!(error_msg.is_some());
    //     //     assert_eq!(error_msg.unwrap(), "Unexpected token");
    //     //
    //     //     // Test error formatting
    //     //     let formatted_error = parser.format_error(source, "test.cel", 1u32);
    //     //     assert!(formatted_error.is_some());
    //     //
    //     //     let formatted = formatted_error.unwrap();
    //     //     assert!(formatted.contains("error: Unexpected token"));
    //     //     assert!(formatted.contains("test.cel:1:")); // Should include line number
    //     //     assert!(formatted.contains("1 | 10 + 20 30")); // Should show the line with line number
    //     //     assert!(formatted.contains("^")); // Should have carets pointing to the error
    //     // }
    //
    //     // #[test]
    //     // fn test_error_formatting_with_line_offset() {
    //     //     let source = "a + b c"; // Missing operator between b and c
    //     //     let input = TokenStream::from_str(source).unwrap();
    //     //     let mut parser = CELParser2::new(input.into_iter());
    //     //
    //     //     // This should fail parsing
    //     //     assert!(!parser.is_expression());
    //     //
    //     //     // Test error formatting with line offset (as if expression starts at line 42)
    //     //     let formatted_error = parser.format_error(source, "large_file.rs", 42u32);
    //     //     assert!(formatted_error.is_some());
    //     //
    //     //     let formatted = formatted_error.unwrap();
    //     //     assert!(formatted.contains("error: Unexpected token"));
    //     //     assert!(formatted.contains("large_file.rs:42:")); // Should show offset line number
    //     //     assert!(formatted.contains("42 | a + b c")); // Should show the line with offset line number
    //     //     assert!(formatted.contains("^")); // Should have carets pointing to the error
    //     // }
    //
    //     // #[test]
    //     // fn print_error_formatting() {
    //     //     let line = line!() + 1;
    //     //     let source = r#"
    //     //         10 + 20 30 // Unexpected token
    //     //     "#;
    //     //     let input = TokenStream::from_str(source).unwrap();
    //     //     let mut parser = CELParser2::new(input.into_iter());
    //     //
    //     //     if !parser.is_expression() {
    //     //         // Format error starting at line 1
    //     //         if let Some(formatted_error) = parser.format_error(source, file!(), line) {
    //     //             println!("{}", formatted_error);
    //     //             // error: Unexpected token
    //     //             // --> cel-parser/src/lib.rs:593:21
    //     //             //     |
    //     //             // 593 |             10 + 20 30 // Unexpected token
    //     //             //     |                     ^^
    //     //         }
    //     //     }
    //     // }
    //
    //     // #[test]
    //     // fn test_postfix_array_access() {
    //     //     assert_eq!(parse_str("[1, 2, 3] myArray[0]"), Ok(expected));
    //     // }
    //
    //     // #[test]
    //     // fn test_postfix_member_access() {
    //     //     let expected = Expression::MemberAccess {
    //     //         object: Box::new(Expression::Identifier("myObj".to_string())),
    //     //         member: "field".to_string(),
    //     //     };
    //     //     assert_eq!(parse_str("myObj.field"), Ok(expected));
    //     // }
}
