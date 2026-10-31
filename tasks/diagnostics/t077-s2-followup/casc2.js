// class prototype used as base by many prototype-mode objects
function Base() {} Base.prototype.m = function () { return 1; };
var keep = [];
for (var i = 0; i < 1e5; i++) { var p = Object.create(Base.prototype); keep.push(Object.create(p)); }
var t0 = Date.now();
for (var j = 0; j < 1000; j++) { Base.prototype.m = function () { return j; }; }
for (var j = 0; j < 200; j++) { Base.prototype['k' + j] = j; }
print('cascade2 ms', Date.now() - t0);
