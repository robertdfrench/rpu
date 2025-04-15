use ybrpu;
use anyhow::Result;
use std::io;

fn main() -> Result<()> {
    let mut stdout = io::stdout();
    let mut pu = ybrpu::ProcessingUnit::new(&mut stdout);

    let source = [
        "put 7 gr0",
        "put 5 gr1",
        "add gr1 gr0",
        "cp srA out",
        "add srA gr0",
        "cp srA out",
        "cp srA gr0",
        "add gr0 gr0",
        "cp srA out"
    ];
    let source = source.join("\n");

    pu.load_source(&source)?;

    pu.start()?;

    Ok(())
}
