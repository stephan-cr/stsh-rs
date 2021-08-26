// https://github.com/Geal/nom/blob/master/doc/choosing_a_combinator.md

use nom::{
    branch::alt,
    bytes::complete::take_while1,
    character::complete::{alphanumeric1, char, space0, space1},
    multi::separated_list0,
    sequence::delimited,
    IResult,
};

#[derive(Debug, PartialEq)]
pub(crate) struct Command<'a> {
    name: &'a str,
    pipe: bool,
    background: bool,
    parameters: Vec<&'a str>,
}

pub(crate) fn parse(input: &str) -> IResult<&str, Vec<Command>> {
    // let pipe = char('|');
    // let input_redirect = char('<');
    // let output_redirect = char('>');
    // let error_redirect = tag("2>");
    // let background = char('&');
    // let sep = char(';');
    // let escape = char('\\');

    let (i, command) = parse_background_command(input)?;

    Ok((i, vec![command]))
}

fn is_allowed_in_double_quotes(chr: char) -> bool {
    chr.is_alphanumeric() || chr.is_whitespace()
}

fn parse_background_command(input: &str) -> IResult<&str, Command> {
    let unquoted_param = alphanumeric1;
    let single_quote = char('\'');
    let double_quote = char('"');
    let param_within_quotes = take_while1(is_allowed_in_double_quotes);
    let quoted_param = alt((
        delimited(&double_quote, &param_within_quotes, &double_quote),
        delimited(&single_quote, &param_within_quotes, &single_quote),
    ));

    let (i, _) = space0(input)?; // ignore all leading space
    let (i, command_name) = alphanumeric1(i)?;
    let (i, _) = space0(i)?;
    let (i, parameters) = separated_list0(space1, alt((quoted_param, unquoted_param)))(i)?;
    let (i, _) = space0(i)?;
    let (i, _) = char('&')(i)?;

    Ok((
        i,
        Command {
            name: command_name,
            pipe: false,
            background: true,
            parameters: parameters,
        },
    ))
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_parse_command() {
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
    fn test_parse_background_command() {
        assert_eq!(
            super::parse_background_command("abc &"),
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
            super::parse_background_command("abc&"),
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

        assert!(super::parse_background_command("abc").is_err());

        assert_eq!(
            super::parse_background_command("abc x y \"n m\" 's t'&"),
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
    }
}
