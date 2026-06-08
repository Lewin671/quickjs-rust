// Derived from: test/built-ins/Array/prototype/concat/Array.prototype.concat_sloppy-arguments.js
// Derived from: test/built-ins/Array/prototype/concat/Array.prototype.concat_holey-sloppy-arguments.js
// Derived from: test/built-ins/Array/prototype/concat/Array.prototype.concat_sloppy-arguments-with-dupes.js

var args = (function(a, b, c) {
  return arguments;
})(1, 2, 3);
args[Symbol.isConcatSpreadable] = true;

var spread = [].concat(args, args);
if (spread.length !== 6) {
  throw "concat should spread arguments object length";
}
if (spread[0] !== 1 || spread[1] !== 2 || spread[2] !== 3 ||
    spread[3] !== 1 || spread[4] !== 2 || spread[5] !== 3) {
  throw "concat should read spreadable arguments object indexed values";
}

Object.defineProperty(args, "length", { value: 6 });
var expanded = [].concat(args);
if (expanded.length !== 6) {
  throw "concat should observe arguments object length changes";
}
if (expanded[0] !== 1 || expanded[1] !== 2 || expanded[2] !== 3 ||
    expanded[3] !== undefined || expanded[4] !== undefined || expanded[5] !== undefined) {
  throw "concat should fill missing arguments indexes with undefined values";
}

var holey = (function(a) {
  return arguments;
})(1, 2, 3);
delete holey[1];
holey[Symbol.isConcatSpreadable] = true;

var holeySpread = [].concat(holey, holey);
if (holeySpread.length !== 6) {
  throw "concat should preserve spreadable arguments length after delete";
}
if (holeySpread[0] !== 1 || holeySpread[2] !== 3 ||
    holeySpread[3] !== 1 || holeySpread[5] !== 3) {
  throw "concat should copy remaining arguments indexes after delete";
}
if (holeySpread.hasOwnProperty("1") || holeySpread.hasOwnProperty("4")) {
  throw "concat should preserve holes from deleted arguments indexes";
}

var dupes = (function(a, a, a) {
  return arguments;
})(1, 2, 3);
dupes[Symbol.isConcatSpreadable] = true;
var dupedSpread = [].concat(dupes, dupes);
if (dupedSpread.join("|") !== "1|2|3|1|2|3") {
  throw "concat should spread unmapped duplicate-parameter arguments values";
}
