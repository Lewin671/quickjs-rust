// Derived from: test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-182.js
// Derived from: test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-185.js
// Derived from: test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-189.js
// Derived from: test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-190.js
// Derived from: test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-193.js
// Derived from: test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-195.js
// Derived from: test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-210.js
// Derived from: test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-211.js
// Derived from: test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-216.js
// Derived from: test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-217.js
// Derived from: test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-218.js
// Derived from: test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-219.js
// Derived from: test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-220.js
// Derived from: test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-221.js
// Derived from: test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-222.js
function checkConstructorPrototype(constructor) {
  var desc = Object.getOwnPropertyDescriptor(constructor, "prototype");
  if (desc.writable !== false) { throw constructor.name + " writable"; }
  if (desc.enumerable !== false) { throw constructor.name + " enumerable"; }
  if (desc.configurable !== false) { throw constructor.name + " configurable"; }
  if (desc.hasOwnProperty("get") !== false) { throw constructor.name + " get"; }
  if (desc.hasOwnProperty("set") !== false) { throw constructor.name + " set"; }
}

checkConstructorPrototype(Object);
checkConstructorPrototype(Function);
checkConstructorPrototype(Array);
checkConstructorPrototype(String);
checkConstructorPrototype(Boolean);
checkConstructorPrototype(Number);
checkConstructorPrototype(Date);
checkConstructorPrototype(RegExp);
checkConstructorPrototype(Error);
checkConstructorPrototype(EvalError);
checkConstructorPrototype(RangeError);
checkConstructorPrototype(ReferenceError);
checkConstructorPrototype(SyntaxError);
checkConstructorPrototype(TypeError);
checkConstructorPrototype(URIError);
