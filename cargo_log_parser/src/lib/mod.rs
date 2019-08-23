use nom::{
    branch::alt,
    bytes::{streaming::tag, streaming::take_till, streaming::take_till1},
    character::streaming::line_ending,
    combinator::{complete, map, map_parser, map_res, opt},
    multi::fold_many0,
    sequence::{delimited, terminated, tuple},
    IResult,
};
use std::convert::TryFrom;
use std::string::FromUtf8Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrateWithError {
    pub name: String,
}

impl TryFrom<&[u8]> for CrateWithError {
    type Error = FromUtf8Error;

    fn try_from(name: &[u8]) -> Result<Self, Self::Error> {
        Ok(CrateWithError {
            name: String::from_utf8(name.to_owned())?,
        })
    }
}

fn quoted_name(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let backtick = || tag(b"`");
    delimited(backtick(), take_till1(|b| b == b'`'), backtick())(input)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Line {
    Other,
    Error(CrateWithError),
}

impl From<CrateWithError> for Line {
    fn from(c: CrateWithError) -> Self {
        Line::Error(c)
    }
}

impl From<&[u8]> for Line {
    fn from(_: &[u8]) -> Self {
        Line::Other
    }
}

pub fn parse_errors(input: &[u8]) -> IResult<&[u8], Vec<CrateWithError>> {
    fold_many0(
        |i: &[u8]| {
            if i.len() == 0 {
                return Err(nom::Err::Error((i, nom::error::ErrorKind::Eof)));
            }
            opt(alt((
                map(complete(line_with_error), Line::from),
                map(line_without_ending, Line::from),
            )))(i)
        },
        Vec::new(),
        |mut acc, c| {
            if let Some(Line::Error(c)) = c {
                acc.push(c);
            }
            acc
        },
    )(input)
}

fn is_newline(b: u8) -> bool {
    b == b'\n' || b == b'\r'
}

fn line_without_ending(input: &[u8]) -> IResult<&[u8], &[u8]> {
    terminated(take_till(is_newline), line_ending)(input)
}

pub fn line_with_error(input: &[u8]) -> IResult<&[u8], CrateWithError> {
    map_parser(
        line_without_ending,
        map_res(
            tuple((tag(b"error:"), take_till1(|b| b == b'`'), quoted_name)),
            |(_, _, name)| CrateWithError::try_from(name),
        ),
    )(input)
}

#[cfg(test)]
mod tests;
