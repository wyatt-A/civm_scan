use clap;
use std::path::PathBuf;
use crate::build::ContextParams;

#[derive(clap::Parser,Debug)]
pub struct SeqLibArgs {
    #[command(subcommand)]
    pub action: Action,
}

#[derive(clap::Subcommand,Debug)]
pub enum Action {
    NewConfig(NewConfigArgs),
    New(NewArgs),
    NewSimulation(NewArgs),
    NewDiffusionExperiment(NewDiffusionExperimentArgs),
    NewScout(NewArgs),
    NewSetup(NewArgs),
    NewAdjustment(NewAdjArgs),
    ApplySetup(ApplySetupArgs),
    ListSequences,
}

#[derive(clap::Args,Debug)]
pub struct NewDiffusionExperimentArgs {
    pub alias:String,
    pub destination:PathBuf,
    pub b_table:PathBuf,
    pub adjustment_file:Option<PathBuf>
}

impl NewDiffusionExperimentArgs {
    pub fn context_params(&self) -> ContextParams {
        ContextParams {
            export_dir: self.destination.clone(),
            adjustment: self.adjustment_file.clone(),
            build: true
        }
    }
}

#[derive(clap::Args,Debug)]
pub struct NewArgs {
    pub alias:String,
    pub destination:PathBuf,
    pub adjustment_file:Option<PathBuf>
}

impl NewArgs {
    pub fn context_params(&self) -> ContextParams {
        ContextParams {
            export_dir: self.destination.clone(),
            adjustment: self.adjustment_file.clone(),
            build: true
        }
    }
}

#[derive(clap::Args,Debug)]
pub struct NewConfigArgs {
    pub name:String,
    pub alias:String
}

#[derive(clap::Args,Debug)]
pub struct ApplySetupArgs {
    pub setup_ppr:PathBuf,
    pub children:PathBuf,
    #[clap(short, long)]
    pub depth:Option<u16>
}

#[derive(clap::Args,Debug)]
pub struct NewAdjArgs {
    pub alias:String,
    pub destination:PathBuf,
}

impl NewAdjArgs {
    pub fn context_params(&self) -> ContextParams {
        ContextParams {
            export_dir: self.destination.clone(),
            adjustment: None,
            build: true
        }
    }
}