use std::path::{Path,PathBuf};
use std::process::Command;

const SEQ_TEMPLATE:&str = "c:/workstation/data/seq_file";
const GRAD_TEMPLATE:&str = "civm_grad_template.seq";
const RF_TEMPLATE:&str = "civm_rf_template.seq";

fn main() {

    let wd = Path::new("d:/dev/221011");

    let gseq = wd.join("civm_grad.seq");
    let rseq = wd.join("civm_grad.seq");

    let gparam = wd.join("civm_grad_params.txt");
    let rparam = wd.join("civm_rf_params.txt");

    let grad_template = Path::new(SEQ_TEMPLATE).join(GRAD_TEMPLATE);
    let rf_template = Path::new(SEQ_TEMPLATE).join(RF_TEMPLATE);

    println!("{:?}",grad_template);
    println!("{:?}",rf_template);

    let mut grad_cmd = Command::new("seq_gen");
    grad_cmd.args(vec![grad_template,gseq,gparam]);

    let mut rf_cmd = Command::new("seq_gen");
    rf_cmd.args(vec![rf_template,rseq,rparam]);

    println!("{:?}",grad_cmd);
    println!("{:?}",rf_cmd);

}
