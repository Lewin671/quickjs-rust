// Derived from: test/built-ins/encodeURI/S15.1.3.3_A1.1_T1.js
if (encodeURI("https://example.test/a b?x=1&y=\u00E9#frag") !== "https://example.test/a%20b?x=1&y=%C3%A9#frag") {
  throw "encodeURI should encode spaces and UTF-8 characters while preserving URI syntax";
}
if (encodeURI(";/?:@&=+$,#") !== ";/?:@&=+$,#") {
  throw "encodeURI should preserve URI reserved characters";
}

