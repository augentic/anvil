use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use crux_core::{
    cli::{BindgenArgsBuilder, bindgen},
    type_generation::facet::{Config, TypeRegistry},
};
use log::info;
use uniffi::deps::anyhow::Result;

use shared::__APP_STRUCT__;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Language {
    Swift,
    Kotlin,
    Typescript,
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, value_enum)]
    language: Language,
    #[arg(short, long)]
    output_dir: PathBuf,
}

fn main() -> Result<()> {
    pretty_env_logger::init();
    let args = Args::parse();

    let typegen_app = TypeRegistry::new()
        .register_app::<__APP_STRUCT__>()?
        .build()?;

    let name = match args.language {
        Language::Swift => "SharedTypes",
        Language::Kotlin => "__ANDROID_PACKAGE__",
        Language::Typescript => "shared_types",
    };
    let config = Config::builder(name, &args.output_dir)
        .add_extensions()
        .add_runtimes()
        .build();

    match args.language {
        Language::Swift => {
            info!("Generating Swift types");
            typegen_app.swift(&config)?;
        }
        Language::Kotlin => {
            info!("Generating Kotlin types");
            typegen_app.kotlin(&config)?;

            info!("Generating Kotlin bindings");
            let bindgen_args = BindgenArgsBuilder::default()
                .crate_name(env!("CARGO_PKG_NAME").to_string())
                .kotlin(&args.output_dir)
                .build()?;
            bindgen(&bindgen_args)?;
        }
        Language::Typescript => {
            info!("Generating TypeScript types");
            typegen_app.typescript(&config)?;
        }
    }

    Ok(())
}
