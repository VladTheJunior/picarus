use vergen_gitcl::{Build, Cargo, Emitter, Gitcl, Rustc};

fn main() -> Result<(), Box<dyn std::error::Error>> {

let build = Build::all_build();
let cargo = Cargo::all_cargo();
let gitcl = Gitcl::all_git();
let rustc = Rustc::all_rustc();

Emitter::default()
    .add_instructions(&build)?
    .add_instructions(&cargo)?
    .add_instructions(&gitcl)?
    .add_instructions(&rustc)?
    .emit()?;

    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("icon.ico");
        res.compile().unwrap();
    }

    Ok(())
}