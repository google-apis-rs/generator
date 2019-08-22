///! A parser for the full uri templates syntax as described in RFC 6570.
use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1, take_while_m_n},
    character::complete::digit1,
    combinator::{all_consuming, map, map_res, opt, recognize},
    multi::{many0, many1, separated_nonempty_list},
    sequence::{delimited, tuple},
    IResult,
};

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum AstNode<'a> {
    Lit(&'a str),
    Expr(Expression<'a>),
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Expression<'a> {
    pub operator: Operator,
    pub var_spec_list: Vec<VarSpec<'a>>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct VarSpec<'a> {
    pub var_name: &'a str,
    pub modifier: Modifier,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Operator {
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
pub enum Modifier {
    NoModifier,
    Prefix(u16),
    Explode,
}

pub fn ast_nodes(input: &str) -> Option<Vec<AstNode>> {
    Some(all_consuming(many0(template_ast_node))(input).ok()?.1)
}

fn operator(input: &str) -> IResult<&str, Operator> {
    fn tag_to_operator(op_prefix: char, op: Operator) -> impl Fn(&str) -> IResult<&str, Operator> {
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

fn template_ast_node(input: &str) -> IResult<&str, AstNode> {
    alt((
        map(take_while1(|c| c != '{'), |lit| AstNode::Lit(lit)),
        map(expression, |expr| AstNode::Expr(expr)),
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
        assert_eq!(
            expression("{foo:3,bar:1}"),
            Ok((
                "",
                Expression {
                    operator: Operator::Simple,
                    var_spec_list: vec![
                        VarSpec {
                            var_name: "foo",
                            modifier: Modifier::Prefix(3),
                        },
                        VarSpec {
                            var_name: "bar",
                            modifier: Modifier::Prefix(1)
                        }
                    ],
                }
            ))
        );
        assert!(expression("{+foo:3*}").is_err());
        assert!(expression("{+foo%2z:3}").is_err());
        assert!(expression("{+foo:3,*bar:1}").is_err());
        assert!(expression("{+foo:3, bar:1}").is_err());
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
            Ok(("", AstNode::Lit("foobar")))
        );
        assert_eq!(
            template_ast_node("{foobar}"),
            Ok((
                "",
                AstNode::Expr(Expression {
                    operator: Operator::Simple,
                    var_spec_list: vec![VarSpec {
                        var_name: "foobar",
                        modifier: Modifier::NoModifier,
                    }],
                })
            ))
        );
    }

    #[test]
    fn test_ast_nodes() {
        use super::{AstNode::*, Modifier::*, Operator::*};
        assert_eq!(ast_nodes("hello"), Some(vec![Lit("hello")]));
        assert_eq!(ast_nodes(""), Some(vec![]));
        assert_eq!(
            ast_nodes("/{foo}/{bar,baz}/literal"),
            Some(vec![
                Lit("/"),
                Expr(Expression {
                    operator: Simple,
                    var_spec_list: vec![VarSpec {
                        var_name: "foo",
                        modifier: NoModifier
                    }]
                }),
                Lit("/"),
                Expr(Expression {
                    operator: Simple,
                    var_spec_list: vec![
                        VarSpec {
                            var_name: "bar",
                            modifier: NoModifier
                        },
                        VarSpec {
                            var_name: "baz",
                            modifier: NoModifier
                        }
                    ]
                }),
                Lit("/literal")
            ])
        );
    }
}
