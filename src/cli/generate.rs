use std::path::PathBuf;

use crate::{env, version, Result};

/// Generates a new hk.pkl file for a project
#[derive(Debug, clap::Args)]
#[clap(visible_alias = "g")]
pub struct Generate {
    /// Generate a mise.toml file with hk configured
    ///
    /// Set HK_MISE=1 to make this default behavior.
    #[clap(long, verbatim_doc_comment)]
    mise: bool,
}

impl Generate {
    pub async fn run(&self) -> Result<()> {
        let hk_file = PathBuf::from("hk.pkl");
        let version = version::version();
        let hook_content = format!(
            r#"
// amends "package://hk.jdx.dev/hk@0.1.0#/hk.pkl"
amends "pkl/hk.pkl"
import "pkl/builtins.pkl"

min_hk_version = "{version}"

// example git hooks are defined below
//
// `pre-commit` {{
//     // "prelint" here is simply a name to define the step
//     ["prelint"] {{
//         // if a step has a "run" script it will execute that
//         run = "mise run prelint"
//         exclusive = true // ensures that the step runs in isolation
//     }}
//     // everything from here to postlint is run in parallel
//     ["pkl"] {{
//         glob = new {{ "*.pkl" }}
//         run = "pkl eval {{files}} >/dev/null"
//     }}
//     // predefined formatters+linters
//     ["cargo-check"] = new builtins.CargoCheck {{}}
//     ["cargo-fmt"] = new builtins.CargoFmt {{}}
//     ["eslint"] = new builtins.Eslint {{}}
//     ["prettier"] = new builtins.Prettier {{
//         glob = new {{ "*.js"; "*.ts" }} // override the default globs
//     }}
//     ["postlint"] {{
//         run = "mise run postlint"
//         exclusive = true
//     }}
// }}
//
// // instead of pre-commit, you can instead define pre-push hooks
// `pre-push` {{
//     ["eslint"] = new builtins.Eslint {{}}
// }}
//
// // TODO
// `commit-msg` {{
// }}
//
// // TODO
// prepare-commit-msg {{
// }}
//
// // TODO
// update {{
// }}
"#
        );
        xx::file::write(hk_file, hook_content.trim_start())?;

        if *env::HK_MISE || self.mise {
            let mise_toml = PathBuf::from("mise.toml");
            let mise_content = r#"[tools]
hk = "latest"

[tasks]
pre-commit = "hk run pre-commit"
"#;
            if mise_toml.exists() {
                warn!("mise.toml already exists");
            } else {
                xx::file::write(mise_toml, mise_content)?;
                println!("Generated mise.toml");
            }
        }
        Ok(())
    }
}
