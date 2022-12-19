use clap::{Parser};
use std::path::Path;
use scan_control::command::*;
use scan_control::args::*;


fn main(){
    let args = ScanControlArgs::parse();

    match args.action {
        Action::UploadTable(path_str) => {
            upload_table(Path::new(&path_str.path)).unwrap()
        }
        Action::SetPPR(path_str) => {
            println!("Setting ppr ...");
            set_ppr(Path::new(&path_str.path)).unwrap();
        }
        Action::SetMRD(path_str) => {
            println!("Setting mrd ...");
            set_mrd(Path::new(&path_str.path)).unwrap();
        }
        Action::Status => {
            let stat = scan_status().unwrap();
            println!("scan_status: {:?}",stat);
        }
        Action::Abort => {
            abort().unwrap()
        }
        Action::RunDirectory(args) => {
            run_directory(args).unwrap()
        }
        Action::SetupPPR(args) => {
            setup_ppr(args).unwrap()
        }
        Action::AcquirePPR(args) => {
            acquire_ppr(args).unwrap()
        }
    }
}