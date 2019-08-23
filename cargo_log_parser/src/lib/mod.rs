use nom::{
    bytes::{streaming::tag, streaming::take_till, streaming::take_till1},
    character::streaming::line_ending,
    combinator::map,
    sequence::{delimited, tuple},
    IResult,
};

#[derive(Debug, PartialEq, Eq)]
pub struct CrateWithError {
    pub name: String,
}

impl From<&[u8]> for CrateWithError {
    fn from(name: &[u8]) -> Self {
        CrateWithError {
            name: String::from_utf8(name.to_owned()).unwrap(),
        }
    }
}

fn quoted_name(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let backtick = || tag(b"`");
    delimited(backtick(), take_till1(|b| b == b'`'), backtick())(input)
}

pub fn line_with_error(input: &[u8]) -> IResult<&[u8], CrateWithError> {
    let till_backtick = || take_till1(|b| b == b'`');

    map(
        tuple((
            tag(b"error:"),
            till_backtick(),
            quoted_name,
            take_till(|b| b == b'\n' || b == b'\r'),
            line_ending,
        )),
        |(_, _, name, _, _)| CrateWithError::from(name),
    )(input)
}

#[cfg(test)]
mod tests;
