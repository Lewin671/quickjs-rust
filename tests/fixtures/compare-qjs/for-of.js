(function () {
  var total = 0;
  for (var value of [1, 2, 3]) {
    total += value;
  }

  var setSeen = "";
  for (const value of new Set(["a", "b"])) {
    setSeen = setSeen + value;
  }

  var mapSeen = "";
  for (let entry of new Map([["x", 4], ["y", 5]])) {
    mapSeen = mapSeen + entry[0] + entry[1];
  }

  var controlled = 0;
  for (var item of [1, 2, 3, 4]) {
    if (item === 2) {
      continue;
    }
    if (item === 4) {
      break;
    }
    controlled += item;
  }

  var target = {};
  for (target.value of [8, 9]) {}

  return total + ":" + setSeen + ":" + mapSeen + ":" + controlled + ":" + target.value;
})()
