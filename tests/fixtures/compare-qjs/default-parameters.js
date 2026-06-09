(function() {
  function pick(a, b = a + 1) { return b; }
  let arrow = (a, b = 4,) => a + b;
  return [
    pick(3),
    pick(3, 8),
    pick.length,
    arrow(3, undefined),
    arrow(3, 5),
    arrow.length
  ].join("|");
})()
