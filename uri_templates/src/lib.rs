//! A subset of the uri templates syntax (RFC 6570) necessary to parse and
//create url paths for google services. Google service use uri templates for
//defining paths, they do not use the full capabilities of uri templates. They
//also ensure that any parameter interpolated into a path is required. This
//allows for a much simpler implementation to generate the paths that only
//needs to handle simple and raw expansions of required fields.

use std::collections::HashMap;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct GooglePathTemplate<'a> {
    pub nodes: Vec<GooglePathAstNode<'a>>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum GooglePathAstNode<'a> {
    Lit(&'a str),
    Var {
        var_name: &'a str,
        expansion_style: ExpansionStyle,
    },
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum ExpansionStyle {
    Simple,
    Reserved,
}

impl<'a> GooglePathAstNode<'a> {
    fn from_parser_ast_node(
        n: parser::TemplateAstNode<'a>,
    ) -> Result<GooglePathAstNode<'a>, String> {
        Ok(match n {
            parser::TemplateAstNode::Lit(lit) => GooglePathAstNode::Lit(lit),
            parser::TemplateAstNode::Expr(expr) => {
                let expansion_style = match expr.operator {
                    parser::Operator::Simple => ExpansionStyle::Simple,
                    parser::Operator::Reserved => ExpansionStyle::Reserved,
                    x => return Err(format!("Unsupported uri template operator: {:?}", x)),
                };
                if expr.var_spec_list.len() != 1 {
                    return Err(format!(
                        "Unsupported number of variables in uri template varspec: {}",
                        expr.var_spec_list.len()
                    ));
                }
                let var_name = &expr.var_spec_list[0].var_name;
                GooglePathAstNode::Var {
                    var_name,
                    expansion_style,
                }
            }
        })
    }
}

impl<'a> GooglePathTemplate<'a> {
    pub fn new(tmpl: &str) -> Result<GooglePathTemplate, String> {
        use parser::{Modifier, Operator, TemplateAstNode};
        let nodes = parser::ast_nodes(tmpl)
            .ok_or_else(|| "Failed to parse uri template".to_owned())?
            .into_iter()
            .map(GooglePathAstNode::from_parser_ast_node)
            .collect::<Result<Vec<GooglePathAstNode>, String>>()?;
        Ok(GooglePathTemplate { nodes })
    }
}

mod parser {
    ///! A parser for the full uri templates syntax. Most of the syntax is
    ///unused and not supported by the GooglePathTemplate, but parsing the full
    ///syntax allows checking to verify none of the templates use unsupported
    ///features.
    use nom::{
        branch::alt,
        bytes::complete::{tag, take_while1, take_while_m_n},
        character::complete::digit1,
        combinator::{all_consuming, map, map_res, opt, recognize},
        multi::{many0, many1, separated_nonempty_list},
        sequence::{delimited, tuple},
        IResult,
    };
    use percent_encoding::AsciiSet;

    #[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
    pub(super) enum TemplateAstNode<'a> {
        Lit(&'a str),
        Expr(Expression<'a>),
    }

    #[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
    pub(super) struct Expression<'a> {
        pub(super) operator: Operator,
        pub(super) var_spec_list: Vec<VarSpec<'a>>,
    }

    #[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
    pub(super) struct VarSpec<'a> {
        pub(super) var_name: &'a str,
        pub(super) modifier: Modifier,
    }

    #[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd)]
    pub(super) enum Operator {
        Simple,
        Reserved,
        Fragment,
        Label,
        PathSegment,
        PathParameter,
        QueryExpansion,
        QueryContinuation,
    }

    #[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd)]
    pub(super) enum Modifier {
        NoModifier,
        Prefix(u16),
        Explode,
    }

    pub(super) fn ast_nodes(input: &str) -> Option<Vec<TemplateAstNode>> {
        Some(all_consuming(many0(template_ast_node))(input).ok()?.1)
    }

    fn operator(input: &str) -> IResult<&str, Operator> {
        fn tag_to_operator(
            op_prefix: char,
            op: Operator,
        ) -> impl Fn(&str) -> IResult<&str, Operator> {
            use nom::character::complete::char;
            move |input: &str| -> IResult<&str, Operator> { map(char(op_prefix), |_| op)(input) }
        }

        map(
            opt(alt((
                tag_to_operator('+', Operator::Reserved),
                tag_to_operator('#', Operator::Fragment),
                tag_to_operator('.', Operator::Label),
                tag_to_operator('/', Operator::PathSegment),
                tag_to_operator(';', Operator::PathParameter),
                tag_to_operator('?', Operator::QueryExpansion),
                tag_to_operator('&', Operator::QueryContinuation),
            ))),
            |x| x.unwrap_or(Operator::Simple),
        )(input)
    }

    fn modifier(input: &str) -> IResult<&str, Modifier> {
        map(opt(alt((prefix_mod, explode_mod))), |x| {
            x.unwrap_or(Modifier::NoModifier)
        })(input)
    }

    fn prefix_mod(input: &str) -> IResult<&str, Modifier> {
        map(
            tuple((
                tag(":"),
                map_res(digit1, |s: &str| s.parse().map(Modifier::Prefix)),
            )),
            |(_, prefix_modifier)| prefix_modifier,
        )(input)
    }

    fn explode_mod(input: &str) -> IResult<&str, Modifier> {
        map(tag("*"), |_| Modifier::Explode)(input)
    }

    fn percent_encoded(input: &str) -> IResult<&str, &str> {
        recognize(tuple((
            nom::character::complete::char('%'),
            take_while_m_n(2, 2, |c: char| c.is_ascii_hexdigit()),
        )))(input)
    }

    fn var_name(input: &str) -> IResult<&str, &str> {
        recognize(many1(map(varchar, |_| ())))(input)
    }

    fn varchar(input: &str) -> IResult<&str, &str> {
        alt((recognize(take_while1(is_valid_varchar)), percent_encoded))(input)
    }

    fn is_reserved_char(c: char) -> bool {
        false
            || c == ':'
            || c == '/'
            || c == '?'
            || c == '#'
            || c == '['
            || c == ']'
            || c == '@'
            || c == '!'
            || c == '$'
            || c == '&'
            || c == '\''
            || c == '('
            || c == ')'
            || c == '*'
            || c == '+'
            || c == ','
            || c == ';'
            || c == '='
    }

    fn encode_unreserved(input: &str) -> percent_encoding::PercentEncode {
        use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
        const UNRESERVED: &AsciiSet = &CONTROLS
            .add(b' ')
            .add(b'"')
            .add(b'\'')
            .add(b'<')
            .add(b'>')
            .add(b'\\')
            .add(b'^')
            .add(b'`')
            .add(b'{')
            .add(b'|')
            .add(b'}');
        utf8_percent_encode(input, &UNRESERVED)
    }

    fn encode_literal(input: &str) -> percent_encoding::PercentEncode {
        use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
        utf8_percent_encode(input, &CONTROLS)
    }

    fn is_unreserved_char(c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '_' || c == '~'
    }

    fn is_valid_varchar(c: char) -> bool {
        c.is_ascii_alphanumeric() || c.is_ascii_digit() || c == '_' || c == '.'
    }

    fn var_spec(input: &str) -> IResult<&str, VarSpec> {
        map(tuple((var_name, modifier)), |(var_name, modifier)| {
            VarSpec { var_name, modifier }
        })(input)
    }

    fn expression(input: &str) -> IResult<&str, Expression> {
        use nom::character::complete::char;
        delimited(
            char('{'),
            map(
                tuple((operator, separated_nonempty_list(char(','), var_spec))),
                |(operator, var_spec_list)| Expression {
                    operator,
                    var_spec_list,
                },
            ),
            char('}'),
        )(input)
    }

    fn template_ast_node(input: &str) -> IResult<&str, TemplateAstNode> {
        alt((
            map(take_while1(|c| c != '{'), |lit| TemplateAstNode::Lit(lit)),
            map(expression, |expr| TemplateAstNode::Expr(expr)),
        ))(input)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_modifier() {
            assert_eq!(modifier(":100"), Ok(("", Modifier::Prefix(100))));
            assert_eq!(modifier(":3"), Ok(("", Modifier::Prefix(3))));
            assert_eq!(modifier(":9999"), Ok(("", Modifier::Prefix(9999))));
            assert_eq!(modifier("*"), Ok(("", Modifier::Explode)));
            assert_eq!(modifier(":3*"), Ok(("*", Modifier::Prefix(3))));
        }

        #[test]
        fn test_opt() {
            assert_eq!(operator("foo"), Ok(("foo", Operator::Simple)));
            assert_eq!(operator("+foo"), Ok(("foo", Operator::Reserved)));
            assert_eq!(operator("#foo"), Ok(("foo", Operator::Fragment)));
            assert_eq!(operator(".foo"), Ok(("foo", Operator::Label)));
            assert_eq!(operator("/foo"), Ok(("foo", Operator::PathSegment)));
            assert_eq!(operator(";foo"), Ok(("foo", Operator::PathParameter)));
            assert_eq!(operator("?foo"), Ok(("foo", Operator::QueryExpansion)));
            assert_eq!(operator("&foo"), Ok(("foo", Operator::QueryContinuation)));
        }

        #[test]
        fn test_var_name() {
            assert_eq!(var_name("foo"), Ok(("", "foo")));
            assert_eq!(var_name("foo_bar"), Ok(("", "foo_bar")));
            assert_eq!(var_name("1foo_bar"), Ok(("", "1foo_bar")));
            assert_eq!(var_name("foo%2a"), Ok(("", "foo%2a")));
            assert_eq!(var_name("foo%FF"), Ok(("", "foo%FF")));
            assert_eq!(var_name("foo%ZF"), Ok(("%ZF", "foo")));
            assert_eq!(var_name("foo\nbar"), Ok(("\nbar", "foo")));
            assert!(var_name("").is_err());
        }

        #[test]
        fn test_varchar() {
            assert_eq!(varchar("foo"), Ok(("", "foo")));
            assert_eq!(varchar("foo_bar"), Ok(("", "foo_bar")));
            assert_eq!(varchar("1foo_bar"), Ok(("", "1foo_bar")));
            assert_eq!(varchar("foo.a"), Ok(("", "foo.a")));
            assert_eq!(varchar("%FF"), Ok(("", "%FF")));
            assert_eq!(varchar("foo%FF"), Ok(("%FF", "foo")));
            assert_eq!(varchar("%FFfoo%ZF"), Ok(("foo%ZF", "%FF")));
            assert_eq!(varchar("foo\nbar"), Ok(("\nbar", "foo")));
            assert!(varchar("").is_err());
        }

        #[test]
        fn test_var_spec() {
            assert_eq!(
                var_spec("foo:3"),
                Ok((
                    "",
                    VarSpec {
                        var_name: "foo",
                        modifier: Modifier::Prefix(3),
                    }
                ))
            );
            assert_eq!(
                var_spec("foo*"),
                Ok((
                    "",
                    VarSpec {
                        var_name: "foo",
                        modifier: Modifier::Explode,
                    }
                ))
            );
            assert_eq!(
                var_spec("foo:3*"),
                Ok((
                    "*",
                    VarSpec {
                        var_name: "foo",
                        modifier: Modifier::Prefix(3),
                    }
                ))
            );
        }

        #[test]
        fn test_expression() {
            assert_eq!(
                expression("{foo:3}"),
                Ok((
                    "",
                    Expression {
                        operator: Operator::Simple,
                        var_spec_list: vec![VarSpec {
                            var_name: "foo",
                            modifier: Modifier::Prefix(3),
                        }],
                    }
                ))
            );
            assert_eq!(
                expression("{+foo*}"),
                Ok((
                    "",
                    Expression {
                        operator: Operator::Reserved,
                        var_spec_list: vec![VarSpec {
                            var_name: "foo",
                            modifier: Modifier::Explode,
                        }],
                    }
                ))
            );
            assert_eq!(
                expression("{#foo%2a:9999}remaining"),
                Ok((
                    "remaining",
                    Expression {
                        operator: Operator::Fragment,
                        var_spec_list: vec![VarSpec {
                            var_name: "foo%2a",
                            modifier: Modifier::Prefix(9999),
                        }],
                    }
                ))
            );
            assert!(expression("{+foo:3*}").is_err());
            assert!(expression("{+foo%2z:3}").is_err());
        }

        #[test]
        fn test_percent_encoded() {
            assert_eq!(percent_encoded("%20"), Ok(("", "%20")));
            assert_eq!(percent_encoded("%aaa"), Ok(("a", "%aa")));
            assert!(percent_encoded("foo").is_err());
            assert!(percent_encoded("%2").is_err());
        }

        #[test]
        fn test_template_ast_node() {
            assert_eq!(
                template_ast_node("foobar"),
                Ok(("", TemplateAstNode::Lit("foobar")))
            );
            assert_eq!(
                template_ast_node("{foobar}"),
                Ok((
                    "",
                    TemplateAstNode::Expr(Expression {
                        operator: Operator::Simple,
                        var_spec_list: vec![VarSpec {
                            var_name: "foobar",
                            modifier: Modifier::NoModifier,
                        }],
                    })
                ))
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template() {
        assert_eq!(
            GooglePathTemplate::new("foobar"),
            Ok(GooglePathTemplate {
                nodes: vec![GooglePathAstNode::Lit("foobar"),]
            })
        );
        assert_eq!(
            GooglePathTemplate::new("{project}/managedZones/{+managedZone}/changes"),
            Ok(GooglePathTemplate {
                nodes: vec![
                    GooglePathAstNode::Var {
                        var_name: "project",
                        expansion_style: ExpansionStyle::Simple
                    },
                    GooglePathAstNode::Lit("/managedZones/"),
                    GooglePathAstNode::Var {
                        var_name: "managedZone",
                        expansion_style: ExpansionStyle::Reserved
                    },
                    GooglePathAstNode::Lit("/changes"),
                ],
            })
        );
        /// output.extend(percent_encoded(self.project));
        /// output.push_str("/managedZones/");
        /// output.extend(percent_encoded(self.managed_zone));
        /// output.push_str("/changes");
    }
}


