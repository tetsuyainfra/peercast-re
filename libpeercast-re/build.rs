// build.rs

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let build = vergen::BuildBuilder::all_build()?;
    let cargo = vergen::CargoBuilder::all_cargo()?;
    let rustc = vergen::RustcBuilder::all_rustc()?;
    // let si = SysinfoBuilder::all_sysinfo()?;
    let git2 = vergen_git2::Git2Builder::default()
        .sha(false)
        .branch(true)
        .commit_timestamp(true)
        .commit_count(true)
        .dirty(true)
        .build()?;

    // let si = vergen::SysinfoBuilder::all_sysinfo()?;
    vergen::Emitter::default()
        .add_instructions(&build)?
        .add_instructions(&cargo)?
        .add_instructions(&rustc)?
        // .add_instructions(&si)?
        .add_instructions(&git2)?
        .emit()?;
    Ok(())
}
