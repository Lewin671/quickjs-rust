// Derived from: test/built-ins/decodeURI/S15.1.3.1_A1.5_T1.js
if (decodeURI("https://example.test/a%20b?x=1&y=%C3%A9") !== "https://example.test/a b?x=1&y=\u00E9") {
  throw "decodeURI should decode UTF-8 percent escapes";
}
if (decodeURI("%3F%23%2F") !== "%3F%23%2F") {
  throw "decodeURI should preserve escaped URI reserved characters";
}

