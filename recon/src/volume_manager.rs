use crate::mrd::Mrd;
use crate::pe_table::Petable;
use crate::bart_wrapper::{BartPicsSettings,bart_pics};
use serde::{Deserialize, Serialize};
use std::path::{Path,PathBuf};
use std::fs::{File,create_dir_all,remove_file};
use std::io::{Write,Read};
use crate::slurm::BatchScript;
use std::process::Command;
use std::error::Error;
use crate::cfl;
use seq_lib::headfile::Headfile;
use crate::config::Recon;

const VOLUME_MANAGER_FILENAME:&str = "volume-manager";

#[derive(Deserialize, Serialize)]
pub struct VolumeManager {
    file:String,
    mrd:String,
    phase_table:String,
    mrd_vol_offset:usize,
    reco_settings:PathBuf,
    state:VmState,
    kspace:Option<String>,
    imspace:Option<String>,
}

#[derive(Deserialize, Serialize,Clone,PartialEq,Eq,Debug)]
pub enum VmState {
    NotInstantiated,
    Idle,
    PreProcessing,
    Reconstructing,
    WritingOutput,
    Done,
}

#[derive(Deserialize, Serialize)]
struct ScalingInfo{
    histo_percent:f32,
    scale_factor:f32
}

impl VolumeManager {

    pub fn launch(workdir:&str,mrd:&str,phase_table:&str,vol_offset:usize,reco_settings:&Path) -> VolumeManager{
        if VolumeManager::exists(workdir){
            return VolumeManager::advance(workdir);
        }
        let vm_path = Path::new(workdir).join(VOLUME_MANAGER_FILENAME).with_extension("toml");
        let vm_path_str = vm_path.into_os_string().into_string().unwrap();
        let vm = VolumeManager{
            file:vm_path_str,
            mrd:mrd.to_string(),
            phase_table:phase_table.to_string(),
            mrd_vol_offset:vol_offset,
            reco_settings:reco_settings.to_owned(),
            state:VmState::Idle,
            kspace:None,
            imspace:None
        };
        VolumeManager::to_file(&vm);
        return vm;
    }

    pub fn is_done(workdir:&str) -> bool{
        use VmState::*;
        if VolumeManager::exists(workdir){
            let vm = VolumeManager::open(workdir);
            return match vm.state {
                Done => true,
                _ => false
            };
        }
        return false;
    }

    pub fn state(workdir:&str) -> VmState{
        use VmState::*;
        if VolumeManager::exists(workdir){
            let vm = VolumeManager::open(workdir);
            return vm.state
        }else{
            return NotInstantiated
        }
    }

    pub fn open(workdir:&str) -> VolumeManager{
        let vm_path = Path::new(workdir).join(VOLUME_MANAGER_FILENAME).with_extension("toml");
        let mut f = File::open(vm_path).expect("cannot open volume manager that doesn't yet exist");
        let mut sbuf = String::new();
        f.read_to_string(&mut sbuf).expect("problem reading file");
        return toml::from_str(&sbuf).expect("cannot deserialize volume manager. File may be corrupt");
    }

    fn to_file(&self){
        let vm_path = Path::new(&self.file);
        let mut f = File::create(vm_path).expect("cannot create file");
        let vm_str = toml::to_string(&self).expect("cannot serialize data structure");
        f.write_all(vm_str.as_bytes()).expect("trouble writing to file");
    }

