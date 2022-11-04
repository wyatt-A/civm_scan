exe_destination=/mnt/c/workstation/civm_scan
cargo build -p seq_lib --release --target x86_64-pc-windows-gnu
cp target/x86_64-pc-windows-gnu/release/seq_lib.exe $exe_destination