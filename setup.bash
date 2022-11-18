/mnt/c/workstation/civm_scan/acquire.exe new-setup 5xfad_fse "D:\dev\221118\fse\setup"
/mnt/c/workstation/civm_scan/scan_control.exe setup-ppr "D:\dev\221118\fse\setup\fse_dti_setup.ppr"
/mnt/c/workstation/civm_scan/acquire.exe new-diffusion-experiment 5xfad_fse "D:\dev\221118\fse\ico61_6b0" "C:\workstation\data\diffusion_table\ICO61_6b0.txt"
/mnt/c/workstation/civm_scan/acquire.exe apply-setup "D:\dev\221118\fse\setup\fse_dti_setup.ppr" "D:\dev\221118\fse\ico61_6b0" -d=1
/mnt/c/workstation/civm_scan/scan_control.exe run-directory "D:\dev\221118\fse\ico61_6b0" -c=cs_table -d=1


/mnt/c/workstation/civm_scan/acquire.exe new-setup 5xfad_se "D:\dev\221118\se\setup"
/mnt/c/workstation/civm_scan/scan_control.exe setup-ppr "D:\dev\221118\se\setup\se_dti_setup.ppr"
/mnt/c/workstation/civm_scan/acquire.exe new-diffusion-experiment 5xfad_se "D:\dev\221118\se\ico61_6b0" "C:\workstation\data\diffusion_table\ICO61_6b0.txt"
/mnt/c/workstation/civm_scan/acquire.exe apply-setup "D:\dev\221118\se\setup\se_dti_setup.ppr" "D:\dev\221118\se\ico61_6b0" -d=1
/mnt/c/workstation/civm_scan/scan_control.exe run-directory "D:\dev\221118\se\ico61_6b0" -c=cs_table -d=1