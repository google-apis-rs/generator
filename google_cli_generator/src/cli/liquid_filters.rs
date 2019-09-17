use liquid::compiler::Filter;
use liquid::derive::*;
use liquid::error::Result;
use liquid::interpreter::Context;
use liquid::value::Value;

#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "rust_string_literal",
    description = "make any string printable as a Rust string",
    parsed(RustStringLiteralFilter)
)]
pub struct RustStringLiteral;

#[derive(Debug, Default, Display_filter)]
#[name = "rust_string_literal"]
struct RustStringLiteralFilter;

impl Filter for RustStringLiteralFilter {
    fn evaluate(&self, input: &Value, _context: &Context) -> Result<Value> {
        Ok(Value::scalar(format!("{:?}", input.to_str())))
    }
}
