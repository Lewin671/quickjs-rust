(function () {
  var map = new Map();
  map.set("a", 1);
  var calls = 0;
  var existing = map.getOrInsert("a", 2);
  var inserted = map.getOrInsert("b", 3);
  var computedExisting = map.getOrInsertComputed("a", function (key) {
    calls = calls + 1;
    return 4;
  });
  var computedInserted = map.getOrInsertComputed("c", function (key) {
    calls = calls + 1;
    return key + "!";
  });
  var overwritten = map.getOrInsertComputed("d", function (key) {
    map.set(key, "inner");
    return "outer";
  });
  return existing + ":" +
    inserted + ":" +
    computedExisting + ":" +
    computedInserted + ":" +
    overwritten + ":" +
    map.get("d") + ":" +
    map.size + ":" +
    calls + ":" +
    Map.prototype.getOrInsert.length + ":" +
    Map.prototype.getOrInsertComputed.length;
})()
