/\/\*---/{inside=1; next}
/---\*\//{exit}
inside {
  if (sub(/^[[:space:]]*flags:[[:space:]]*/, "", $0)) {
    gsub(/^[[:space:]]*|[[:space:]]*$/, "", $0)
    flags=$0
    next
  }
  if (sub(/^[[:space:]]*includes:[[:space:]]*/, "", $0)) {
    gsub(/^[[:space:]]*|[[:space:]]*$/, "", $0)
    includes=$0
    next
  }
  if (sub(/^[[:space:]]*features:[[:space:]]*/, "", $0)) {
    gsub(/^[[:space:]]*|[[:space:]]*$/, "", $0)
    features=$0
    next
  }
  if ($0 ~ /^[[:space:]]*negative[[:space:]]*:/) {
    negative=1
  }
}
END {
  print flags
  print includes
  print features
  print (negative ? "1" : "")
}
