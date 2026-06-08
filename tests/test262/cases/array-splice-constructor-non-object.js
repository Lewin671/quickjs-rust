// Derived from: test/built-ins/Array/prototype/splice/create-ctor-non-object.js
var values = [null, 1, "string", true];
for (var index = 0; index < values.length; index = index + 1) {
  var array = [];
  array.constructor = values[index];
  var caught = false;
  try {
    array.splice();
  } catch (error) {
    caught = error instanceof TypeError;
  }
  if (!caught) {
    throw "Array.prototype.splice should reject non-object constructors";
  }
}
