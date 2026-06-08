function trim(value) {
  gsub(/^[[:space:]]+|[[:space:]]+$/, "", value)
  return value
}

function append_entry(field, value) {
  value = trim(value)
  sub(/^-/, "", value)
  value = trim(value)
  sub(/#.*/, "", value)
  value = trim(value)
  if (value == "") return
  if (substr(value, 1, 1) == "[" && substr(value, length(value), 1) == "]") {
    value = substr(value, 2, length(value) - 2)
  }
  count = split(value, parts, ",")
  for (entry_index = 1; entry_index <= count; entry_index++) {
    entry = trim(parts[entry_index])
    if (entry == "") continue
    if ((substr(entry, 1, 1) == "\"" && substr(entry, length(entry), 1) == "\"") ||
        (substr(entry, 1, 1) == "'" && substr(entry, length(entry), 1) == "'")) {
      entry = substr(entry, 2, length(entry) - 2)
    }
    if (field == "flags") {
      flags = flags ? flags ", " entry : entry
    } else if (field == "includes") {
      includes = includes ? includes ", " entry : entry
    } else if (field == "features") {
      features = features ? features ", " entry : entry
    }
  }
}

/\/\*---/{inside=1; next}
/---\*\//{exit}
inside {
  if (match($0, /^[[:space:]]*flags:[[:space:]]*/)) {
    current = "flags"
    value = substr($0, RLENGTH + 1)
    append_entry(current, value)
    next
  }
  if (match($0, /^[[:space:]]*includes:[[:space:]]*/)) {
    current = "includes"
    value = substr($0, RLENGTH + 1)
    append_entry(current, value)
    next
  }
  if (match($0, /^[[:space:]]*features:[[:space:]]*/)) {
    current = "features"
    value = substr($0, RLENGTH + 1)
    append_entry(current, value)
    next
  }
  if ($0 ~ /^[[:space:]]*negative[[:space:]]*:/) {
    negative=1
    current = "negative"
    next
  }
  if (current == "flags" || current == "includes" || current == "features") {
    if ($0 ~ /^[[:space:]]*-/) {
      append_entry(current, $0)
      next
    }
    if ($0 ~ /^[[:space:]]*[A-Za-z0-9_-]+[[:space:]]*:/) {
      current = ""
    }
  }
  if (negative && sub(/^[[:space:]]*phase:[[:space:]]*/, "", $0)) {
    negative_phase=trim($0)
    next
  }
  if (negative && sub(/^[[:space:]]*type:[[:space:]]*/, "", $0)) {
    negative_type=trim($0)
    next
  }
}
END {
  print flags
  print includes
  print features
  print negative_phase
  print negative_type
}
