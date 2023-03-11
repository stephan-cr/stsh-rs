// https://github.com/Geal/nom/blob/master/doc/choosing_a_combinator.md
// https://github.com/bminor/bash/blob/master/parse.y

use nom::{
    branch::alt,
    bytes::complete::{escaped, tag, take_while1},
    character::complete::{alphanumeric1, char, satisfy, space0, space1},
    combinator::{all_consuming, cond, opt, verify},
    multi::separated_list0,
    sequence::delimited,
    IResult, InputLength,
};

#[derive(Debug, PartialEq)]
pub(crate) struct InputRedirect<'a> {
    pub filename: &'a str,
    pub file_descriptor: u8,
}

#[derive(Debug, PartialEq)]
pub(crate) struct OutputRedirect<'a> {
    pub filename: &'a str,
    pub append: bool,
    pub file_descriptor: u8,
}

#[derive(Debug, PartialEq)]
pub(crate) struct Command<'a> {
    pub name: &'a str,
    pipe: bool,
    pub background: bool,
    pub input_file: Option<InputRedirect<'a>>,
    pub output_file: Option<OutputRedirect<'a>>,
    pub parameters: Vec<&'a str>,
}

pub(crate) fn parse(input: &str) -> IResult<&str, Vec<Command>> {
    // let pipe = char('|');
    // let sep = char(';');
    // let and = tag("&&");
    // let or = tag("||");
    // let subshell = delimited(char('('), ..., char(')'));
    // let star = char('*');
    // let questionmark = char('?');

    let (i, command) = all_consuming(parse_command)(input)?;

    Ok((i, vec![command]))
}

fn is_allowed_in_double_quotes(chr: char) -> bool {
    chr.is_alphanumeric() || chr.is_whitespace()
}

