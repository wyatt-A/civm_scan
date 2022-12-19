use std::path::PathBuf;

#[derive(clap::Parser,Debug)]
pub struct ScanControlArgs {
    #[command(subcommand)]
    pub action: Action,
}

#[derive(clap::Subcommand,Debug)]
pub enum Action {
    /// upload a lookup table for compressed sensing
    UploadTable(PathArgs),
    /// set the ppr for the scan
    SetPPR(PathArgs),
    /// set the output mrd for data collection
    SetMRD(PathArgs),
    /// get the status of the scan supervisor
    Status,
    /// finds all pprs nested in the parent directory and runs them
    RunDirectory(RunDirectoryArgs),
    /// abort the scan
    Abort,
    /// Run a ppr in setup mode
    SetupPPR(RunDirectoryArgs),
    /// Acquire data for PPR
    AcquirePPR(RunDirectoryArgs),
}

#[derive(clap::Args,Debug)]
pub struct PathArgs {
    pub path:PathBuf,
}

#[derive(clap::Args,Debug)]
pub struct RunDirectoryArgs {
    pub path:PathBuf,
    #[clap(short, long)]
    pub cs_table:Option<String>,
    #[clap(short, long)]
    pub depth_to_search:Option<u8>
}