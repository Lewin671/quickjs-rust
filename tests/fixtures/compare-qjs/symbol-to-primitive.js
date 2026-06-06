(function () {
  var object = {};
  object[Symbol.toPrimitive] = function (hint) {
    return hint;
  };

  var returnedObjectThrows = false;
  var invalidMethodThrows = false;
  var invalidHintThrows = false;
  var primitiveThisThrows = false;
  var badReturn = {};
  badReturn[Symbol.toPrimitive] = function () {
    return {};
  };
  var badMethod = {};
  badMethod[Symbol.toPrimitive] = 1;
  try {
    badReturn + "";
  } catch (error) {
    returnedObjectThrows = error instanceof TypeError;
  }
  try {
    badMethod + "";
  } catch (error) {
    invalidMethodThrows = error instanceof TypeError;
  }
  try {
    Date.prototype[Symbol.toPrimitive].call({}, "bad");
  } catch (error) {
    invalidHintThrows = error instanceof TypeError;
  }
  try {
    Date.prototype[Symbol.toPrimitive].call(1, "string");
  } catch (error) {
    primitiveThisThrows = error instanceof TypeError;
  }

  var method = Date.prototype[Symbol.toPrimitive];
  var descriptor = Object.getOwnPropertyDescriptor(Date.prototype, Symbol.toPrimitive);
  var log = "";
  var dateLike = {
    toString: function () {
      log += "t";
      return {};
    },
    valueOf: function () {
      log += "v";
      return 5;
    },
  };
  var numberLog = "";
  var numberLike = {
    toString: function () {
      numberLog += "t";
      return "str";
    },
    valueOf: function () {
      numberLog += "v";
      return 7;
    },
  };

  return [
    String(object),
    +object,
    object + "",
    typeof method,
    method.length,
    method.name,
    descriptor.writable,
    descriptor.enumerable,
    descriptor.configurable,
    method.call(dateLike, "default"),
    log,
    method.call(numberLike, "number"),
    numberLog,
    returnedObjectThrows,
    invalidMethodThrows,
    invalidHintThrows,
    primitiveThisThrows,
  ].join(":");
})()
