use std::error;

pub struct Config {
    pub cache: String,
    pub dial: String,
}

pub fn run(config: Config) -> Result<(), Box<dyn error::Error>> {
    println!("Cache directory: {}", config.cache);
    println!("Dial address: {}", config.dial);
    Ok(())
}
