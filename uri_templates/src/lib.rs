use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1, take_while_m_n},
    character::complete::digit1,
    combinator::{all_consuming, map, map_res, opt, recognize},
    multi::{many0, many1, separated_nonempty_list},
    sequence::{delimited, tuple},
    IResult,
};
use std::collections::HashMap;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Template<'a> {
    pub nodes: Vec<TemplateNode<'a>>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum TemplateNode<'a> {
    Lit(&'a str),
    Expr(Expression<'a>),
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Expression<'a> {
    operator: Option<Operator>,
    var_spec_list: Vec<VarSpec<'a>>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct VarSpec<'a> {
    var_name: &'a str,
    modifier: Option<Modifier>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Operator {
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
    Prefix(u16),
    Explode,
}

impl<'a> Template<'a> {
    pub fn new(tmpl: &str) -> Option<Template> {
        Some(Template {
            nodes: all_consuming(many0(template_node))(tmpl).ok()?.1,
        })
    }

    pub fn render(&self, variables: impl TemplateVariables) -> String {
        let mut output = String::new();
        for node in &self.nodes {
            match node {
                TemplateNode::Lit(lit) => output.push_str(lit),
                TemplateNode::Expr(expr) => {
                    variables.append_expression(expr)

                    for var_spec in &expr.var_spec_list {
                        output.push_str(&variables.get(var_spec.var_name).unwrap());
                    }
                }
            }
        }
        output
    }
}

trait TemplateVariables {
    // Append to output the expansion of the variable `var_name` according to the provided operator and modifier.
    fn append_var_expansion(&self, var_name: &str, operator: Option<Operator>, modifier: Option<Modifier>, output: &mut String);
}

impl TemplateVariables for HashMap<String, V>
where
    V: TemplateVariable
{
    fn append_expansion(&self, expr: &Expression, output: &mut String) {
        let any_defined = expr.var_spec_list.iter().any(|var_spec| self.contains(&var_spec.var_name));
        let value = self.get(varvar_name);
        match expr.operator {
            None => { 
                value.append_var(expr.operator, expr.modifier, output);
            },
            Some(Operator::Reserved) => {
                value.append_var(expr.operator, expr.modifier, output);
            },
            Some(Operator::Fragment) => {},
            Some(Operator::Label) => {},
            Some(Operator::PathSegment) => {},
            Some(Operator::PathParameter) => {},
            Some(Operator::QueryExpansion) => {},
            Some(Operator::QueryContinuation) => {},
        }

        match (value, operator, modifier) {
            (None, _, Some(QueryExpansion))

        }
        let value = match self.get(var_name) {
            Some(value) => value,
            None => return,
        };
    }
}

trait TemplateVariable<'a> {
    type Output: Iterator<Item=&'a str>;

    fn append_var(
        &self,
        operator: Option<Operator>,
        modifier: Option<Modifier>,
    ) -> Self::Output;
}

impl TemplateVariable<'a> for String {
    type Output = percent_encoding::PercentEncode<'a>;

    fn append_var(
        &self,
        operator: Option<Operator>,
        modifier: Option<Modifier>,
    ) -> Self::Output {
        let input = if let Some(Modifier::Prefix(len)) = modifier {
            let end_idx = self.char_indices().map(|(idx, _char)| idx).skip(len.into()).next().unwrap_or(input.len());
            &self[..end_idx]
        } else {
            self
        }
        encode_unreserved(self)
    }
}

fn operator(input: &str) -> IResult<&str, Operator> {
    fn tag_to_operator(op_prefix: char, op: Operator) -> impl Fn(&str) -> IResult<&str, Operator> {
        use nom::character::complete::char;
        move |input: &str| -> IResult<&str, Operator> { map(char(op_prefix), |_| op)(input) }
    }

    alt((
        tag_to_operator('+', Operator::Reserved),
        tag_to_operator('#', Operator::Fragment),
        tag_to_operator('.', Operator::Label),
        tag_to_operator('/', Operator::PathSegment),
        tag_to_operator(';', Operator::PathParameter),
        tag_to_operator('?', Operator::QueryExpansion),
        tag_to_operator('&', Operator::QueryContinuation),
    ))(input)
}

fn modifier(input: &str) -> IResult<&str, Modifier> {
    alt((prefix_mod, explode_mod))(input)
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
    use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};
    const UNRESERVED: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'\'').add(b'<').add(b'>').add(b'\\').add(b'^').add(b'`').add(b'{').add(b'|').add(b'}');
    utf8_percent_encode(input, &UNRESERVED)
}

