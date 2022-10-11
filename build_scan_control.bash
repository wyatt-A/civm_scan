exe_destination=/mnt/c/workstation/civm_scan
cargo build -p scan_control --target x86_64-pc-windows-gnu
cp target/x86_64-pc-windows-gnu/debug/scan_control.exe $exe_destination