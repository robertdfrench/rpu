use ybrpu;
use anyhow::Result;
use std::io;

fn main() -> Result<()> {
    let mut stdout = io::stdout();
    let mut pu = ybrpu::ProcessingUnit::new(&mut stdout);

    let source = [
        "put 7 gp0",
        "put 5 gp1",
        "add gp1 gp0",
        "cp ans out",
        "add ans gp0",
        "cp ans out",
        "cp ans gp0",
        "add gp0 gp0",
        "cp ans out"
    ];
    let source = source.join("\n");

    pu.load_source(&source)?;

    pu.start()?;

    Ok(())
}
