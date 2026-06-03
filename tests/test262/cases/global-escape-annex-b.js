// Derived from: test/annexB/built-ins/escape/empty-string.js
// Derived from: test/annexB/built-ins/escape/unmodified.js
// Derived from: test/annexB/built-ins/escape/escape-above.js

if (escape("") !== "") {
  throw "escape should preserve the empty string";
}

if (escape("AZaz09@*_+-./") !== "AZaz09@*_+-./") {
  throw "escape should preserve Annex B unescaped characters";
}

if (escape(" #éĀ") !== "%20%23%E9%u0100") {
  throw "escape should percent-encode Latin-1 and Unicode code units";
}

if (unescape("%20%23%E9%u0100") !== " #éĀ") {
  throw "unescape should decode percent and unicode escape sequences";
}
