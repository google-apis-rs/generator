use nom::branch::alt;
use nom::combinator::opt;
use nom::multi::many0;
use nom::{
    bytes::{streaming::tag, streaming::take_till, streaming::take_till1},
    character::streaming::line_ending,
    combinator::map_res,
    sequence::terminated,
    sequence::{delimited, tuple},
    IResult,
};
use std::convert::TryFrom;
use std::string::FromUtf8Error;

#[derive(Debug, PartialEq, Eq)]
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

pub fn parse_errors(mut input: &[u8]) -> IResult<&[u8], Vec<CrateWithError>> {
    let mut out = Vec::new();
    loop {
        match line_with_error(input) {
            Ok((i, r)) => {
                out.push(r);
                input = i;
            }
            Err(nom::Err::Error(_)) => unimplemented!(),
            Err(e) => return Err(e),
        }
    }
    //    let (i, e) = line_with_error(input)?;
    //    many0(opt(alt((line_with_error, line))))(input)
    //        .map(|v| v.into_iter().filter_map(|v| v).collect())
    Ok((input, out))
}

pub fn line(input: &[u8]) -> IResult<&[u8], &[u8]> {
    terminated(take_till(|b| b == b'\n' || b == b'\r'), line_ending)(input)
}

pub fn line_with_error(input: &[u8]) -> IResult<&[u8], CrateWithError> {
    let take_till_backtick = || take_till1(|b| b == b'`');
    let take_till_newline = take_till(|b| b == b'\n' || b == b'\r');

    map_res(
        tuple((
            tag(b"error:"),
            take_till_backtick(),
            quoted_name,
            take_till_newline,
            line_ending,
        )),
        |(_, _, name, _, _)| CrateWithError::try_from(name),
    )(input)
}

#[cfg(test)]
mod tests;