    pub fn advance(workdir:&str) -> VolumeManager{
        use VmState::*;
        let mut vm = VolumeManager::open(workdir);
        let mut r = Recon::open(&vm.reco_settings.to_str().unwrap()).unwrap();
        match vm.state {
            Idle => {
                vm.advance_state();
            }
            PreProcessing => {
                //if Path::new(&vm.mrd).exists(){
                    let mut mrd = Mrd::new(&vm.mrd);
                    mrd.load_volume(vm.mrd_vol_offset as u16);
                    let petab = Petable::new(&vm.phase_table);
                    let mrd_name = Path::new(&vm.mrd).with_extension("");
                    let mrd_name = mrd_name.file_name().unwrap().to_str().unwrap();
                    let kspace = Path::new(&workdir).join(&format!("{}_kspace",mrd_name)).with_extension("");
                    mrd.write_zero_filled_cfl(kspace.to_str().unwrap(), &petab);
                    vm.kspace = Some(kspace.to_str().unwrap().to_string());
                    vm.advance_state();
                //}
            },
            Reconstructing => {
                let kspace = vm.kspace.clone().unwrap();
                let mrd_name = Path::new(&vm.mrd).with_extension("");
                let mrd_name = mrd_name.file_name().unwrap().to_str().unwrap();
                let imspace = Path::new(&workdir).join(&format!("{}_imspace",mrd_name)).with_extension("");
                bart_pics(&kspace,imspace.to_str().unwrap(),&mut r.project.recon_settings);
                vm.imspace = Some(imspace.to_str().unwrap().to_string());
                vm.advance_state();
            }
            WritingOutput => {
                let imspace = vm.imspace.clone().unwrap();
                let cfl = Path::new(&imspace);
                let thisdir = Path::new(&vm.file);
                let outdir = thisdir.with_file_name("image");
                if !outdir.exists(){create_dir_all(&outdir).expect("cannot make directory");}
                
                let dirname = Path::new(&vm.file).parent().unwrap().file_name().unwrap().to_str().unwrap();
                let imgname = format!("{}_m{}",&r.run_number,dirname);

                /* if we are the first volume (up to a collection of 1000), calculate the appropriate
                scaling for a u16 image, scale by this amount, then write to a file to inform the other volume
                managers what we chose for a scale factor. We immediatly advance state when we are done, but force
                the others to wait.
                */
                let workdir = thisdir.parent().unwrap().parent().unwrap();
                let img_scale_file = workdir.join("image-scaling").with_extension("toml");
                match dirname {
                    "0"|"00"|"000" => {
                        let imspace = vm.imspace.clone().unwrap();
                        let scale = cfl::find_u16_scale(Path::new(&imspace),r.project.recon_settings.image_scale_histo_percent);
                        let s = ScalingInfo{histo_percent:0.9995,scale_factor:scale};
                        let mut f = File::create(img_scale_file).expect("cannot create file");
                        let str = toml::to_string(&s).expect("cannot deserialize struct");
                        f.write_all(str.as_bytes()).expect("trouble writing to file");
                        cfl::to_civm_raw_u16(&cfl,&outdir,&imgname,"t9imx",scale);
                        vm.advance_state();
                    },
                    _=> {
                        if img_scale_file.exists(){
                            let mut f = File::open(img_scale_file).expect("trouble opening file");
                            let mut str = String::new();
                            f.read_to_string(&mut str).expect("trouble reading file");
                            let r:std::result::Result<ScalingInfo, toml::de::Error> = toml::from_str(&str);
                            match r{
                                Ok(scale_info) => {
                                    cfl::to_civm_raw_u16(&cfl,&outdir,&imgname,"t9imx",scale_info.scale_factor);
                                    vm.advance_state();
                                }
                                Err(_) => {/* do nothing. Will need to run again later when data is available*/},
                            };
                        }
                    },
                };
                let mrd_meta = Path::new(&vm.mrd);
                let base = mrd_meta.parent().unwrap();
                let mrd_name = mrd_meta.file_stem().unwrap().to_str().unwrap();
                let meta_name = format!("{}_meta.txt",mrd_name);

                let meta_path = base.join(&meta_name);
                println!("meta path: {:?}",meta_path);
                let mut hf = Headfile::from_mrd_meta(&meta_path);
                let headfile = outdir.join(&format!("{}.headfile",&imgname));
                
            
                /*
                headfile=mrs_meta_data(mrd);
                headfile.dti_vols = n_volumes;
                headfile.U_code = project_code;
                headfile.U_civmid = civm_userid;
                headfile.U_specid = specimen_id;
                headfile.scanner_vendor = scanner_vendor;
                headfile.U_runno = strcat(run_number,'_',mnum);
                headfile.dim_X = vol_size(1);
                headfile.dim_Y = vol_size(2);
                headfile.dim_Z = vol_size(3);
                headfile.civm_image_code = 't9';
                headfile.civm_image_source_tag = 'imx';
                headfile.engine_work_directory = pwd;
                */
                let volpath = &vm.imspace.clone().unwrap();
                let dims = cfl::get_dims(&Path::new(volpath));
                if dims.len() < 3 {panic!("what happend to the dimensions of the volume??")}
                // inject more last-minute info into the headfile... this is a bit messy
                if r.n_volumes.is_some(){
                    hf.append_field("dti_vols",&r.n_volumes.unwrap());
                }
                hf.append_field("dim_X", dims[0]);
                hf.append_field("dim_Y", dims[1]);
                hf.append_field("dim_Z", dims[2]);
                hf.append_field("U_code", &r.project.project_code);
                hf.append_field("U_civmid",&r.recon_person);
                hf.append_field("U_specid", &r.specimen_id);
                hf.append_field("scanner_vendor", &r.scanner.vendor);
                hf.append_field("U_runno", &r.run_number);
                hf.append_field("civm_image_code", &r.scanner.image_code);
                hf.append_field("civm_image_source_tag", &r.scanner.image_source_tag);
                hf.append_field("engine_work_directory", &r.engine_work_dir.to_str().unwrap());
                hf.write_headfile(&headfile);

            }
            Done => {/*no op*/}
            NotInstantiated => {/* null case. This state exists for external use only */}
        }
        return vm;
    }

