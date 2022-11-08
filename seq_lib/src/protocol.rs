


/*
    An acquisition is the basic unit of data collection represented with a ppr file
    An experiment is closely related series of acquisitions such as:
        diffsuion series, t1 map series, localizer series (orthoganol views)
    A protocol is a specific queue of experiments or acquisitions
 */



// sequences that implement this trait may return a path to a setup ppr
// that needs to adjusted before continuing
pub trait Setup {
    fn setup(&self) -> Option<PathBuf>;
}


// an acquisition is something that needs to get exported and built
pub trait Acquisition {
    // needs to look at a scan calibration before export
    // may need to point to a setup ppr to inherit settings from
    fn export(&self,path:&Path);
    fn get_setup(&self,path:&Path);
}

// things like running a diffusion scan go here
pub trait Experiment {

}


// struct that holds shared information about scan/specimen setup
// (rf base frequency, rf power)
pub struct ScanCalibration {

}