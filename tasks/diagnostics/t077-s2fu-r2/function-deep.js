var p = {}; for (var i = 0; i < 100000; i++) p = Object.create(p);
var f = function () {}; var result = Reflect.setPrototypeOf(f, p);
if (!result || Object.getPrototypeOf(f) !== p) throw new Error('function target');
print('function-deep', result);
