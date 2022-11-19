TARGET_CC=x86_64-linux-musl-gcc cargo build -p recon --release --target x86_64-unknown-linux-musl
scp /C/Users/waust/OneDrive/Desktop/civm_scan/target/x86_64-unknown-linux-musl/release/recon wa41@civmcluster1:~/recon_test/
ssh wa41@civmcluster1 "export SLURM_DISABLE=1;~/recon_test/recon dti-recon wa41 ~/recon_test/21qa01 N60test \"/d/dev/221118/fse/ico61_6b0\" 221026-1:1"
#ssh wa41@civmcluster1 "export SLURM_DISABLE=1;~/recon_test/recon dti-recon wa41 ~/recon_test/21qa01 N6Orient \"/d/dev/221111/ico61\" dummy_spec"