use pulldown_cmark::Parser;
use pulldown_cmark_to_cmark::fmt::cmark;

pub fn sanitize(md: &str) -> String {
    let mut output = String::with_capacity(2048);
    cmark(
        Parser::new_ext(&md, pulldown_cmark::Options::all()).map(|e| {
            use pulldown_cmark::Event::*;
            match e {
                Start(ref tag) => {
                    use pulldown_cmark::Tag::*;
                    match tag {
                        CodeBlock(code) => Start(CodeBlock(format!("text{}", code).into())),
                        _ => e,
                    }
                }
                _ => e,
            }
        }),
        &mut output,
        None,
    )
    .unwrap();
    output
}
