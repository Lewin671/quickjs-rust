(function () {
  var map = new Map([["attr", 1], ["foo", 2], ["foo", 3]]);
  var set = new Set([1, 2, 2]);
  var stringSet = new Set("aba");
  return map.size + ":" +
    map.get("attr") + ":" +
    map.get("foo") + ":" +
    set.size + ":" +
    set.has(1) + ":" +
    set.has(2) + ":" +
    stringSet.size + ":" +
    stringSet.has("a") + ":" +
    stringSet.has("b");
})()
