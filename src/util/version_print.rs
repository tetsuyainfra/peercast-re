use std::collections::BTreeMap;

use nom::combinator::Opt;

pub fn version_print(output_as_json: bool) -> anyhow::Result<()> {
    version_print_with(output_as_json, |_env| {});
    Ok(())
}

pub fn version_print_with<F>(output_as_json: bool, f: F) -> anyhow::Result<()>
where
    F: Fn(&mut BTreeMap<&'static str, Option<&'static str>>) -> (),
{
    let x = || -> (bool) { true };

    let mut build_envs = vergen_pretty::vergen_pretty_env!();
    // build_envs.insert("VERGEN_BIN_NAME", Some(env!("CARGO_BIN_NAME")));
    // build_envs.insert("VERGEN_BIN_VERSION", Some(crate::PKG_VERSION));
    build_envs.insert("VERGEN_PKG_VERSION", Some(crate::PKG_VERSION));
    build_envs.insert("VERGEN_PKG_VERSION_MAJOR", Some(crate::PKG_VERSION_MAJOR));
    build_envs.insert("VERGEN_PKG_VERSION_MINOR", Some(crate::PKG_VERSION_MINOR));
    build_envs.insert("VERGEN_PKG_VERSION_PATCH", Some(crate::PKG_VERSION_PATCH));
    build_envs.insert("VERGEN_PKG_AGENT", Some(crate::PKG_AGENT));

    f(&mut build_envs);

    if output_as_json {
        let build_envs = build_envs
            .into_iter()
            .filter_map(|(k, v)| v.map(|v| (k, v)))
            .collect::<BTreeMap<_, _>>();

        let s = serde_json::to_string_pretty(&build_envs)?;
        println!("{}", s);
    } else {
        let stdout = std::io::stdout();
        let mut stdout = stdout.lock();

        let _pp = vergen_pretty::PrettyBuilder::default()
            .env(build_envs)
            .build()?
            .display(&mut stdout)?;
    }

    Ok(())
}
