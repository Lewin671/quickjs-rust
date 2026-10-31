(function () {
  var ordinary = /(a)/.exec('a');
  var indexed = /(?<letter>a)/d.exec('a');
  return Object.keys(ordinary).join('|') + ':' + Object.getOwnPropertyNames(ordinary).join('|') + ':' + Object.keys(indexed).join('|') + ':' + Object.getOwnPropertyNames(indexed).join('|');
})()
