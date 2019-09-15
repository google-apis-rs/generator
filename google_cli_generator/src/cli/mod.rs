use discovery_parser::DiscoveryRestDesc;
use log::info;
use std::{convert::TryFrom, error::Error, fs, io::Write, path::Path};

use crate::cargo;
use model::Model;
use std::cmp::Ordering;
use std::ffi::OsStr;

mod model;

pub fn generate(
    output_dir: impl AsRef<Path>,
    discovery_desc: &DiscoveryRestDesc,
) -> Result<(), Box<dyn Error>> {
    const MAIN_RS: &str = r#"
   "#;
    //    info!("cli: building api desc");
    //    let _api_desc = APIDesc::from_discovery(discovery_desc);
    let api = shared::Api::try_from(discovery_desc)?;

    let constants = shared::Standard::default();
    let output_dir = output_dir.as_ref();
    let cargo_toml_path = output_dir.join(&constants.cargo_toml_path);
    let main_path = output_dir.join(&constants.main_path);

    info!("cli: creating source directory and Cargo.toml");
    fs::create_dir_all(&main_path.parent().expect("file in directory"))?;

    let cargo_contents = cargo::cargo_toml(&api, &constants);
    fs::write(&cargo_toml_path, &cargo_contents)?;

    info!("cli: writing main '{}'", main_path.display());
    let mut rustfmt_writer = shared::RustFmtWriter::new(fs::File::create(&main_path)?)?;
    rustfmt_writer.write_all(MAIN_RS.as_bytes())?;

    let templates_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
    let engine = liquid::ParserBuilder::with_liquid().build()?;
    let model = into_liquid_object(Model::new(api, discovery_desc))?;
    let mut templates: Vec<_> = templates_dir
        .read_dir()?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|e| {
            e.file_type().map(|e| e.is_file()).unwrap_or(false)
                && e.path().extension() == Some(OsStr::new("liquid"))
        })
        .collect();
    templates.sort_by(|l, r| {
        l.path()
            .file_name()
            .and_then(|fl| r.path().file_name().map(|rl| rl.cmp(fl)))
            .unwrap_or(Ordering::Equal)
    });

    for entry in templates {
        let template = fs::read_to_string(entry.path())?;
        let template = engine.parse(&template).map_err(|err| {
            format!(
                "Failed to parse liquid template at '{}': {}",
                entry.path().display(),
                err
            )
        })?;
        let rendered = template.render(&model)?;
        rustfmt_writer.write_all(rendered.as_bytes())?;
    }

    rustfmt_writer.close()?;

    Ok(())
}

fn into_liquid_object(src: impl serde::Serialize) -> Result<liquid::value::Object, Box<dyn Error>> {
    let src = serde_json::to_value(src)?;
    let dst = serde_json::from_value(src)?;
    match dst {
        liquid::value::Value::Object(obj) => Ok(obj),
        _ => Err("Data model root must be an object".to_owned().into()),
    }
}
