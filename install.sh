#!/bin/bash

set -e

cargo build

exe_path=$(cargo build --message-format=json | python -c "import json, sys; print ''.join([filename for [filename] in [json.loads(line)['filenames'] for line in sys.stdin.readlines()] if not filename.endswith('.rlib')])")
exe_name=$(basename ${exe_path})
full_path=/usr/local/bin/${exe_name}

set -x

sudo cp ${exe_path} ${full_path}
sudo chown root.root ${full_path}
sudo chmod u+s ${full_path}
