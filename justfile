set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

protein-4hhb:
    mkdir -p vizmat-app/assets/proteins
    curl -L "https://files.rcsb.org/download/4HHB.pdb" -o vizmat-app/assets/proteins/4HHB.pdb
    awk 'function trim(s){ gsub(/^ +| +$/, "", s); return s } BEGIN{ n=0 } /^(ATOM  |HETATM)/ { n++; x=substr($0,31,8)+0; y=substr($0,39,8)+0; z=substr($0,47,8)+0; el=trim(substr($0,77,2)); if (el=="") { name=trim(substr($0,13,4)); el=substr(name,1,1) } elem[n]=el; xs[n]=x; ys[n]=y; zs[n]=z } END{ out="vizmat-app/assets/proteins/4HHB.xyz"; print n > out; print "4HHB hemoglobin (from PDB ATOM/HETATM records)" >> out; for(i=1;i<=n;i++) printf "%s %.5f %.5f %.5f\n", elem[i], xs[i], ys[i], zs[i] >> out }' vizmat-app/assets/proteins/4HHB.pdb
    wc -l vizmat-app/assets/proteins/4HHB.xyz
    head -n 5 vizmat-app/assets/proteins/4HHB.xyz

protein-6vxx:
    mkdir -p vizmat-app/assets/proteins
    curl -L "https://files.rcsb.org/download/6VXX.pdb" -o vizmat-app/assets/proteins/6VXX.pdb
    awk 'function trim(s){ gsub(/^ +| +$/, "", s); return s } BEGIN{ n=0 } /^(ATOM  |HETATM)/ { n++; x=substr($0,31,8)+0; y=substr($0,39,8)+0; z=substr($0,47,8)+0; el=trim(substr($0,77,2)); if (el=="") { name=trim(substr($0,13,4)); el=substr(name,1,1) } elem[n]=el; xs[n]=x; ys[n]=y; zs[n]=z } END{ out="vizmat-app/assets/proteins/6VXX.xyz"; print n > out; print "6VXX structure (from PDB ATOM/HETATM records)" >> out; for(i=1;i<=n;i++) printf "%s %.5f %.5f %.5f\n", elem[i], xs[i], ys[i], zs[i] >> out }' vizmat-app/assets/proteins/6VXX.pdb
    wc -l vizmat-app/assets/proteins/6VXX.xyz
    head -n 5 vizmat-app/assets/proteins/6VXX.xyz

protein-3j3a:
    mkdir -p vizmat-app/assets/proteins
    curl -L "https://files.rcsb.org/download/3J3A.pdb" -o vizmat-app/assets/proteins/3J3A.pdb
    awk 'function trim(s){ gsub(/^ +| +$/, "", s); return s } BEGIN{ n=0 } /^(ATOM  |HETATM)/ { n++; x=substr($0,31,8)+0; y=substr($0,39,8)+0; z=substr($0,47,8)+0; el=trim(substr($0,77,2)); if (el=="") { name=trim(substr($0,13,4)); el=substr(name,1,1) } elem[n]=el; xs[n]=x; ys[n]=y; zs[n]=z } END{ out="vizmat-app/assets/proteins/3J3A.xyz"; print n > out; print "3J3A ribosome (from PDB ATOM/HETATM records)" >> out; for(i=1;i<=n;i++) printf "%s %.5f %.5f %.5f\n", elem[i], xs[i], ys[i], zs[i] >> out }' vizmat-app/assets/proteins/3J3A.pdb
    test "$(head -n 1 vizmat-app/assets/proteins/3J3A.xyz)" -gt 0
    test "$(wc -l < vizmat-app/assets/proteins/3J3A.xyz)" -eq "$(( $(head -n 1 vizmat-app/assets/proteins/3J3A.xyz) + 2 ))"
    echo "validated 3J3A.xyz: $(head -n 1 vizmat-app/assets/proteins/3J3A.xyz) atoms, $(wc -l < vizmat-app/assets/proteins/3J3A.xyz) lines"
    ls -lh vizmat-app/assets/proteins/3J3A.pdb vizmat-app/assets/proteins/3J3A.xyz
    head -n 5 vizmat-app/assets/proteins/3J3A.xyz

watch:
    cargo watch -x "run"
