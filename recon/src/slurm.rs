use std::collections::HashMap;
use std::io::{Write, Read};
use std::process::Command;
use std::path::{Path, PathBuf};
use std::fs::File;

#[derive(PartialEq,Eq,Debug,Clone)]
pub enum JobState {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Unknown,
}

impl JobState {
    pub fn encode(state_str: &str) -> Self {
        match state_str {
            "pending" => JobState::Pending,
            "cancelled" => JobState::Cancelled,
            "failed" => JobState::Failed,
            "running" => JobState::Running,
            "completed" => JobState::Completed,
            _ => JobState::Unknown,
        }
    }
    pub fn decode(&self) -> String {
        format!("{:?}",&self)
    }
}



pub struct SBatchOpts{
    // convert to hash map/set (time permitting :D)
    reservation:String,
    pub(crate) job_name:String,
    no_requeue:bool,
    pub memory:Option<String>,
    pub output:String,
    pub partition:String,
    pub start_delay_sec:Option<u32>,
    pub email:Option<String>,
}

pub struct BatchScript{
    preamble:String,
    pub options:SBatchOpts,
    pub commands:Vec<String>,
    pub job_id:Option<u32>
}

impl SBatchOpts{
    pub fn new(job_name:&str) -> SBatchOpts {
        return SBatchOpts{
            job_name:job_name.to_string(),
            reservation:String::from(""),
            memory:Some(String::from("80G")),
            no_requeue: true,
            output:String::from(""),
            partition:String::from(""),
            start_delay_sec:None,
            email:None
        };
    }
    pub fn print(&self) -> String {
        let mut opts = Vec::<String>::new();
        opts.push(format!("#SBATCH --job-name={}",&self.job_name));
        if !self.reservation.is_empty(){opts.push(format!("#SBATCH --reservation={}",&self.reservation))}
        if self.no_requeue{ opts.push("#SBATCH --no-requeue".to_string())}
        if !self.output.is_empty(){ opts.push(format!("#SBATCH --output={}",&self.output))}
        if !self.partition.is_empty(){ opts.push(format!("#SBATCH --partition={}",&self.partition))}
        if self.start_delay_sec.is_some() { opts.push(format!("#SBATCH --begin=now+{}",self.start_delay_sec.unwrap()))}
        opts.push(format!("#SBATCH --mem={}",self.memory.clone().expect("memory request must be specified")));
        match &self.email {
            Some(email) => {
                opts.push(String::from("#SBATCH --mail-type=ALL"));
                opts.push(format!("#SBATCH --mail-user={}",email))
            }
            None => {}
        }
        return opts.join("\n");
    }
}

impl BatchScript{
    pub fn new(job_name:&str,commands:&Vec<Command>) -> BatchScript {
        let preamble = "#!/usr/bin/env bash".to_string();
        let opts = SBatchOpts::new(job_name);
        let command:Vec<String> = commands.iter().map(|cmd| format!("{:?}",cmd)).collect();
        return BatchScript {
            preamble:preamble,
            options:opts,
            commands:command,
            job_id:None
        }
    }

    pub fn commands(&self) -> String{
        return self.commands.join("\n");
    }

    pub fn print(&self) -> String {
        let mut elems = Vec::<String>::new();
        elems.push(self.preamble.clone());
        elems.push(self.options.print());
        elems.push(String::from("hostname"));
        elems.push(self.commands());
        return elems.join("\n");
    }

    pub fn write(&self,location:&Path) -> PathBuf{
        let mut fname = location.to_owned();
        fname = fname.join(&self.options.job_name).with_extension("bash");
        let mut f = File::create(&fname).expect("cannot create file");
        f.write_all(self.print().as_bytes()).expect("trouble writing to file");
        return fname;
    }

    pub fn submit_later(&mut self, write_location:&Path,seconds_later:u32) -> u32{
        self.options.start_delay_sec = Some(seconds_later);
        let path = self.write(write_location);
        let mut cmd = Command::new("sbatch");
        cmd.arg(path);
        let o = cmd.output().expect("failed to run command");
        let response = String::from_utf8_lossy(&o.stdout);
        let jid = BatchScript::response_to_job_id(&response);
        //println!("job id: {}",jid);
        self.job_id = Some(jid);
        return jid;
    }

    pub fn submit_now(&mut self, write_location:&Path) -> u32{
        let path = self.write(write_location);
        let mut cmd = Command::new("sbatch");
        cmd.arg(path);
        let o = cmd.output().expect("failed to run command");
        let response = String::from_utf8_lossy(&o.stdout);
        let jid = BatchScript::response_to_job_id(&response);
        //println!("job id: {}",jid);
        self.job_id = Some(jid);
        return jid;
    }

    pub fn get_details(&self){
        match self.job_id {
            Some(jid) => {
                let mut cmd = Command::new("squeue");
                cmd.arg("-j");
                cmd.arg(jid.to_string());
                //cmd.arg("--format=avevmsize");
                let o =cmd.output().expect("process failed");
                if o.status.success(){
                    println!("return text: {}",String::from_utf8_lossy(&o.stdout));
                }
            }
            None => {
                println!("{} job has not been successfully submitted",self.options.job_name);
            }
        }
    }

