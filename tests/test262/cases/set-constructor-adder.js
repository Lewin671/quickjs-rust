// Derived from: test/built-ins/Set/set-iterable-calls-add.js
var originalAdd = Set.prototype.add;
var calls = 0;
var receiver;
Set.prototype.add = function(value) {
  calls = calls + 1;
  receiver = this;
  return originalAdd.call(this, value);
};
var set = new Set(["a"]);
if (calls !== 1 || receiver !== set || !set.has("a")) {
  throw new Error("Set constructor should call the prototype add adder");
}

// Derived from: test/built-ins/Set/set-does-not-throw-when-add-is-not-callable.js
Set.prototype.add = null;
if (new Set().size !== 0) {
  throw new Error("Set constructor should not read add without an iterable");
}

// Derived from: test/built-ins/Set/set-iterable-throws-when-add-is-not-callable.js
var caught = false;
try {
  new Set(["b"]);
} catch (error) {
  caught = error instanceof TypeError;
}
if (!caught) {
  throw new Error("Set constructor should reject non-callable add adders");
}
Set.prototype.add = originalAdd;

// Derived from: test/built-ins/Set/set-get-add-method-failure.js
Object.defineProperty(Set.prototype, "add", {
  get: function() {
    throw new TypeError("Set add getter");
  }
});
caught = false;
try {
  new Set([]);
} catch (error) {
  caught = error instanceof TypeError;
}
if (!caught) {
  throw new Error("Set constructor should propagate add getter abrupt completions");
}
