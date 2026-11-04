set -eu
temporary="$(mktemp -d)"
name="$(basename "$declaration" .ethos)"
cp "$declaration" "$temporary/$name.ethos"
mkdir "$temporary/output"
"$generator/bin/ethos-zero" "Generate.{ $temporary/$name.ethos $temporary/output }" | grep -q '^Generated'
cmp "$temporary/output/$name.rs" "$committed"
touch "$out"
