// Derived from: test/built-ins/Symbol/for/create-value.js
// Derived from: test/built-ins/Symbol/keyFor/arg-symbol-registry-hit.js
var canonical = Symbol.for("s");
if (Symbol.for("s") !== canonical) { throw; }
if (Symbol("s") === canonical) { throw; }
if (Symbol.keyFor(canonical) !== "s") { throw; }
if (Symbol.keyFor(Symbol("s")) !== undefined) { throw; }

var numeric = Symbol.for(7);
if (numeric.description !== "7") { throw; }
if (Symbol.keyFor(numeric) !== "7") { throw; }
