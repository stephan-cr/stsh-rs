// https://github.com/Geal/nom/blob/master/doc/choosing_a_combinator.md
// https://github.com/bminor/bash/blob/master/parse.y

use nom::{
    branch::alt,
    bytes::complete::{escaped, take_while1},
    character::complete::{alphanumeric1, char, space0, space1},
    combinator::opt,
    multi::separated_list0,
    sequence::delimited,
    IResult,
};

#[derive(Debug, PartialEq)]
pub(crate) struct Command<'a> {
    pub name: &'a str,
    pipe: bool,
    background: bool,
    pub parameters: Vec<&'a str>,
}

pub(crate) fn parse(input: &str) -> IResult<&str, Vec<Command>> {
    // let pipe = char('|');
    // let input_redirect = char('<');
    // let output_redirect = char('>');
    // let output_append = tag(">>");
    // let error_redirect = tag("2>"); => it should be [n]>, where n can be any file descriptor, I guess
    // let sep = char(';');
    // let and = tag("&&");
    // let or = tag("||");
    // let subshell = delimited(char('('), ..., char(')'));

    let (i, command) = parse_command(input)?;

    Ok((i, vec![command]))
}

fn is_allowed_in_double_quotes(chr: char) -> bool {
    chr.is_alphanumeric() || chr.is_whitespace()
}

fn parse_command(input: &str) -> IResult<&str, Command> {
    let unquoted_param = alphanumeric1;
    let single_quote = char('\'');
    let double_quote = char('"');
    let param_within_quotes = take_while1(is_allowed_in_double_quotes);
    let quoted_param = alt((
        delimited(&double_quote, &param_within_quotes, &double_quote),
        delimited(&single_quote, &param_within_quotes, &single_quote),
    ));
    let background = char('&');

    let (i, _) = space0(input)?; // ignore all leading whitespace
    let (i, command_name) = escaped(alphanumeric1, '\\', &double_quote)(i)?;
    todo!("allow / in command names");
    let (i, _) = space0(i)?;
    let (i, parameters) = separated_list0(space1, alt((quoted_param, unquoted_param)))(i)?;
    let (i, _) = space0(i)?;
    let (i, background) = opt(background)(i)?;
    let (i, _) = space0(i)?; // ignore all trailing whitespace

    Ok((
        i,
        Command {
            name: command_name,
            pipe: false,
            background: background.is_some(),
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
                    parameters: vec!["x", "y"]
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
                    parameters: vec!["x y"]
                }
            ))
        );
    }
}
