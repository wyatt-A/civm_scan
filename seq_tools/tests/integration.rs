use std::fs::File;
use std::io::Write;
use seq_tools::pulse::{Hardpulse, Pulse, SincPulse};
use utils::{self,trapz,abs,freq_spectrum,real_to_complex};

#[test]
fn integration(){
    println!("this is an integration test");

    let n_samples = 65536;

    //let s = SincPulse::new(2.0E-3,5);
    let s = Hardpulse::new(140E-6);


    let w = s.render(2);

    let a = abs(&w);

    let p = trapz(&a,Some(2.0E-6));

    println!("auc = {}",p);

    let c = real_to_complex(&w);

    let spec = freq_spectrum(&c,n_samples);
    let spec = utils::normalize(&spec);
    let axis = utils::freq_axis(2.0E-6,n_samples);

    // find where spec drops below 0.5 for full width at half-max
    let mut f_index = 0;
    for i in 0..n_samples-1 {
        if spec[i] >= 0.5 && spec[i+1] < 0.5 {
            f_index = i;
            break
        }
        if i >= axis.len(){
            panic!("error finding full width at half max")
        }
    }

    let bw = axis[f_index] + axis[f_index+1];

    println!("pulse bandwidth = {}",bw);


    let mut f = File::create("../matlab_prototypes/out.csv").expect("cannot create file");
    let mut s = String::new();
    spec.iter().for_each(|val|{
        s.push_str(&format!("{}\n",val));
    });
    f.write_all(s.as_bytes()).expect("cannot write to file");

    let mut f = File::create("../matlab_prototypes/out2.csv").expect("cannot create file");
    let mut s = String::new();
    axis.iter().for_each(|val|{
        s.push_str(&format!("{}\n",val));
    });
    f.write_all(s.as_bytes()).expect("cannot write to file");

    println!("{:?}",p);
}