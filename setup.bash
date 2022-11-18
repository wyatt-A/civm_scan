/mnt/c/workstation/civm_scan/acquire.exe new-setup 5xfad_fse "D:\dev\221118\fse\setup"
/mnt/c/workstation/civm_scan/scan_control.exe setup-ppr "D:\dev\221118\fse\setup\fse_dti_setup.ppr"
/mnt/c/workstation/civm_scan/acquire.exe new 5xfad_b0 "D:\dev\221118\fse\b0_test"
/mnt/c/workstation/civm_scan/acquire.exe apply-setup "D:\dev\221118\fse\setup\fse_dti_setup.ppr" "D:\dev\221118\fse\b0_test" -d=0
/mnt/c/workstation/civm_scan/scan_control.exe run-directory "D:\dev\221118\fse\b0_test" -c=cs_table -d=0