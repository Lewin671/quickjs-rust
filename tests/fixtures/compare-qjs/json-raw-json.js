(function () {
  var raw = JSON.rawJSON(null);
  var parsedObject = JSON.parse(JSON.stringify({
    number: JSON.rawJSON(1.25),
    bool: JSON.rawJSON("true"),
    string: JSON.rawJSON('"text"')
  }));
  var arrayJson = JSON.stringify([JSON.rawJSON(1), JSON.rawJSON(false)]);
  var objectRejected = false;
  try {
    JSON.rawJSON('{"x":1}');
  } catch (error) {
    objectRejected = error instanceof SyntaxError;
  }
  return JSON.rawJSON.length + ":" +
    JSON.isRawJSON.length + ":" +
    JSON.stringify(raw) + ":" +
    JSON.isRawJSON(raw) + ":" +
    JSON.isRawJSON({ rawJSON: "null" }) + ":" +
    (Object.getPrototypeOf(raw) === null) + ":" +
    Object.getOwnPropertyNames(raw).join() + ":" +
    Object.isFrozen(raw) + ":" +
    parsedObject.number + ":" +
    parsedObject.bool + ":" +
    parsedObject.string + ":" +
    arrayJson + ":" +
    objectRejected;
})()
