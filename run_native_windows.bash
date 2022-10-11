exe_destination=/mnt/c/workstation/civm_scan

cargo build --target x86_64-pc-windows-gnu
cp target/x86_64-pc-windows-gnu/debug/scan_ui.exe $exe_destination

$exe_destination/scan_ui.exe