    fn advance_state(&mut self){
        use VmState::*;
        match self.state{
            Idle => self.state = PreProcessing,
            PreProcessing => self.state = Reconstructing,
            Reconstructing => self.state = WritingOutput,
            WritingOutput => self.state = Done,
            Done => {/*no op*/}
            NotInstantiated => {/* null case. This state exists for external use only */}
        }
        self.to_file();
    }

    fn fpath(workdir:&str) -> PathBuf{
        return Path::new(workdir).join(VOLUME_MANAGER_FILENAME).with_extension("toml");
    }

    pub fn exists(workdir:&str) -> bool{
        let vm_path = VolumeManager::fpath(workdir);
        return vm_path.exists();
    }
}

pub fn launch_volume_manager(workdir:&str,mrd:&str,phase_table:&str,vol_offset:usize,reco_settings:&Path){
    use VmState::*;
    let mut vm = VolumeManager::launch(workdir,mrd,phase_table,vol_offset,reco_settings);
    loop {
        let prev_state = vm.state.clone();
        vm = VolumeManager::launch(workdir,mrd,phase_table,vol_offset,reco_settings);
        if prev_state == vm.state{
            break
        }

        match vm.state {
            Done => break,
            _ => continue
        }
    }
}

pub fn re_launch_volume_manager(workdir:&str) -> u32{
    let vm = VolumeManager::open(workdir);
    return launch_volume_manager_job(workdir,&vm.mrd,&vm.phase_table,vm.mrd_vol_offset,&vm.reco_settings);
}

pub fn launch_volume_manager_job(workdir:&str,mrd:&str,phase_table:&str,vol_offset:usize,reco_settings:&Path) -> u32{
    let wp = Path::new(&workdir);
    let this_executable = std::env::current_exe().expect("failed to resolve this exe path");
    let mut cmd = Command::new(this_executable.to_str().unwrap());
    // we are giving the sbatch file our identity to call ourself
    cmd.arg("volume-manager");
    cmd.arg(workdir);
    cmd.arg(&mrd);
    cmd.arg(phase_table);
    cmd.arg(vol_offset.to_string());
    cmd.arg(reco_settings.to_str().unwrap());
    let cmd = format!("{:?}",cmd);
    let mut job = BatchScript::new(&format!("slurm-job"));
    job.options.output = wp.join("slurm-log.out").into_os_string().into_string().unwrap();
    let mrd_name  = Path::new(mrd).file_name().expect("cannot find file name from path").to_str().unwrap();
    job.options.job_name = format!("recon_{}",mrd_name);
    job.options.partition = std::env::var("PIPELINE_QUEUE").unwrap_or(String::from(""));
    job.commands.push(cmd);
    let job_id = job.submit(&workdir);
    return job_id;
}

pub fn re_launch_volume_manager_job(workdir:&str) -> u32{
    let wp = Path::new(&workdir);
    let this_executable = std::env::current_exe().expect("failed to resolve this exe path");
    let mut cmd = Command::new(this_executable.to_str().unwrap());
    cmd.arg("volume-manager-relaunch");
    cmd.arg(workdir);
    let cmd = format!("{:?}",cmd);
    let mut job = BatchScript::new(&format!("slurm-job"));
    job.options.output = wp.join("slurm-log.out").into_os_string().into_string().unwrap();
    job.commands.push(cmd);
    let job_id = job.submit(&workdir);
    return job_id;
}


// #[test]
// fn test(){
//     use VmState::*;
//     let bart_settings_file = "/home/waustin/vol_00/reco_settings";
//     let mrd = "/home/waustin/mrs_test_data/test_data/N20220728_00/_02_ICO61_6b0/220728T16_m00.mrd";
//     let ptab = "/home/waustin/mrs_test_data/petableCS_stream/stream_CS480_8x_pa18_pb54";
//     let workdirname = "/home/waustin/vol_00";
//     let runno = "test_runno";
//     let workdir = Path::new(workdirname);
//     if !workdir.exists(){
//         create_dir_all(workdir).expect("problem creating directory"); 
//     }

//     let bart_settings = BartPicsSettings::quick();
//     bart_settings.to_file(bart_settings_file);

//     let mut vm = VolumeManager::launch(workdirname,runno,mrd,ptab,0,bart_settings_file);
//     loop {
//         let prev_state = vm.state.clone();
//         vm = VolumeManager::launch(workdirname,runno,mrd,ptab,0,bart_settings_file);
//         if prev_state == vm.state{
//             break
//         }
//         match vm.state {
//             Done => break,
//             _ => continue
//         }
//     }
    
// }