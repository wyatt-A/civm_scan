use crate::{cfl, resource::*, utils};
use crate::volume_index::VolumeIndex;
use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use std::fs::{File,create_dir_all};
use std::io::{Read,Write};
use std::path::{Path, PathBuf};
use whoami;
use serde_json;
use crate::bart_wrapper::{bart_pics, BartPicsSettings};
use crate::volume_manager::{VmState,VolumeManager,launch_volume_manager,launch_volume_manager_job,re_launch_volume_manager_job};
use crate::slurm::{self,BatchScript, JobState};
use std::process::Command;
use seq_lib::pulse_sequence::MrdToKspaceParams;
use crate::config::{ProjectSettings, Recon};
use mr_data::mrd::{fse_raw_to_cfl};

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

pub fn main_test_cluster(){
    return;
    //let mut bart_settings = BartPicsSettings::quick();
    //bart_settings.set_bart_binary("bart");
    //let bart_settings_file = "/privateShares/wa41/cs_recon_test/reco_settings";
    //let scanner = Host::new("mrs","stejskal");
    let ptab = "/home/wa41/cs_recon_test/stream_CS256_8x_pa18_pb54";
    let vpath = "/d/smis/recon_test_data/_01_46_3b0/volume_index.txt";
    //let mrd_meta_suffix = "_meta.txt";

    let mut recon = Recon::new("grumpy","testrunno0001",vpath,"5xfad","dummyspec");

    let cwd = recon.engine_work_dir.join(format!("{}.work",&recon.run_number));
    if !cwd.exists(){ create_dir_all(&cwd).expect("unable to create specified working directory")}

    let volman_jobs_file = cwd.join("volume-manager-jobs").with_extension("toml");

    let mrd_vol_offset = 0;

    //bart_settings.to_file(bart_settings_file);

    let raw_base_path = Path::new(vpath).parent().unwrap();
    let local_raw_path = Path::new(&cwd).join("raw");
    if !local_raw_path.exists(){create_dir_all(&local_raw_path).expect("issue creating directory");}

    let local_vpath = VolumeIndex::fetch_from(vpath,&recon.scanner.host(),cwd.to_str().unwrap());
    let ready_mrds = VolumeIndex::read_ready(&local_vpath);
    let all_mrds = VolumeIndex::read_all(&local_vpath);

    recon.n_volumes = Some(all_mrds.len());

    let mut r = ResourceList::open(local_raw_path.to_str().unwrap());
    r.set_host(&recon.scanner.host());
    ready_mrds.iter().for_each(|(mrd,_)| {
        let mrdname = Path::new(mrd).file_stem().unwrap().to_str().unwrap();
        let mrd_srcpath = Path::new(raw_base_path).join(mrd);
        let meta_srcpath = Path::new(raw_base_path).join(format!("{}{}",mrdname,&recon.scanner.vol_meta_suffix));
        r.try_add(Resource::new(mrd_srcpath.to_str().unwrap(),""));
        r.try_add(Resource::new(meta_srcpath.to_str().unwrap(),""));
    });
    r.start_transfer();

    /*
        This builds a hashmap of volume managers and their slurm
        job ids that will updated and saved every time this runs
    */
    let mut vol_man_jobs:HashMap<PathBuf,u32>;
    println!("looking for {:?} ...",volman_jobs_file);
    if volman_jobs_file.exists(){
        println!("loading jobs ...");
        let s = utils::read_to_string(volman_jobs_file.to_str().unwrap(),"toml").expect("cannot open file");
        vol_man_jobs = toml::from_str(&s).expect("cannot deserialize hash");
    }
    else{
        println!("creating new jobs file ...");
        vol_man_jobs = HashMap::<PathBuf,u32>::new();
    }
    println!("{:?}",vol_man_jobs);

    /*
        We are assuming a one-to-one mapping a mrd file to a volume manager
        in "volume_index" mode. If a mrd file is available, and a volume manager
        hasn't already been launched, a new volume manager will be instantiated
    */
    all_mrds.iter().for_each(|(index,mrd)| {
        let voldir = cwd.join(index);
        if !voldir.exists(){create_dir_all(&voldir).expect("issue creating directory");}
        if !VolumeManager::exists(voldir.to_str().unwrap()) && mrd.is_some(){
            println!("vol man doesn't exist and mrd is available... submitting new job");
            let mrd_path = local_raw_path.join(mrd.clone().unwrap());
            let job_id = launch_volume_manager_job(voldir.to_str().unwrap(),mrd_path.to_str().unwrap(),&ptab,mrd_vol_offset,&recon.path());
            vol_man_jobs.insert(voldir.clone(),job_id);
        }
    });

    /*
        For every volume manager that has been launched, we find the job state
        from slurm. Note that this is not the volume managers state. This just tells
        us the state of the slurm job (pending,running,completed,failed ... ect)
    */
    let mut job_states = HashMap::<PathBuf,slurm::JobState>::new();
    vol_man_jobs.iter().for_each(|(vol,job)|{
        let jstate = slurm::get_job_state(*job,60);
        job_states.insert(vol.clone(),jstate.clone());
    });

    /*
        If for some reason a volume manager cannot advance state (commonly because it is waiting for
        image scaling information from volume 00), it will return and the slurm state will say "completed."
        In this case, we need to check for inactivity of volume managers that still have work to do. If this is
        the case, we need to restart them, returning a new slurm job id to track
    */
    job_states.iter().for_each(|(vol,state)|{
        if *state == JobState::Completed && !VolumeManager::is_done(vol.to_str().unwrap()){
            //println!("restarting {:?}",vol);
            let workdir = vol.to_str().unwrap();
            let job_id = re_launch_volume_manager_job(workdir);
            vol_man_jobs.insert(vol.clone(),job_id);
        }
    });

    /*
        Here we need to build up some info about the overall progress of the system for reporting and as a
        stop condition for rescheduling.
    */

    //let mc = all_mrds.clone();
    let mut m:Vec<&String> = all_mrds.keys().collect();
    m.sort();
    //println!("sorted idx: {:?}",m);

    let mut state_str = String::new();
    let mut n_completed:usize = 0;
    let states:Vec<VmState> = m.iter().map(|index| {
        let voldir = cwd.join(index);
        let s = VolumeManager::state(voldir.to_str().unwrap());
        if s == VmState::Done {n_completed += 1};
        let slurm_state = job_states.get(&voldir);
        match slurm_state {
            Some(state) => state_str.push_str(&format!("{} : slurm job : {:?}; volume-manager : {:?}\t\n",index,state,&s)),
            None => state_str.push_str(&format!("{} : slurm job : not submitted; volume-manager : {:?}\t\n",index,&s))
        }
        return s;
    }).collect();

    println!("{}",state_str);
    println!("{} completed out of {}.",n_completed,m.len());
    /*
        Here we save information we want to load up the next time this code runs. Right now, this only has
        to be the slurm job ids of the volume managers
    */
    let vol_man_jobs_str = toml::to_string(&vol_man_jobs).expect("cannot serialize hash");
    utils::write_to_file(volman_jobs_file.to_str().unwrap(),"toml",&vol_man_jobs_str);

    // if all work isn't done, schedule this code to run again later (2 minutes seems good?)
    // if n_vols != n_complete{
    //     /* reschedule for later */
    // }

}

