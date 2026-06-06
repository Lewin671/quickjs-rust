// Derived from: test/built-ins/RegExp/prototype/Symbol.split/species-ctor.js
var flagsArg;
var re = {};
re.flags = "i";
re.constructor = function() {};
re.constructor[Symbol.species] = function(_, flags) {
  flagsArg = flags;
  return /b/y;
};

var result = RegExp.prototype[Symbol.split].call(re, "abc");

if (result.length !== 2 || result[0] !== "a" || result[1] !== "c") {
  throw "RegExp.prototype[Symbol.split] should use the species splitter";
}

if (flagsArg !== "iy") {
  throw "RegExp.prototype[Symbol.split] should append the sticky flag for species constructors";
}
