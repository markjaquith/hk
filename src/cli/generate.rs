use std::path::PathBuf;

use crate::{Result, env};

/// Generates a new hk.pkl file for a project
#[derive(Debug, clap::Args)]
#[clap(visible_alias = "g", alias = "init")]
pub struct Generate {
    /// Overwrite existing hk.pkl file
    #[clap(short, long)]
    force: bool,
    /// Generate a mise.toml file with hk configured
    ///
    /// Set HK_MISE=1 to make this default behavior.
    #[clap(long, verbatim_doc_comment)]
    mise: bool,
}

impl Generate {
    pub async fn run(&self) -> Result<()> {
        let hk_file = PathBuf::from("hk.pkl");
        let hook_content = r#"
amends "package://github.com/jdx/hk/releases/download/v0.6.5/hk@0.6.5#/Config.pkl"
// import "package://github.com/jdx/hk/releases/download/v0.6.5/hk@0.6.5#/builtins/prettier.pkl"

// example hk config is defined below
//
// linters = new {{
//     // uses builtin prettier linter config
//     ["prettier"] = new prettier.Prettier {{}}
//
//     // uses custom pkl linter config
//     ["pkl"] {{
//         glob = List("*.pkl")
//         check = "pkl eval {{files}} >/dev/null"
//     }}
// }}
//
// hooks = new {{
//   ["pre-commit"] {{
//     // "prelint" here is simply a name to define the step
//     ["prelint"] {{
//         // if a step has a "check" script it will execute that
//         check = "mise run prelint"
//         exclusive = true // ensures that the step runs in isolation
//     }}
//     // runs the linters using the "fix" command
//     ["fix"] = new Fix {{}}
//     ["postlint"] {{
//         check = "mise run postlint"
//         exclusive = true
//     }}
//   }}
//
//   // instead of pre-commit, you can instead define pre-push hooks
//   ["pre-push"] {{
//     // runs the linters using the "check" command
//     ["check"] = new Check {{}}
//   }}
// }}
"#;
        xx::file::write(hk_file, hook_content.trim_start())?;

        if *env::HK_MISE || self.mise {
            let mise_toml = PathBuf::from("mise.toml");
            let mise_content = r#"[tools]
hk = "latest"
pkl = "latest"
"#;
            if mise_toml.exists() {
                warn!("mise.toml already exists, run with --force to overwrite");
            } else {
                xx::file::write(mise_toml, mise_content)?;
                println!("Generated mise.toml");
            }
        }
        Ok(())
    }
}
