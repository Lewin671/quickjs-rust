// Derived from: test/built-ins/Reflect/apply/arguments-list-is-not-array-like-but-still-valid.js
// Derived from: test/built-ins/Reflect/apply/arguments-list-is-not-array-like.js
// Derived from: test/built-ins/Reflect/construct/arguments-list-is-not-array-like.js
function assertTypeError(callback) {
  try {
    callback();
  } catch (error) {
    if (error instanceof TypeError) {
      return;
    }
    throw "expected TypeError";
  }
  throw "expected throw";
}

function capture() {
  return arguments.length + ":" + arguments[0];
}

var functionArguments = function() {};
Object.defineProperty(functionArguments, "length", {
  get: function() {
    return 1;
  }
});
if (Reflect.apply(capture, null, functionArguments) !== "1:undefined") {
  throw "Reflect.apply should read function argumentsList length as a property";
}

var objectArguments = {};
Object.defineProperty(objectArguments, "length", {
  get: function() {
    return 1;
  }
});
if (Reflect.apply(capture, null, objectArguments) !== "1:undefined") {
  throw "Reflect.apply should read object argumentsList length as a property";
}

assertTypeError(function() {
  Reflect.apply(capture, null);
});
assertTypeError(function() {
  Reflect.apply(capture, null, Symbol());
});
assertTypeError(function() {
  Reflect.apply(capture, null, 1);
});

function Constructed(value) {
  this.value = value;
}
var constructed = Reflect.construct(Constructed, objectArguments);
if (!(constructed instanceof Constructed) || constructed.value !== undefined) {
  throw "Reflect.construct should accept object argumentsList";
}
assertTypeError(function() {
  Reflect.construct(Constructed, 1);
});
