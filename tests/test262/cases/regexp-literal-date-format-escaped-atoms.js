// Derived from: test/built-ins/Date/prototype/toString/format.js
var pattern = /^(Sun|Mon|Tue|Wed|Thu|Fri|Sat) [0-9]{2}:[0-9]{2} GMT[+-][0-9]{4}( \(.+\))?$/;

if (!(pattern instanceof RegExp)) {
  throw new Error("escaped parens should parse as a RegExp literal");
}
