(function () {
  var mapAdded = "";
  var map = new Map([["a", 1], ["b", 2]]);
  map.forEach(function (value, key) {
    if (key === "a") {
      map.set("c", 3);
    }
    mapAdded = mapAdded + key + ":" + value + "|";
  });

  var mapReadded = "";
  map = new Map([["a", 1], ["b", 2]]);
  var mapReaddCount = 0;
  map.forEach(function (value, key) {
    if (mapReaddCount === 0) {
      map.delete("a");
      map.set("a", 3);
    }
    mapReadded = mapReadded + key + ":" + value + "|";
    mapReaddCount = mapReaddCount + 1;
  });

  var setAdded = "";
  var set = new Set([1]);
  set.forEach(function (value) {
    if (value === 1) {
      set.add(2);
    }
    if (value === 2) {
      set.add(3);
    }
    setAdded = setAdded + value + "|";
  });

  var setReadded = "";
  set = new Set(["a", "b"]);
  var setReaddCount = 0;
  set.forEach(function (value) {
    if (setReaddCount === 0) {
      set.delete("a");
      set.add("a");
    }
    setReadded = setReadded + value + "|";
    setReaddCount = setReaddCount + 1;
  });

  return mapAdded + ":" + mapReadded + ":" + setAdded + ":" + setReadded;
})()
