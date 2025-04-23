use rpu;
use anyhow::Result;

use std::env;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    match args.len() {
        0 => Ok(()),
        1 => Ok(()),
        2 => {
            let path = &args[1];
            rpu::gui::main(path).unwrap();
            Ok(())
        },
        _ => Ok(())
    }
}
