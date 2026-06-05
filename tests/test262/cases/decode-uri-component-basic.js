// Derived from: test/built-ins/decodeURIComponent/S15.1.3.2_A1.5_T1.js
if (decodeURIComponent("a%20b%3Fx%3D1%26y%3D%C3%A9") !== "a b?x=1&y=\u00E9") {
  throw "decodeURIComponent should decode URI component escapes";
}
var caught = false;
try {
  decodeURIComponent("%E0%A4%A");
} catch (error) {
  caught = error instanceof URIError;
}
if (!caught) {
  throw "decodeURIComponent should throw URIError for malformed escapes";
}

