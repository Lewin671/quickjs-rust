// Derived from: test/built-ins/Object/groupBy/evenOdd.js
// Derived from: test/built-ins/Object/groupBy/null-prototype.js
// Derived from: test/built-ins/Map/groupBy/evenOdd.js
var objectGroups = Object.groupBy([1, 2, 3, 4], function(value) {
  return value % 2 ? 'odd' : 'even';
});
if (Object.getPrototypeOf(objectGroups) !== null) {
  throw 'expected Object.groupBy to return a null-prototype object';
}
if (objectGroups.odd.join('|') !== '1|3' || objectGroups.even.join('|') !== '2|4') {
  throw 'expected Object.groupBy to collect grouped arrays';
}

var seen = '';
var mapGroups = Map.groupBy({ 0: 'a', 1: 'b', length: 2 }, function(value, index) {
  seen += value + index;
  return index;
});
if (!(mapGroups instanceof Map)) {
  throw 'expected Map.groupBy to return a Map';
}
if (mapGroups.get(0)[0] !== 'a' || mapGroups.get(1)[0] !== 'b' || seen !== 'a0b1') {
  throw 'expected Map.groupBy to collect values by callback keys';
}

var key = {};
var identityGroups = Map.groupBy(['x', 'y'], function(value) {
  return value === 'x' ? key : {};
});
if (identityGroups.get(key)[0] !== 'x' || identityGroups.size !== 2) {
  throw 'expected Map.groupBy to preserve object key identity';
}
