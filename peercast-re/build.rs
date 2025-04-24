fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    // let mut will_build = match std::env::var("PEERCAST_RT_BUILD_NPM_REBUILD")
    //     .unwrap_or_else(|_| "false".into())
    //     .as_str()
    // {
    //     "1" | "true" => true,
    //     _ => false,
    // };

    // if false == std::fs::exists("client/dist").unwrap_or(false) {
    //     will_build = true;
    // }

    // if will_build {
    //     println!("RE-BUILD NPM");
    //     let exit_status = NpmEnv::default()
    //         .set_path("client")
    //         .with_node_env(&NodeEnv::Development)
    //         // .with_env("FOO", "bar")
    //         .init_env()
    //         .install(None)
    //         .run("gen-api")
    //         .run("gen-api2")
    //         .run("build")
    //         .exec()?;
    //     assert!(exit_status.success());
    // }

    Ok(())
}
