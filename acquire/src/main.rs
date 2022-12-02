use clap::Parser;
use acquire::build::{apply_setup, new, new_adjustment, new_config, new_diffusion_experiment, new_scout_experiment, new_setup, new_simulation, Sequence};
use acquire::args::*;

fn main(){
    let args = SeqLibArgs::parse();
    use Action::*;
    match &args.action {
        ListSequences => println!("{}", Sequence::list()),
        NewConfig(args) => new_config(&args),
        New(args) => new(&args),
        NewDiffusionExperiment(args) => new_diffusion_experiment(&args),
        NewScout(args) => new_scout_experiment(&args),
        NewSetup(args) => new_setup(&args),
        ApplySetup(args) => apply_setup(&args),
        NewSimulation(args) => new_simulation(&args),
        NewAdjustment(args) => new_adjustment(&args),
        _=> {}
    }
}