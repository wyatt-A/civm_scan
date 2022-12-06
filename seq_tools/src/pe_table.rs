use std::fs::File;
use std::io::Read;
use std::path::Path;

// #[test]
// fn test() {
//     let table = r"C:\workstation\data\petableCS_stream\stream_CS480_8x_pa18_pb54";
//
//     let table_path = Path::new(table);
//
//     let mut s = String::new();
//
//     let mut f = File::open(table_path).expect("cannot open file");
//     f.read_to_string(&mut s).expect("cannot read from file");
//
//     // we are expecting a list of integers
//
//     let nums:Vec<i16> = s.lines().flat_map(|line| line.parse()).collect();
//
//     //let mut coords = Vec::<Coordinate>::new();
//
// }

struct Coordinate {
    kx:i16,
    ky:i16
}