/*
    cs_recon main is the entry point for the civm reconstruction pipeline that is using BART under
    the hood.
*/
//use recon::volume_manager::{launch_volume_manager,re_launch_volume_manager};
//use recon::test::{main_test_cluster};
use clap::Parser;
use std::path::Path;
use recon::vol_manager::VolumeManagerArgs;
use recon::vol_manager::test_updated;


/*
    recon volume-manager launch "path/to/args/file"
 */


// fn main(){
//     let args = VolumeManagerCmd::parse();
//     match args.action {
//         Action::Launch(args) => {
//             VolumeManager::launch(args)
//         }
//         Action::ReLaunch(args) => {
//             VolumeManager::re_launch(args)
//         }
//     }
// }

fn main() {
    test_updated()
}