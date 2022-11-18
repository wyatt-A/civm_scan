exe_destination=/mnt/c/workstation/civm_scan
cargo build -p acquire --release --target x86_64-pc-windows-gnu
cp target/x86_64-pc-windows-gnu/release/acquire.exe $exe_destination