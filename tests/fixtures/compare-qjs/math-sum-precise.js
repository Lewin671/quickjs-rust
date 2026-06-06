[
  Math.sumPrecise.length,
  Math.sumPrecise.name,
  Math.sumPrecise([1, 2, 3]),
  Math.sumPrecise([1e30, 0.1, -1e30]),
  Math.sumPrecise([1e308, 1e308, 0.1, 0.1, 1e30, 0.1, -1e30, -1e308, -1e308]),
  1 / Math.sumPrecise([]),
  Math.sumPrecise([Infinity, -Infinity]) === Math.sumPrecise([Infinity, -Infinity])
].join(":")
