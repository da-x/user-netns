#!/bin/bash

set -e

cargo build --release

exe_path=target/release/user-netns
exe_name=$(basename ${exe_path})
full_path=/usr/local/bin/${exe_name}

set -x

sudo cp ${exe_path} ${full_path}
sudo chown root.root ${full_path}
sudo chmod u+s ${full_path}
