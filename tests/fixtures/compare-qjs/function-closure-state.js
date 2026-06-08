(function () {
  function makeCounter() {
    let index = 0;
    return function () {
      index = index + 1;
      return index;
    };
  }

  function makePair() {
    let index = 0;
    return [
      function () {
        index = index + 1;
        return index;
      },
      function () {
        index = index + 1;
        return index;
      },
    ];
  }

  var next = makeCounter();
  var pair = makePair();
  return next() + ":" + next() + ":" + pair[0]() + ":" + pair[1]() + ":" + pair[0]();
})()
