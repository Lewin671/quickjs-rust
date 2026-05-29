// Derived from: test/built-ins/Array/from/iter-map-fn-args.js
var seen = "";
var result = Array.from([10, 20], function(value, index) {
  seen = seen + value + ":" + index + ";";
  return value + index;
});
if (seen !== "10:0;20:1;" || result[0] !== 10 || result[1] !== 21) {
  throw "Array.from should call mapfn with value and index";
}
