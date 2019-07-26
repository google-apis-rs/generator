use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    print!(
        "{}",
        generator::generate("https://www.googleapis.com/discovery/v1/apis/drive/v3/rest")?
    );
    Ok(())
}
