/*
    cs_recon main is the entry point for the civm reconstruction pipeline that is using BART under
    the hood.
*/
//use recon::volume_manager::{launch_volume_manager,re_launch_volume_manager};
//use recon::test::{main_test_cluster};
use clap::Parser;
use std::path::Path;
use recon::test::test_updated;

/*
    Main entry point for arguments: looking for a sub-command and their args
*/
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args{
    sub_cmd:String,
    vargs:Vec<String>
}

/*
    Volume manager arguments
*/
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct VolumeManagerArgs{
    parent:String,
    working_directory:String,
    mrd_file:String,
    phase_encode_stream_table:String,
    volume_offset:usize,
    reco_settings_json:String
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct VolumeManagerRelaunchArgs{
    parent:String,
    working_directory:String,
}

/*
    Mrd to cfl args
*/
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct MrdToCflArgs{
    parent:String,
    cmd2:String,
    cmd3:String
}



fn main(){
    let args = Args::parse();
    match args.sub_cmd.as_str(){
        // "volume-manager" => {
        //     let a = VolumeManagerArgs::parse();
        //     launch_volume_manager(
        //         &a.working_directory,
        //         &a.mrd_file,
        //         &a.phase_encode_stream_table,
        //         a.volume_offset,
        //         &Path::new(&a.reco_settings_json)
        //     );
        // }
        // "volume-manager-relaunch" => {
        //     let a = VolumeManagerRelaunchArgs::parse();
        //     re_launch_volume_manager(&a.working_directory);
        // },
        "cluster-test" => {
            test_updated();
        },
        _ => println!("sub-command not recognized")
    }
}