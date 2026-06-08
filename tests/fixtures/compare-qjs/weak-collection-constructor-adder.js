(function () {
  var mapKey = {};
  var mapCalls = 0;
  var originalSet = WeakMap.prototype.set;
  WeakMap.prototype.set = function (key, value) {
    mapCalls = mapCalls + 1;
    return originalSet.call(this, key, value);
  };
  var map = new WeakMap([[mapKey, 7]]);

  var setValue = {};
  var setCalls = 0;
  var originalAdd = WeakSet.prototype.add;
  WeakSet.prototype.add = function (value) {
    setCalls = setCalls + 1;
    return originalAdd.call(this, value);
  };
  var set = new WeakSet([setValue]);

  return mapCalls + ":" + map.get(mapKey) + ":" + setCalls + ":" + set.has(setValue);
})()
