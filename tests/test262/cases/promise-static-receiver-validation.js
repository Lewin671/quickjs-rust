// Derived from: test/built-ins/Promise/try/ctx-non-ctor.js
// Derived from: test/built-ins/Promise/try/ctx-non-object.js
// Derived from: test/built-ins/Promise/withResolvers/ctx-non-ctor.js
// Derived from: test/built-ins/Promise/withResolvers/ctx-non-object.js
var nonConstructors = [undefined, null, 86, "string", true, Symbol(), eval];

for (var i = 0; i < nonConstructors.length; i++) {
  var value = nonConstructors[i];
  try {
    Promise.try.call(value, function() {});
    throw "Promise.try should reject non-constructor receivers";
  } catch (error) {
    if (String(error).indexOf("TypeError") !== 0) {
      throw error;
    }
  }

  try {
    Promise.withResolvers.call(value);
    throw "Promise.withResolvers should reject non-constructor receivers";
  } catch (error) {
    if (String(error).indexOf("TypeError") !== 0) {
      throw error;
    }
  }
}
