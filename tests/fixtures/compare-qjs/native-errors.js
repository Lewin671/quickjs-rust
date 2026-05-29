(function () {
  var constructors = [EvalError, RangeError, ReferenceError, SyntaxError, TypeError, URIError];
  var parts = [];
  for (var i = 0; i < constructors.length; i++) {
    var Ctor = constructors[i];
    var value = new Ctor("boom");
    parts.push(
      typeof Ctor + ":" +
      Ctor.length + ":" +
      Ctor.name + ":" +
      Ctor.prototype.name + ":" +
      (Ctor.prototype.constructor === Ctor) + ":" +
      value.message + ":" +
      value.toString() + ":" +
      (value instanceof Ctor) + ":" +
      (value instanceof Error) + ":" +
      Object.prototype.toString.call(value) + ":" +
      Object.getOwnPropertyDescriptor(value, "message").enumerable
    );
  }
  return parts.join("|");
})()
