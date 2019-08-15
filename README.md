Overview

 * discovery_parser: The most fundamental translation of the google discovery
   doscument into rust types.
 * field_selector/field_selector_derive: Includes the FieldSelector trait and a
   proc-macro to automatically derive the trait from a struct. FieldSelector
   provides a method to return a value to be used in the `fields` attribute of
   google apis. Syntax would look something like
   `kind,items(title,characteristics/length)`
 * uri_template_parser: Parse uri templates (RFC 6570) into an AST. The AST can
   represent the full syntax as described in the RFC, but does not attempt to
   produce the rendered template output. The AST is used by the generator to
   generate rust code that renders the template.
 * generator: This is the primary purpose of this repository. Given a discovery
   document it will produce idiomatic rust bindings to work with the API. The
   input is a discovery document and the output is rust crate at a specified
   directory.
