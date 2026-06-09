(function() {
  var constructed = new RegExp('');
  var constructedDescriptor = Object.getOwnPropertyDescriptor(constructed, 'lastIndex');
  var literal = /./g;
  var literalDescriptor = Object.getOwnPropertyDescriptor(literal, 'lastIndex');
  return [
    constructed.lastIndex,
    constructedDescriptor.writable,
    constructedDescriptor.enumerable,
    constructedDescriptor.configurable,
    literal.lastIndex,
    literalDescriptor.writable,
    literalDescriptor.enumerable,
    literalDescriptor.configurable
  ].join(':');
})()