fn parse_command(input: &str) -> IResult<&str, Command> {
    let single_quote = char('\'');
    let double_quote = char('"');
    let background = char('&');
    let input_redirect = char('<');
    let output_redirect = tag(">");
    let output_redirect_append = tag(">>");
    let error_redirect = tag("2>"); // => it should be [n]>, where n can be any file descriptor, I guess

    let mut command = verify(escaped(alphanumeric1, '\\', &double_quote), |s: &str| {
        !s.is_empty()
    });

    let unquoted_param = verify(escaped(alphanumeric1, '\\', &double_quote), |s: &str| {
        !s.is_empty()
    });

    let param_within_quotes = take_while1(is_allowed_in_double_quotes);
    let quoted_param = alt((
        delimited(&double_quote, &param_within_quotes, &double_quote),
        delimited(&single_quote, &param_within_quotes, &single_quote),
    ));

    let (i, _) = space0(input)?; // ignore all leading whitespace
    let (i, command_name) = command(i)?;
    // todo!("allow / in command names");
    let (i, _) = space0(i)?;
    let (i, parameters) = separated_list0(space1, alt((quoted_param, unquoted_param)))(i)?;
    let (i, _) = space0(i)?;
    let (i, has_input_redirect) = opt(input_redirect)(i)?;
    let (i, _) = space0(i)?;
    let (i, input_file) = cond(has_input_redirect.is_some(), alphanumeric1)(i)?;
    let (i, _) = space0(i)?;
    let (i, has_output_redirect) = opt(alt((
        error_redirect,
        output_redirect_append,
        output_redirect,
    )))(i)?;
    let (i, _) = space0(i)?;
    let (i, output_file) = cond(has_output_redirect.is_some(), alphanumeric1)(i)?;
    let (i, _) = space0(i)?;
    let (i, background) = opt(background)(i)?;
    let (i, _) = space0(i)?; // ignore all trailing whitespace

    Ok((
        i,
        Command {
            name: command_name,
            pipe: false,
            background: background.is_some(),
            input_file: input_file.map(|file| InputRedirect {
                filename: file,
                file_descriptor: 0,
            }),
            output_file: output_file.map(|file| OutputRedirect {
                filename: file,
                append: has_output_redirect == Some(">>"),
                file_descriptor: if has_output_redirect == Some("2>") {
                    2
                } else {
                    1
                },
            }),
            parameters: parameters,
        },
    ))
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_parse_commands() {
        let result = super::parse("foo bar &");
        assert!(result.is_ok());

        assert_eq!(
            result,
            Ok((
                "",
                vec![super::Command {
                    name: "foo",
                    pipe: false,
                    background: true,
                    input_file: None,
                    output_file: None,
                    parameters: vec!["bar"]
                }]
            ))
        );
    }

    #[test]
    fn test_parse_command() {
        assert_eq!(
            super::parse_command("abc &"),
            Ok((
                "",
                super::Command {
                    name: "abc",
                    pipe: false,
                    background: true,
                    input_file: None,
                    output_file: None,
                    parameters: vec![]
                }
            ))
        );

        assert_eq!(
            super::parse_command("abc&"),
            Ok((
                "",
                super::Command {
                    name: "abc",
                    pipe: false,
                    background: true,
                    input_file: None,
                    output_file: None,
                    parameters: vec![]
                }
            ))
        );

        assert_eq!(
            super::parse_command("abc"),
            Ok((
                "",
                super::Command {
                    name: "abc",
                    pipe: false,
                    background: false,
                    input_file: None,
                    output_file: None,
                    parameters: vec![]
                }
            ))
        );

        assert_eq!(
            super::parse_command("abc x y \"n m\" 's t'&"),
            Ok((
                "",
                super::Command {
                    name: "abc",
                    pipe: false,
                    background: true,
                    input_file: None,
                    output_file: None,
                    parameters: vec!["x", "y", "n m", "s t"]
                }
            ))
        );

        // leading a trailing whitespace
        assert_eq!(
            super::parse_command("\tabc x y &   "),
            Ok((
                "",
                super::Command {
                    name: "abc",
                    pipe: false,
                    background: true,
                    input_file: None,
                    output_file: None,
                    parameters: vec!["x", "y"]
                }
            ))
        );
    }

    #[test]
    fn test_parse_command_with_redirect() {
        assert_eq!(
            super::parse_command("abc < input"),
            Ok((
                "",
                super::Command {
                    name: "abc",
                    pipe: false,
                    background: false,
                    input_file: Some(super::InputRedirect {
                        filename: "input",
                        file_descriptor: 0
                    }),
                    output_file: None,
                    parameters: vec![]
                }
            ))
        );

        assert_eq!(
            super::parse_command("abc > output"),
            Ok((
                "",
                super::Command {
                    name: "abc",
                    pipe: false,
                    background: false,
                    input_file: None,
                    output_file: Some(super::OutputRedirect {
                        filename: "output",
                        append: false,
                        file_descriptor: 1
                    }),
                    parameters: vec![]
                }
            ))
        );

        assert_eq!(
            super::parse_command("abc >> output"),
            Ok((
                "",
                super::Command {
                    name: "abc",
                    pipe: false,
                    background: false,
                    input_file: None,
                    output_file: Some(super::OutputRedirect {
                        filename: "output",
                        append: true,
                        file_descriptor: 1
                    }),
                    parameters: vec![]
                }
            ))
        );

        assert_eq!(
            super::parse_command("abc 2> erroroutput"),
            Ok((
                "",
                super::Command {
                    name: "abc",
                    pipe: false,
                    background: false,
                    input_file: None,
                    output_file: Some(super::OutputRedirect {
                        filename: "erroroutput",
                        append: false,
                        file_descriptor: 2
                    }),
                    parameters: vec![]
                }
            ))
        );

        assert_eq!(
            super::parse_command("abc > output &"),
            Ok((
                "",
                super::Command {
                    name: "abc",
                    pipe: false,
                    background: true,
                    input_file: None,
                    output_file: Some(super::OutputRedirect {
                        filename: "output",
                        append: false,
                        file_descriptor: 1
                    }),
                    parameters: vec![]
                }
            ))
        );

        assert_eq!(
            super::parse_command("abc < input > output"),
            Ok((
                "",
                super::Command {
                    name: "abc",
                    pipe: false,
                    background: false,
                    input_file: Some(super::InputRedirect {
                        filename: "input",
                        file_descriptor: 0
                    }),
                    output_file: Some(super::OutputRedirect {
                        filename: "output",
                        append: false,
                        file_descriptor: 1
                    }),
                    parameters: vec![]
                }
            ))
        );
    }

    #[test]
    fn test_parse_quoted_double_quote() {
        assert_eq!(
            super::parse_command("a\\\"bc \"x y\""),
            Ok((
                "",
                super::Command {
                    name: "a\\\"bc",
                    pipe: false,
                    background: false,
                    input_file: None,
                    output_file: None,
                    parameters: vec!["x y"]
                }
            ))
        );

        assert_eq!(
            super::parse_command("cmd a\\\"b"),
            Ok((
                "",
                super::Command {
                    name: "cmd",
                    pipe: false,
                    background: false,
                    input_file: None,
                    output_file: None,
                    parameters: vec!["a\\\"b"]
                }
            ))
        );
    }
}
