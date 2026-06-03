// Derived from: test/built-ins/String/prototype/toLowerCase/supplementary_plane.js
if ("\uD801\uDC00".toLowerCase() !== "\uD801\uDC28") {
  throw "expected Deseret capital long I to lowercase across a UTF-16 surrogate pair";
}
if ("\uD801\uDC27".toLowerCase() !== "\uD801\uDC4F") {
  throw "expected Deseret capital EW to lowercase across a UTF-16 surrogate pair";
}
