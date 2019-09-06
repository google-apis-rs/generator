use crate::{APIDesc, Resource};
use std::fmt;

pub(crate) fn generate(api_desc: &APIDesc) -> String {
    let mut output = "# Resources and Methods\n".to_owned();
    for resource in &api_desc.resources {
        generate_resource(resource, &mut output);
    }
    output
}

fn generate_resource(resource: &Resource, output: &mut String) {
    let mod_path = module_path(resource);
    let indent_amount = resource.parent_path.segments.len();
    use fmt::Write;
    for _ in 0..indent_amount * 2 {
        output.push(' ');
    }
    output.push_str("* ");
    write!(
        output,
        "[{}]({}/struct.{}.html)\n",
        &resource.ident,
        &mod_path,
        resource.action_type_name()
    )
    .unwrap();
    if !resource.methods.is_empty() {
        for _ in 0..(indent_amount + 1) * 2 {
            output.push(' ');
        }
        output.push_str("* ");
        let mut first_method = true;
        for method in &resource.methods {
            if !first_method {
                output.push_str(", ");
            }
            write!(
                output,
                "[*{}*]({}/struct.{}.html)",
                &method.id,
                &mod_path,
                method.builder_name()
            )
            .unwrap();
            first_method = false;
        }
        output.push('\n');
    }
    for resource in &resource.resources {
        generate_resource(resource, output);
    }
}

fn module_path(resource: &Resource) -> String {
    use std::fmt::Write;
    let mut output = String::new();
    for seg in resource.parent_path.segments.iter().skip(1) {
        write!(&mut output, "{}/", &seg.ident).unwrap();
    }
    write!(&mut output, "{}", &resource.ident).unwrap();
    output
}
