use clap::{Parser,Arg,Subcommand};
use std::path::Path;
use scan_control::command::*;
use scan_control::args::*;


fn main(){
    let args = ScanControlArgs::parse();

    match args.action {
        Action::UploadTable(path_str) => {
            upload_table(Path::new(&path_str.path))
        }
        Action::SetPPR(path_str) => {
            println!("Setting ppr ...");
            set_ppr(Path::new(&path_str.path));
        }
        Action::SetMRD(path_str) => {
            println!("Setting mrd ...");
            set_mrd(Path::new(&path_str.path));
        }
        Action::Status => {
            let stat = scan_status();
            println!("scan_status: {:?}",stat);
        }
        Action::RunSetup => {
            run_setup()
        }
        Action::RunScan => {
            run_acquisition()
        }
        Action::Abort => {
            abort()
        }
        Action::RunDirectory(args) => {
            run_directory(args)
        }
        Action::SetupPPR(args) => {
            setup_ppr(args);
        }
        Action::AcquirePPR(args) => {
            acquire_ppr(args)
        }
    }
}