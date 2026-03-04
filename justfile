set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

check-pdb path:
    @if rg -n -i "^[[:space:]]*<!doctype|^[[:space:]]*<html|^[[:space:]]*<head|^[[:space:]]*<body" {{path}} >/dev/null; then \
        echo "ERROR: {{path}} looks like HTML, not PDB"; \
        exit 1; \
    fi
    @if ! rg -n "^(ATOM  |HETATM)" {{path}} >/dev/null; then \
        echo "ERROR: {{path}} has no ATOM/HETATM records"; \
        exit 1; \
    fi

clean-broken-proteins:
    rm -f vizmat-app/assets/proteins/4V6F.pdb vizmat-app/assets/proteins/4V6F.xyz
    rm -f vizmat-app/assets/proteins/7K00.pdb vizmat-app/assets/proteins/7K00.xyz
    ls -1 vizmat-app/assets/proteins

verify-proteins:
    @for f in vizmat-app/assets/proteins/*.pdb; do \
        just check-pdb "$f"; \
    done
    @for f in vizmat-app/assets/proteins/*.xyz; do \
        n="$(head -n 1 "$f" | tr -d '\r')"; \
        if ! [[ "$n" =~ ^[0-9]+$ ]] || [ "$n" -eq 0 ]; then \
            echo "ERROR: $f has invalid/zero atom count in line 1: '$n'"; \
            exit 1; \
        fi; \
    done
    echo "Protein files verify OK"

protein-4hhb:
    mkdir -p vizmat-app/assets/proteins
    curl -fL "https://files.rcsb.org/download/4HHB.pdb" -o vizmat-app/assets/proteins/4HHB.pdb
    just check-pdb vizmat-app/assets/proteins/4HHB.pdb
    awk 'function trim(s){ gsub(/^ +| +$/, "", s); return s } BEGIN{ n=0 } /^(ATOM  |HETATM)/ { n++; x=substr($0,31,8)+0; y=substr($0,39,8)+0; z=substr($0,47,8)+0; el=trim(substr($0,77,2)); if (el=="") { name=trim(substr($0,13,4)); el=substr(name,1,1) } elem[n]=el; xs[n]=x; ys[n]=y; zs[n]=z } END{ out="vizmat-app/assets/proteins/4HHB.xyz"; print n > out; print "4HHB hemoglobin (from PDB ATOM/HETATM records)" >> out; for(i=1;i<=n;i++) printf "%s %.5f %.5f %.5f\n", elem[i], xs[i], ys[i], zs[i] >> out }' vizmat-app/assets/proteins/4HHB.pdb
    wc -l vizmat-app/assets/proteins/4HHB.xyz
    head -n 5 vizmat-app/assets/proteins/4HHB.xyz

protein-6vxx:
    mkdir -p vizmat-app/assets/proteins
    curl -fL "https://files.rcsb.org/download/6VXX.pdb" -o vizmat-app/assets/proteins/6VXX.pdb
    just check-pdb vizmat-app/assets/proteins/6VXX.pdb
    awk 'function trim(s){ gsub(/^ +| +$/, "", s); return s } BEGIN{ n=0 } /^(ATOM  |HETATM)/ { n++; x=substr($0,31,8)+0; y=substr($0,39,8)+0; z=substr($0,47,8)+0; el=trim(substr($0,77,2)); if (el=="") { name=trim(substr($0,13,4)); el=substr(name,1,1) } elem[n]=el; xs[n]=x; ys[n]=y; zs[n]=z } END{ out="vizmat-app/assets/proteins/6VXX.xyz"; print n > out; print "6VXX structure (from PDB ATOM/HETATM records)" >> out; for(i=1;i<=n;i++) printf "%s %.5f %.5f %.5f\n", elem[i], xs[i], ys[i], zs[i] >> out }' vizmat-app/assets/proteins/6VXX.pdb
    wc -l vizmat-app/assets/proteins/6VXX.xyz
    head -n 5 vizmat-app/assets/proteins/6VXX.xyz

protein-3j3a:
    mkdir -p vizmat-app/assets/proteins
    curl -fL "https://files.rcsb.org/download/3J3A.pdb" -o vizmat-app/assets/proteins/3J3A.pdb
    just check-pdb vizmat-app/assets/proteins/3J3A.pdb
    awk 'function trim(s){ gsub(/^ +| +$/, "", s); return s } BEGIN{ n=0 } /^(ATOM  |HETATM)/ { n++; x=substr($0,31,8)+0; y=substr($0,39,8)+0; z=substr($0,47,8)+0; el=trim(substr($0,77,2)); if (el=="") { name=trim(substr($0,13,4)); el=substr(name,1,1) } elem[n]=el; xs[n]=x; ys[n]=y; zs[n]=z } END{ out="vizmat-app/assets/proteins/3J3A.xyz"; print n > out; print "3J3A ribosome (from PDB ATOM/HETATM records)" >> out; for(i=1;i<=n;i++) printf "%s %.5f %.5f %.5f\n", elem[i], xs[i], ys[i], zs[i] >> out }' vizmat-app/assets/proteins/3J3A.pdb
    test "$(head -n 1 vizmat-app/assets/proteins/3J3A.xyz)" -gt 0
    test "$(wc -l < vizmat-app/assets/proteins/3J3A.xyz)" -eq "$(( $(head -n 1 vizmat-app/assets/proteins/3J3A.xyz) + 2 ))"
    echo "validated 3J3A.xyz: $(head -n 1 vizmat-app/assets/proteins/3J3A.xyz) atoms, $(wc -l < vizmat-app/assets/proteins/3J3A.xyz) lines"
    ls -lh vizmat-app/assets/proteins/3J3A.pdb vizmat-app/assets/proteins/3J3A.xyz
    head -n 5 vizmat-app/assets/proteins/3J3A.xyz

watch:
    cargo watch -x "run"

bench:
    cargo bench -p vizmat-core --bench bond_cache

wasm:
    rustup target add wasm32-unknown-unknown --toolchain nightly-aarch64-apple-darwin
    cd vizmat-app && PATH="$HOME/.cargo/bin:$PATH" NO_COLOR=false trunk serve --port 8082

wasm-release:
    rustup target add wasm32-unknown-unknown --toolchain nightly-aarch64-apple-darwin
    cd vizmat-app && PATH="$HOME/.cargo/bin:$PATH" NO_COLOR=false trunk serve --release --port 8082

download-sdf:
    mkdir -p vizmat-app/assets/compounds
    curl -L "https://files.rcsb.org/ligands/download/NAX_ideal.sdf" -o vizmat-app/assets/compounds/NAX.sdf
    curl -L "https://files.rcsb.org/ligands/download/ESM_ideal.sdf" -o vizmat-app/assets/compounds/ESM.sdf
    curl -L "https://pubchem.ncbi.nlm.nih.gov/rest/pug/compound/cid/14969/SDF" -o vizmat-app/assets/compounds/vancomycin.sdf
    curl -L "https://pubchem.ncbi.nlm.nih.gov/rest/pug/compound/cid/5284373/SDF" -o vizmat-app/assets/compounds/cyclosporin_a.sdf
    awk 'NR==4{print "NAX counts:",$0; exit}' vizmat-app/assets/compounds/NAX.sdf
    awk 'NR==4{print "ESM counts:",$0; exit}' vizmat-app/assets/compounds/ESM.sdf
    awk 'NR==4{print "vancomycin counts:",$0; exit}' vizmat-app/assets/compounds/vancomycin.sdf
    awk 'NR==4{print "cyclosporin_a counts:",$0; exit}' vizmat-app/assets/compounds/cyclosporin_a.sdf
    rg -n "V2000|M  END|\\$\\$\\$\\$" vizmat-app/assets/compounds/NAX.sdf vizmat-app/assets/compounds/ESM.sdf vizmat-app/assets/compounds/vancomycin.sdf vizmat-app/assets/compounds/cyclosporin_a.sdf
    echo "Downloaded all example SDF files to vizmat-app/assets/compounds/"
