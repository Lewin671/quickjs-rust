// Derived from: test/language/expressions/addition/coerce-symbol-to-prim-invocation.js
// Derived from: test/language/expressions/addition/coerce-symbol-to-prim-return-obj.js
// Derived from: test/language/expressions/addition/coerce-symbol-to-prim-return-prim.js
// Derived from: test/language/expressions/addition/get-symbol-to-prim-err.js
var object = {};
object[Symbol.toPrimitive] = function (hint) {
  return hint;
};
if (String(object) !== "string") {
  throw "expected string conversion to pass string hint";
}
if (String(+object) !== "NaN") {
  throw "expected number conversion to pass number hint";
}
if (object + "" !== "default") {
  throw "expected addition conversion to pass default hint";
}

var returnedObject = {};
returnedObject[Symbol.toPrimitive] = function () {
  return {};
};
var objectReturnThrows = false;
try {
  returnedObject + "";
} catch (error) {
  objectReturnThrows = error instanceof TypeError;
}
if (objectReturnThrows !== true) {
  throw "expected object return from Symbol.toPrimitive to throw";
}

var invalidMethod = {};
invalidMethod[Symbol.toPrimitive] = 1;
var invalidMethodThrows = false;
try {
  invalidMethod + "";
} catch (error) {
  invalidMethodThrows = error instanceof TypeError;
}
if (invalidMethodThrows !== true) {
  throw "expected non-callable Symbol.toPrimitive to throw";
}
