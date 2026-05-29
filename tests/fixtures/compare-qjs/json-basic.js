(function () {
  var parsed = JSON.parse("{\"a\":1,\"b\":[true,null,\"x\"],\"c\":\"line\\nfeed\"}");
  var objectJson = JSON.stringify({a: 1, b: [true, null], c: undefined});
  var objectRoundTrip = JSON.parse(objectJson);
  var arrayJson = JSON.stringify(["x", undefined, NaN, Infinity]);
  var arrayRoundTrip = JSON.parse(arrayJson);
  return [
    typeof JSON,
    JSON.parse.length,
    JSON.stringify.length,
    parsed.a,
    parsed.b[0],
    parsed.b[1] === null,
    parsed.c.length,
    objectRoundTrip.a,
    objectRoundTrip.b[0],
    objectRoundTrip.b[1] === null,
    Object.hasOwn(objectRoundTrip, "c"),
    arrayRoundTrip[0],
    arrayRoundTrip[1] === null,
    arrayRoundTrip[2] === null,
    arrayRoundTrip[3] === null,
    JSON.stringify(undefined)
  ].join("|");
})()
