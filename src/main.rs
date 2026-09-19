use std::collections::HashMap;
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;

use clap::Parser;

use strictsubst::dotenv::parse_dotenv;
use strictsubst::parse::parse_template;
use strictsubst::render::render;

#[derive(Parser)]
#[command(
    name = "strictsubst",
    about = "A stricter envsubst: fails loud on an unset variable instead of substituting an empty string"
)]
struct Cli {
    /// Template file. Reads stdin if omitted.
    template: Option<PathBuf>,

    /// Write output to this file instead of stdout.
    #[arg(short = 'o', long)]
    output: Option<PathBuf>,

    /// Layer extra variables from a dotenv-style file on top of the real environment.
    #[arg(short = 'e', long)]
    env_file: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let template_text = match &cli.template {
        Some(path) => fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("reading {}: {e}", path.display()))?,
        None => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)?;
            buf
        }
    };

    let mut env: HashMap<String, String> = std::env::vars().collect();
    if let Some(path) = &cli.env_file {
        let text = fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("reading {}: {e}", path.display()))?;
        env.extend(parse_dotenv(&text));
    }

    let segments = parse_template(&template_text);
    match render(&segments, &env) {
        Ok(rendered) => {
            match &cli.output {
                Some(path) => fs::write(path, rendered)
                    .map_err(|e| anyhow::anyhow!("writing {}: {e}", path.display()))?,
                None => {
                    if let Err(e) = io::stdout().write_all(rendered.as_bytes()) {
                        if e.kind() != io::ErrorKind::BrokenPipe {
                            return Err(e.into());
                        }
                    }
                }
            }
            Ok(())
        }
        Err(errors) => {
            eprintln!("strictsubst: {} variable(s) failed:", errors.len());
            for e in &errors {
                eprintln!("  {e}");
            }
            std::process::exit(1);
        }
    }
}
