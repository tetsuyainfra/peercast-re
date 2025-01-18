// build.rs

use npm_rs::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let build = vergen::BuildBuilder::all_build()?;
    let cargo = vergen::CargoBuilder::all_cargo()?;
    let rustc = vergen::RustcBuilder::all_rustc()?;
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
        .add_instructions(&git2)?
        // .add_instructions(&si)?
        .emit()?;

    let target = std::env::var("TARGET").unwrap();
    if target.contains("windows") {
        // println!("cargo:rerun-if-changed=icon.rc");
        let _compilation_result = embed_resource::compile("icon.rc", embed_resource::NONE);
    }

    // cargo check(rust-analyzer)でこのbuild.rsが毎回実行されて遅くてつらい
    // そのため一旦OFFにする走らせない方法あると思うんだけどなぁ
    //
    // npm run buildでproduction版が常にbuildされる
    // npm run dev で development版がホストされる -> Httpでアクセスして取ってくる
    let is_rebuild = match std::env::var("PEERCAST_RT_BUILD_NPM_REBUILD")
        .unwrap_or_else(|_| "false".into())
        .as_str()
    {
        "1" | "true" => true,
        _ => false,
    };

    if is_rebuild {
        println!("RE-BUILD NPM");
        let exit_status = NpmEnv::default()
            .set_path("client")
            .with_node_env(&NodeEnv::Development)
            // .with_env("FOO", "bar")
            .init_env()
            .install(None)
            .run("build")
            .exec()?;
        assert!(exit_status.success());
    }

    Ok(())
}