    pub fn check_state(&self){
        match self.job_id {
            Some(jid) => {
                let mut cmd = Command::new("sacct");
                cmd.arg("-j");
                cmd.arg(jid.to_string());
                cmd.arg("--format=state,reqmem");
                let o = cmd.output().expect("process failed");
                if o.status.success(){
                    let r = String::from_utf8_lossy(&o.stdout);
                    println!("{}",r);
                    let strs:Vec<&str> = r.lines().collect();
                    let fields = strs[0];
                    println!("{}",fields);
                }else{
                    panic!("command unsuccessful");
                }
            }
            None => {
                println!("{} job has not been successfully submitted",self.options.job_name);
            }
        }
    }

    pub fn output(&self) -> String{
        let p = Path::new(&self.options.output);
        let mut f = File::open(p).expect("cannot open file");
        let mut s = String::new();
        f.read_to_string(&mut s).expect("problem reading file");
        return s;
    }

    fn response_to_job_id(resp:&str) -> u32{
        let nums:Vec<u32> = resp.split(" ").flat_map(|str| str.replace("\n","").parse()).collect();
        if nums.len() == 0 {panic!("no job ids found in slurm response")}
        if nums.len() != 1 {panic!("multiple ids found in slurm response")};
        return nums[0];
    }

}

pub fn is_running(job_id:u32){
    let mut cmd = Command::new("squeue");
    cmd.arg("-j");
    cmd.arg(job_id.to_string());
    let r = cmd.spawn().unwrap();
    let o =r.wait_with_output().unwrap();
    println!("{:?}",o.stdout);
}

pub fn cancel(job_id:u32) -> bool {
    let mut cmd = Command::new("scancel");
    cmd.arg(&format!("{}",job_id));
    match cmd.output(){
        Ok(o) => o.status.success(),
        Err(_) => {
            println!("scancel not found");
            false
        }
    }
}

pub fn get_job_state(job_id:u32,n_tries:u16) -> JobState {
    let mut cmd = Command::new("sacct");
    cmd.arg("-j").arg(job_id.to_string()).arg("--format").arg("state");
    let o = cmd.output().unwrap();
    let s = std::str::from_utf8(&o.stdout).unwrap().to_ascii_lowercase();
    let lines:Vec<&str> = s.lines().collect();
    let mut statestr = lines[lines.len()-1];
    statestr = statestr.trim();
    return match statestr {
        "pending" => JobState::Pending,
        "cancelled" => JobState::Cancelled,
        "failed" => JobState::Failed,
        "running" => JobState::Running,
        "completed" => JobState::Completed,
        _ => {
            if n_tries > 0 {
                std::thread::sleep(std::time::Duration::from_millis(1000));
                return get_job_state(job_id,n_tries-1);
            }else{
                println!("gave up waiting for job state for job id: {}",job_id);
                return JobState::Unknown;
            }
        }
    };
}


pub fn job_state(job_id:u32) -> Option<JobState> {
    match JobCollection::from_id(job_id).state().get(&job_id) {
        Some(job_state) => Some(job_state.clone()),
        None => None
    }
}


pub struct JobCollection {
    job_ids:Vec<u32>,
}

impl JobCollection {
    pub fn new() -> Self {
        Self {
            job_ids:Vec::<u32>::new()
        }
    }
    pub fn from_array(job_ids:&Vec<u32>) -> Self {
        Self {
            job_ids:job_ids.clone()
        }
    }

    pub fn from_id(job_id:u32) -> Self {
        Self {
            job_ids:vec![job_id]
        }
    }

    pub fn push(&mut self,job_id:u32){
        self.job_ids.push(job_id);
    }

    /*
        Checks that all job ids are known to slurm and have a state
     */
    pub fn is_valid(&self) -> bool {
        let hash = self.state();
        for id in self.job_ids.iter(){
            match hash.contains_key(id) {
                false => return false,
                true => continue,
            }
        }
        true
    }

    pub fn is_complete(&self) -> bool {
        let hash = self.state();
        for state in hash.values() {
            match state {
                JobState::Completed => continue,
                _=> return false
            }
        }
        true
    }

    /*
        Get the state of multiple jobs in the form of a hash map
    */
    pub fn state(&self) -> HashMap<u32,JobState> {
        let jid_str:Vec<String> = self.job_ids.iter().map(|j_id| j_id.to_string()).collect();
        let jid_str = jid_str.join(",");
        let mut cmd = Command::new("sacct");
        cmd.args(vec!["--parsable2","--noheader","--format=job,state","-j",&jid_str]);
        let o = cmd.output().expect("sacct failed to launch");
        let mut h = HashMap::<u32,JobState>::new();
        match o.status.success(){
            true => {
                let stdout = String::from_utf8(o.stdout).expect("unable to parse stdout");
                stdout.lines().for_each(|line|{
                    let split = line.split_once('|').expect("delimeter | not found in sacct response");
                    // only parses job ids that are just a number (no extensions)
                    match split.0.parse::<u32>() {
                        Ok(job_id) => {
                            let j_state = JobState::encode(&split.1.to_ascii_lowercase());
                            h.insert(job_id, j_state);
                        }
                        _=> {/*ignore errors*/}
                    }
                });
            }
            false => panic!("sacct failed")
        }
        h
    }
}


/*
simple check to see that slurm is installed on the system
sinfo -V
*/
pub fn is_installed() -> bool{
    let mut cmd = Command::new("sinfo");
    cmd.arg("-V");
    match cmd.output(){
        Err(_) => {
            println!("slurm not found on system");
            false
        }
        Ok(_) => true
    }
}