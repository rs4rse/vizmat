set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

check-pdb file:
	@test -f {{file}}
	@if rg -n -i "^[[:space:]]*<!doctype|^[[:space:]]*<html|^[[:space:]]*<head|^[[:space:]]*<body" {{file}} >/dev/null; then \
		echo "ERROR: {{file}} looks like HTML, not PDB"; \
		exit 1; \
	fi
	@if ! rg -n "^(ATOM  |HETATM)" {{file}} >/dev/null; then \
		echo "ERROR: {{file}} has no ATOM/HETATM records"; \
		exit 1; \
	fi

download-pdb id:
	tmp="$(mktemp)"; \
	curl -fsSL "https://files.rcsb.org/download/{{id}}.pdb" -o "$tmp"; \
	just check-pdb "$tmp"; \
	mv "$tmp" "proteins/{{id}}.pdb"; \
	echo "Saved proteins/{{id}}.pdb"

make-xyz id note:
	@just check-pdb "proteins/{{id}}.pdb"
	awk 'BEGIN {n=0} \
	/^(ATOM  |HETATM)/ {n++; lines[n]=$0} \
	END { \
		print n; \
		print "{{note}}"; \
		for (i=1; i<=n; i++) { \
			line=lines[i]; \
			element=substr(line,77,2); gsub(/ /,"",element); \
			if (element == "") {element=substr(line,13,2); gsub(/ /,"",element)}; \
			x=substr(line,31,8)+0; y=substr(line,39,8)+0; z=substr(line,47,8)+0; \
			printf "%s %.3f %.3f %.3f\n", element, x, y, z; \
		} \
	}' "proteins/{{id}}.pdb" > "proteins/{{id}}.xyz"
	@head -n 2 "proteins/{{id}}.xyz"

verify-proteins:
	@for f in proteins/*.pdb; do \
		if rg -n -i "^[[:space:]]*<!doctype|^[[:space:]]*<html|^[[:space:]]*<head|^[[:space:]]*<body" "$f" >/dev/null; then \
			echo "ERROR: $f looks like HTML, not PDB"; \
			exit 1; \
		fi; \
		if ! rg -n "^(ATOM  |HETATM)" "$f" >/dev/null; then \
			echo "ERROR: $f has no ATOM/HETATM records"; \
			exit 1; \
		fi; \
	done
	@for f in proteins/*.xyz; do \
		n="$(head -n 1 "$f" | tr -d '\r')"; \
		if ! [[ "$n" =~ ^[0-9]+$ ]] || [ "$n" -eq 0 ]; then \
			echo "ERROR: $f has invalid/zero atom count in line 1: '$n'"; \
			exit 1; \
		fi; \
	done
	@echo "Protein files verify OK"

list-txt:
	printf "compounds/water.xyz\n" > list.txt
	find compounds proteins -type f ! -name ".DS_Store" ! -path "compounds/water.xyz" | sort >> list.txt
	wc -l list.txt
	head -n 6 list.txt
