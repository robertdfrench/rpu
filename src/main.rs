use rpu;
use anyhow::Result;

use std::env;
use std::fs;
use std::io;

fn main() -> Result<()> {
    let mut stdout = io::stdout();
    let mut pu = rpu::Core::new(&mut stdout);

    for file in env::args().skip(1) {
        let source = fs::read_to_string(file)?;
        pu.load_source(&source)?;
        pu.start()?;
    }

    Ok(())
}
