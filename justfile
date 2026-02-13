set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

protein-4hhb:
    mkdir -p vizmat-app/assets/proteins
    curl -L "https://files.rcsb.org/download/4HHB.pdb" -o vizmat-app/assets/proteins/4HHB.pdb
    awk 'function trim(s){ gsub(/^ +| +$/, "", s); return s } BEGIN{ n=0 } /^(ATOM  |HETATM)/ { n++; x=substr($0,31,8)+0; y=substr($0,39,8)+0; z=substr($0,47,8)+0; el=trim(substr($0,77,2)); if (el=="") { name=trim(substr($0,13,4)); el=substr(name,1,1) } elem[n]=el; xs[n]=x; ys[n]=y; zs[n]=z } END{ out="vizmat-app/assets/proteins/4HHB.xyz"; print n > out; print "4HHB hemoglobin (from PDB ATOM/HETATM records)" >> out; for(i=1;i<=n;i++) printf "%s %.5f %.5f %.5f\n", elem[i], xs[i], ys[i], zs[i] >> out }' vizmat-app/assets/proteins/4HHB.pdb
    wc -l vizmat-app/assets/proteins/4HHB.xyz
    head -n 5 vizmat-app/assets/proteins/4HHB.xyz

watch:
    cargo watch -x "run"
