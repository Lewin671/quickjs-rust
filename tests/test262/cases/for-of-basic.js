// Derived from: test/language/statements/for-of/array.js
// Derived from: test/language/statements/for-of/Array.prototype.entries.js
// Derived from: test/language/statements/for-of/Array.prototype.keys.js
// Derived from: test/language/statements/for-of/arguments-mapped-aliasing.js
// Derived from: test/language/statements/for-of/arguments-mapped-mutation.js
// Derived from: test/language/statements/for-of/arguments-mapped.js
// Derived from: test/language/statements/for-of/arguments-unmapped-aliasing.js
// Derived from: test/language/statements/for-of/arguments-unmapped-mutation.js
// Derived from: test/language/statements/for-of/arguments-unmapped.js
// Derived from: test/language/statements/for-of/string-astral-truncated.js
// Derived from: test/language/statements/for-of/string-astral.js
// Derived from: test/language/statements/for-of/string-bmp.js
// Derived from: test/built-ins/Map/prototype/getOrInsertComputed/canonical-key-passed-to-callback.js

var total = 0;
for (var value of [1, 2, 3]) {
  total += value;
}
if (total !== 6) {
  throw "for-of should iterate array values";
}

var setSeen = "";
for (const value of new Set(["a", "b"])) {
  setSeen = setSeen + value;
}
if (setSeen !== "ab") {
  throw "for-of should iterate Set values";
}

var mapSeen = "";
for (let entry of new Map([["x", 4], ["y", 5]])) {
  mapSeen = mapSeen + entry[0] + entry[1];
}
if (mapSeen !== "x4y5") {
  throw "for-of should iterate Map entries";
}

var controlled = 0;
for (var item of [1, 2, 3, 4]) {
  if (item === 2) {
    continue;
  }
  if (item === 4) {
    break;
  }
  controlled += item;
}
if (controlled !== 4) {
  throw "for-of should support break and continue";
}

var keySeen = "";
for (var key of ["a", "b"].keys()) {
  keySeen = keySeen + key;
}
if (keySeen !== "01") {
  throw "for-of should iterate array key iterator objects";
}

var entrySeen = "";
for (var arrayEntry of ["a", "b"].entries()) {
  entrySeen = entrySeen + arrayEntry[0] + arrayEntry[1];
}
if (entrySeen !== "0a1b") {
  throw "for-of should iterate array entry iterator objects";
}

var stringSeen = "";
for (var ch of "abc") {
  stringSeen = stringSeen + ch;
}
if (stringSeen !== "abc") {
  throw "for-of should iterate BMP string elements";
}

var astralSeen = "";
for (var astral of "a\uD801\uDC28b\uD801\uDC28") {
  astralSeen = astralSeen + astral + "|";
}
if (astralSeen !== "a|𐐨|b|𐐨|") {
  throw "for-of should keep surrogate pairs together";
}

var truncatedSeen = "";
for (var truncated of "a\uD801b\uD801") {
  truncatedSeen = truncatedSeen + truncated.length + ":" + truncated.charCodeAt(0) + "|";
}
if (truncatedSeen !== "1:97|1:55297|1:98|1:55297|") {
  throw "for-of should preserve unpaired string surrogates";
}

var argumentsSeen = (function() {
  var seen = "";
  for (var value of arguments) {
    seen = seen + value;
  }
  return seen;
}("a", "b", "c"));
if (argumentsSeen !== "abc") {
  throw "for-of should iterate arguments object values";
}

var mutatedArgumentsSeen = (function() {
  var seen = "";
  for (var value of arguments) {
    seen = seen + value;
    arguments[1] = "z";
  }
  return seen;
}("a", "b"));
if (mutatedArgumentsSeen !== "az") {
  throw "for-of should observe arguments object element mutation";
}

var unmappedAliasSeen = (function(a, b) {
  "use strict";
  var seen = "";
  for (var value of arguments) {
    a = "x";
    b = "y";
    seen = seen + value;
  }
  return seen;
}("a", "b"));
if (unmappedAliasSeen !== "ab") {
  throw "strict arguments iteration should not observe parameter mutation";
}

var mappedAliasSeen = (function(a, b, c) {
  var seen = "";
  for (var value of arguments) {
    a = b;
    b = c;
    c = 1;
    seen = seen + value;
  }
  return seen;
}(1, 2, 3));
if (mappedAliasSeen !== "131") {
  throw "mapped arguments iteration should observe parameter mutation";
}
