exe_destination=/mnt/c/workstation/civm_scan
cargo build -p build_sequence --target x86_64-pc-windows-gnu
cp target/x86_64-pc-windows-gnu/debug/build_sequence.exe $exe_destination