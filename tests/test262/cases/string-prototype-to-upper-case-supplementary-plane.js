// Derived from: test/built-ins/String/prototype/toUpperCase/supplementary_plane.js
if ("\uD801\uDC28".toUpperCase() !== "\uD801\uDC00") {
  throw "expected Deseret small long I to uppercase across a UTF-16 surrogate pair";
}
if ("\uD801\uDC4F".toUpperCase() !== "\uD801\uDC27") {
  throw "expected Deseret small EW to uppercase across a UTF-16 surrogate pair";
}
