(function () {
  var setSpread = [...new Set([1, 2])].join("|");
  var mapSpread = [...new Map([["a", 3]])][0].join(":");
  var customSpread = [...{
    [Symbol.iterator]: function () {
      return ["z"][Symbol.iterator]();
    }
  }].join("|");
  var unionSpread = [...new Set([1, 2]).union(new Set([2, 3]))].join("|");
  return setSpread + ":" + mapSpread + ":" + customSpread + ":" + unionSpread;
})()
