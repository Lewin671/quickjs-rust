// Derived from: test/built-ins/Object/values/getter-adding-key.js
var object = {
  a: "A",
  get b() {
    this.c = "C";
    return "B";
  }
};
var values = Object.values(object);
if (values.length !== 2) {
  throw "expected Object.values to snapshot enumerable keys";
}
if (values[0] !== "A" || values[1] !== "B") {
  throw "expected Object.values to invoke getter values";
}
