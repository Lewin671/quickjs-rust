(function () {
  var object = { first: 1, second: 2 };
  var proto = { inherited: 9 };
  var child = Object.create(proto, { own: { value: 3, enumerable: true } });
  var objectEntries = Object.entries(object);
  var arrayEntries = Object.entries([4, 5]);
  var stringEntries = Object.entries("ab");
  var childEntries = Object.entries(child);
  return [
    Object.entries.length,
    objectEntries[0][0] + "=" + objectEntries[0][1],
    objectEntries[1][0] + "=" + objectEntries[1][1],
    arrayEntries[0][0] + "=" + arrayEntries[0][1],
    arrayEntries[1][0] + "=" + arrayEntries[1][1],
    stringEntries[0][0] + "=" + stringEntries[0][1],
    stringEntries[1][0] + "=" + stringEntries[1][1],
    childEntries.length + ":" + childEntries[0][0] + "=" + childEntries[0][1],
    Object.entries(0).length
  ].join("|");
})()
