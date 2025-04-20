use rpu;
use anyhow::Result;

use std::env;
use std::fs;
use std::io;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    match args.len() {
        0 => Ok(()),
        1 => Ok(()),
        2 => {
            let path = &args[1];
            let source = fs::read_to_string(path)?;

            let mut stdout = io::stdout();
            let mut pu = rpu::Core::new(&mut stdout);

            pu.load_source(&source)?;
            pu.start()?;

            Ok(())
        },
        _ => {
            // assume second arg is '--gui'
            let path = &args[2];
            let source = fs::read_to_string(path)?;
            rpu::gui::main(path).unwrap();
            Ok(())
        }
    }
}