fn is_unreserved_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '_' || c == '~'
}

fn is_valid_varchar(c: char) -> bool {
    c.is_ascii_alphanumeric() || c.is_ascii_digit() || c == '_' || c == '.'
}

fn var_spec(input: &str) -> IResult<&str, VarSpec> {
    map(tuple((var_name, opt(modifier))), |(var_name, modifier)| {
        VarSpec { var_name, modifier }
    })(input)
}

fn expression(input: &str) -> IResult<&str, Expression> {
    use nom::character::complete::char;
    delimited(
        char('{'),
        map(
            tuple((opt(operator), separated_nonempty_list(char(','), var_spec))),
            |(operator, var_spec_list)| Expression {
                operator,
                var_spec_list,
            },
        ),
        char('}'),
    )(input)
}

fn template_node(input: &str) -> IResult<&str, TemplateNode> {
    alt((
        map(take_while1(|c| c != '{'), |lit| TemplateNode::Lit(lit)),
        map(expression, |expr| TemplateNode::Expr(expr)),
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
        assert!(operator("foo").is_err());
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
                    modifier: Some(Modifier::Prefix(3))
                }
            ))
        );
        assert_eq!(
            var_spec("foo*"),
            Ok((
                "",
                VarSpec {
                    var_name: "foo",
                    modifier: Some(Modifier::Explode)
                }
            ))
        );
        assert_eq!(
            var_spec("foo:3*"),
            Ok((
                "*",
                VarSpec {
                    var_name: "foo",
                    modifier: Some(Modifier::Prefix(3))
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
                    operator: None,
                    var_spec_list: vec![VarSpec {
                        var_name: "foo",
                        modifier: Some(Modifier::Prefix(3)),
                    }],
                }
            ))
        );
        assert_eq!(
            expression("{+foo*}"),
            Ok((
                "",
                Expression {
                    operator: Some(Operator::Reserved),
                    var_spec_list: vec![VarSpec {
                        var_name: "foo",
                        modifier: Some(Modifier::Explode),
                    }],
                }
            ))
        );
        assert_eq!(
            expression("{#foo%2a:9999}remaining"),
            Ok((
                "remaining",
                Expression {
                    operator: Some(Operator::Fragment),
                    var_spec_list: vec![VarSpec {
                        var_name: "foo%2a",
                        modifier: Some(Modifier::Prefix(9999)),
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
    fn test_template_node() {
        assert_eq!(
            template_node("foobar"),
            Ok(("", TemplateNode::Lit("foobar")))
        );
        assert_eq!(
            template_node("{foobar}"),
            Ok((
                "",
                TemplateNode::Expr(Expression {
                    operator: None,
                    var_spec_list: vec![VarSpec {
                        var_name: "foobar",
                        modifier: None,
                    }],
                })
            ))
        );
    }

    #[test]
    fn test_template() {
        assert_eq!(
            Template::new("foobar"),
            Some(Template {
                nodes: vec![TemplateNode::Lit("foobar"),]
            })
        );
        assert_eq!(
            Template::new("/{dir:2}/{+dir}{?foobar*}"),
            Some(Template {
                nodes: vec![
                    TemplateNode::Lit("/"),
                    TemplateNode::Expr(Expression {
                        operator: None,
                        var_spec_list: vec![VarSpec {
                            var_name: "dir",
                            modifier: Some(Modifier::Prefix(2)),
                        }],
                    }),
                    TemplateNode::Lit("/"),
                    TemplateNode::Expr(Expression {
                        operator: Some(Operator::Reserved),
                        var_spec_list: vec![VarSpec {
                            var_name: "dir",
                            modifier: None,
                        }],
                    }),
                    TemplateNode::Expr(Expression {
                        operator: Some(Operator::QueryExpansion),
                        var_spec_list: vec![VarSpec {
                            var_name: "foobar",
                            modifier: Some(Modifier::Explode),
                        }],
                    }),
                ],
            })
        );
    }

    #[test]
    fn test_render() {
        let tmpl = Template::new("/{dir:2}/{+dir}{?foobar*}").unwrap();
        let variables: HashMap<String, String> = vec![
            ("dir".to_owned(), "username/nested/dir".to_owned()),
            ("foobar".to_owned(), "?key=value".to_owned()),
        ]
        .into_iter()
        .collect();
        assert_eq!("us/username/nested/dir/?key=value", tmpl.render(&variables));
    }
}
