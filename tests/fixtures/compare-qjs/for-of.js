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

  var keySeen = "";
  for (var key of ["a", "b"].keys()) {
    keySeen = keySeen + key;
  }

  var entrySeen = "";
  for (var arrayEntry of ["a", "b"].entries()) {
    entrySeen = entrySeen + arrayEntry[0] + arrayEntry[1];
  }

  var setIteratorSeen = "";
  for (var setValue of new Set(["x", "y"]).values()) {
    setIteratorSeen = setIteratorSeen + setValue;
  }

  var stringSeen = "";
  for (var ch of "a\uD801\uDC28b") {
    stringSeen = stringSeen + ch + "|";
  }

  var truncatedStringSeen = "";
  for (var part of "a\uD801b") {
    truncatedStringSeen = truncatedStringSeen + part.length + ":" + part.charCodeAt(0) + "|";
  }

  return total + ":" + setSeen + ":" + mapSeen + ":" + controlled + ":" +
    target.value + ":" + keySeen + ":" + entrySeen + ":" + setIteratorSeen + ":" +
    stringSeen + ":" + truncatedStringSeen;
})()
