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

  var mapNullAdderThrows = false;
  var savedSet = WeakMap.prototype.set;
  WeakMap.prototype.set = null;
  try {
    new WeakMap([]);
  } catch (error) {
    mapNullAdderThrows = error.constructor === TypeError;
  }
  WeakMap.prototype.set = savedSet;

  var setNullAdderThrows = false;
  var savedAdd = WeakSet.prototype.add;
  WeakSet.prototype.add = null;
  try {
    new WeakSet([]);
  } catch (error) {
    setNullAdderThrows = error.constructor === TypeError;
  }
  WeakSet.prototype.add = savedAdd;

  var nonIterableMapThrows = false;
  var nonIterableSetThrows = false;
  try {
    new WeakMap({});
  } catch (error) {
    nonIterableMapThrows = error.constructor === TypeError;
  }
  try {
    new WeakSet({});
  } catch (error) {
    nonIterableSetThrows = error.constructor === TypeError;
  }

  return mapCalls + ":" + map.get(mapKey) + ":" + setCalls + ":" +
    set.has(setValue) + ":" + mapNullAdderThrows + ":" + setNullAdderThrows +
    ":" + nonIterableMapThrows + ":" + nonIterableSetThrows;
})()
