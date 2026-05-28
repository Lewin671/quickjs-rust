(function() { var object = { value: 7, add(a, b) { return this.value + a + b; } }; var method = { method() {} }.method; return object.add(2, 3) + ":" + (method.prototype === undefined); })()
