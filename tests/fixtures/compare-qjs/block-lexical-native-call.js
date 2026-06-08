(function () {
  {
    const s1 = new Set([1, 2]);
    const s2 = new Set([2, 3]);
    var first = Object.is(s1.size, 2) + ":" + [...s1.union(s2)].join("|");
  }
  {
    const s1 = new Set([2, 3]);
    const s2 = new Set([1, 2]);
    var second = Object.is(s1.size, 2) + ":" + [...s1.union(s2)].join("|");
  }
  return first + ":" + second;
})()
