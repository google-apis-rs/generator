// Path templates are uri templates as described in RFC 6570. However they are
// only used to define paths in google api's meaning the implementation can be
// simplified greatly if we only support the subset of the uri template syntax
// used by google apis. The defined subset is that only the simple {var} and
// reserved {+var} operator is supported, with no modifiers (prefix or explode).
// We use the uri_template_parser crate to parse the template into an AST,
// validate that the AST conforms to the subset supported in path templates, and
// then generate code to define the path based on the parameters in use by the
// method.
use uri_template_parser as parser;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub(crate) struct PathTemplate<'a> {
    nodes: Vec<PathAstNode<'a>>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub(crate) enum PathAstNode<'a> {
    Lit(&'a str),
    Var {
        var_name: &'a str,
        expansion_style: ExpansionStyle,
    },
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub(crate) enum ExpansionStyle {
    Simple,
    Reserved,
}

impl<'a> PathAstNode<'a> {
    fn from_parser_ast_node(n: parser::AstNode<'a>) -> Result<PathAstNode<'a>, String> {
        Ok(match n {
            parser::AstNode::Lit(lit) => PathAstNode::Lit(lit),
            parser::AstNode::Expr(expr) => {
                let expansion_style = match expr.operator {
                    parser::Operator::Simple => ExpansionStyle::Simple,
                    parser::Operator::Reserved => ExpansionStyle::Reserved,
                    x => {
                        eprintln!("unsupported uri template operator: {:?}", x);
                        ExpansionStyle::Simple
                    },
                    //x => return Err(format!("Unsupported uri template operator: {:?}", x)),
                };
                if expr.var_spec_list.len() != 1 {
                    return Err(format!(
                        "Unsupported number of variables in uri template varspec: {}",
                        expr.var_spec_list.len()
                    ));
                }
                let var_name = &expr.var_spec_list[0].var_name;
                PathAstNode::Var {
                    var_name,
                    expansion_style,
                }
            }
        })
    }
}

impl<'a> PathTemplate<'a> {
    pub(crate) fn new(tmpl: &str) -> Result<PathTemplate, String> {
        let nodes = parser::ast_nodes(tmpl)
            .ok_or_else(|| "Failed to parse uri template".to_owned())?
            .into_iter()
            .map(PathAstNode::from_parser_ast_node)
            .collect::<Result<Vec<PathAstNode>, String>>()?;
        Ok(PathTemplate { nodes })
    }

    pub(crate) fn nodes(&self) -> impl Iterator<Item = &PathAstNode> {
        self.nodes.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template() {
        assert_eq!(
            PathTemplate::new("foobar"),
            Ok(PathTemplate {
                nodes: vec![PathAstNode::Lit("foobar"),]
            })
        );
        assert_eq!(
            PathTemplate::new("{project}/managedZones/{+managedZone}/changes"),
            Ok(PathTemplate {
                nodes: vec![
                    PathAstNode::Var {
                        var_name: "project",
                        expansion_style: ExpansionStyle::Simple
                    },
                    PathAstNode::Lit("/managedZones/"),
                    PathAstNode::Var {
                        var_name: "managedZone",
                        expansion_style: ExpansionStyle::Reserved
                    },
                    PathAstNode::Lit("/changes"),
                ],
            })
        );
    }
}