pub fn test_updated() {
    // fetch directory from scanner

    let run_number = "N60test";
    let work_dir_name = &format!("{}.work",run_number);

    let remote_user = "wyatt";
    let local_user = "wa41";

    //let remote_host = "10.122.164.152";
    let remote_host = "seba";
    let local_host = "civmcluster1";

    let vol_name = "m00";

    let remote_dir = Path::new("/Users/Wyatt/IdeaProjects/test_data/acq").join(vol_name).to_owned();
    let local_dir = Path::new("/privateShares/wa41").join(work_dir_name).join(vol_name);

    let settings = ProjectSettings::new_template("dummy_project");

    let vmr = VolumeManagerResources::from_dir(&local_dir,vol_name);

    let mut puller_cmd = Command::new("puller_simple");
    puller_cmd.args(
        vec![
            "-oer",
            remote_host,
            remote_dir.to_str().unwrap(),
            local_dir.to_str().unwrap(),
        ]
    );
    if !vmr.exist() {
        let o = puller_cmd.output().expect("puller_simple failed to launch");
        match o.status.success() {
            true => {}
            false => {
                println!("failed to transer directory. Removing residual files ...");
                //todo!(delete any leftover files)
                println!("puller_simple command: {:?}", puller_cmd);
            }
        }
    }

    if !vmr.exist() {
        panic!("puller simple is having issues");
    }

    let mtk = MrdToKspaceParams::from_file(&vmr.kspace_config);


    let kspace_cfl = vmr.raw_mrd.with_extension("");
    let image = kspace_cfl.with_file_name("image");
    fse_raw_to_cfl(&vmr.raw_mrd,&vmr.cs_table,&kspace_cfl,&mtk);

    let mut bart_settings = BartPicsSettings::default();
    bart_settings.max_iter = 2;


    bart_pics(kspace_cfl.to_str().unwrap(),image.to_str().unwrap(),&mut bart_settings);

    let scale = cfl::find_u16_scale(&image,0.9995);

    let img_dir = local_dir.join("_images");
    let vname = format!("{}_{}",run_number,vol_name);

    create_dir_all(&img_dir).expect("cannot create directory");
    cfl::to_civm_raw_u16(&image,&img_dir,&vname,"t9imx",scale);

}


#[derive(Debug)]
struct VolumeManagerResources {
    cs_table:PathBuf,
    raw_mrd:PathBuf,
    kspace_config:PathBuf,
}

impl VolumeManagerResources {
    pub fn new(raw_mrd:&Path,cs_table:&Path,kspace_config:&Path) -> Self {
        Self {
            cs_table:cs_table.to_owned(),
            raw_mrd:raw_mrd.to_owned(),
            kspace_config:kspace_config.to_owned(),
        }
    }
    pub fn from_dir(resource_directory:&Path,prefix:&str) -> Self {
        let mrd = resource_directory.join(prefix).with_extension("mrd");
        let cs_table = resource_directory.join("cs_table");
        let kspace_config = resource_directory.join("mrd_to_kspace").with_extension("mtk");
        Self::new(&mrd,&cs_table,&kspace_config)
    }

    pub fn exist(&self) -> bool {
        self.raw_mrd.exists() && self.kspace_config.exists() && self.cs_table.exists()
    }
}






#[test]
fn test(){
    let r = Recon::new("grumpy","N60400","some/data","5xfad","220304");
}