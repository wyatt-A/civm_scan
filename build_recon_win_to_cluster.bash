TARGET_CC=x86_64-linux-musl-gcc cargo build -p recon --release --target x86_64-unknown-linux-musl
scp /C/Users/waust/OneDrive/Desktop/civm_scan/target/x86_64-unknown-linux-musl/release/recon wa41@civmcluster1:~/recon_test/
ssh wa41@civmcluster1 "export PIPELINE_QUEUE=high_priority;~/recon_test/recon launch /privateShares/wa41/test_recon.work/m00 /d/dev/221111/ico61/m00 mrs stejskal"