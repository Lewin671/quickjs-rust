// Derived from: test/built-ins/encodeURIComponent/S15.1.3.4_A1.1_T1.js
if (encodeURIComponent("a b?x=1&y=\u00E9") !== "a%20b%3Fx%3D1%26y%3D%C3%A9") {
  throw "encodeURIComponent should encode URI component separators";
}
if (encodeURIComponent(String.fromCodePoint(0x1D306)) !== "%F0%9D%8C%86") {
  throw "encodeURIComponent should encode supplementary code points as UTF-8";
}

