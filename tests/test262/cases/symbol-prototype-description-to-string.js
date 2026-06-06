// Derived from: test/built-ins/Symbol/prototype/toString/toString.js
// Derived from: test/built-ins/Symbol/prototype/description/this-val-symbol.js
var symbol = Symbol("test");
if (symbol.toString() !== "Symbol(test)") { throw; }
if (String(symbol) !== "Symbol(test)") { throw; }
if (symbol.description !== "test") { throw; }

var empty = Symbol();
if (empty.description !== undefined) { throw; }
if (empty.toString() !== "Symbol()") { throw; }

var undef = Symbol(undefined);
if (undef.description !== undefined) { throw; }

var emptyString = Symbol("");
if (emptyString.description !== "") { throw; }

var getter = Object.getOwnPropertyDescriptor(Symbol.prototype, "description").get;
if (getter.call(Symbol("x")) !== "x") { throw; }
if (symbol.valueOf() !== symbol) { throw; }
