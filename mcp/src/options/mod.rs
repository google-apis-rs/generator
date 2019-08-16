pub fn output_formats() -> &'static [&'static str] {
    &["json", "yaml"]
}

pub mod completions;
pub mod process;
pub mod substitute;
