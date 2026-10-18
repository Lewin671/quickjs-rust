(function () {
function check(condition, label) { if (!condition) throw new Error(label); }
function descriptors(receiver) {
    receiver.x = 1;
    var before = Object.getOwnPropertyDescriptor(receiver, 'x');
    check(before.value === 1 && before.writable, 'initial data');
    var seen;
    Object.defineProperty(receiver, 'x', {
        get: function () { seen = this; this.added = 7; return this.added; },
        enumerable: false, configurable: true
    });
    check(receiver.x === 7 && seen === receiver, 'getter receiver/reentry');
    Object.defineProperty(receiver, 'x', {value: 3, writable: true, enumerable: true, configurable: true});
    delete receiver.x;
    receiver.x = 4;
    check(receiver.x === 4, 'reinsert');
    Object.seal(receiver);
    check(!Reflect.deleteProperty(receiver, 'x') && !Object.isExtensible(receiver), 'seal');
    receiver.x = 5;
    Object.freeze(receiver);
    check(!Reflect.set(receiver, 'x', 6) && receiver.x === 5, 'freeze');
}
function C() { this.x = 0; this.y = 1; }
var many = {};
for (var i = 0; i < 16; i++) many['p' + i] = i;
var receivers = [new C(), {}, {x: 0, y: 1}, {x: 0, y: 1, z: 2}, many,
    [], new String('abc'), function named() {}, new Map(), new Set(), new Uint8Array(3)];
for (var j = 0; j < receivers.length; j++) {
    if (j === receivers.length - 1) {
        /* Integer-indexed elements retain their exotic freeze rejection. */
        var typed = receivers[j]; typed.extra = 1;
        Object.defineProperty(typed, 'extra', {get: function () { this.other = 8; return this.other; }, configurable: true});
        check(typed.extra === 8 && !Reflect.defineProperty(typed, '9', {value: 2}), 'typed exotic');
    } else descriptors(receivers[j]);
}
var halves = '\uD83D' + '\uDE00';
var literal = '\u{1F600}';
var keys = [{ '\u{1F600}': 11, spare: 0 }, new C(), [], function () {}];
for (var k = 0; k < keys.length; k++) {
    var o = keys[k];
    o[literal] = 12;
    check(o[halves] === 12, 'surrogate pair lookup');
    o[halves] = 13;
    check(o[literal] === 13, 'surrogate pair write');
    o['\uD83D'] = 'lone';
    check(o['\uDE00'] === undefined && o['\uD83D'] === 'lone', 'lone distinction');
    delete o[halves];
    check(!(literal in o), 'surrogate deletion');
}
var order = { a: 1, b: 2, c: 3 };
delete order.b; order.b = 4; order[10] = 1; order[2] = 2;
check(Object.keys(order).join() === '2,10,a,c,b', 'enumeration');
var orderedArray = []; orderedArray.b = 1; orderedArray.a = 2;
check(Object.keys(orderedArray).join() === 'b,a', 'array string insertion order');
delete orderedArray.b; orderedArray.b = 3;
check(Object.getOwnPropertyNames(orderedArray).join() === 'length,a,b', 'array reinsert order');
var proto = { x: 1 };
var child = Object.create(proto);
check(child.x === 1, 'prototype initial');
Object.defineProperty(proto, 'x', {get: function () { this.own = 9; return this.own; }});
check(child.x === 9 && child.own === 9, 'prototype accessor receiver');
Object.setPrototypeOf(child, {x: 8});
check(child.x === 8, 'prototype replacement');
var gets = 0, conversions = 0;
var proxy = new Proxy({}, { get: function (t, name, r) {
    gets++; Object.defineProperty(t, name, {value: 19, writable: true, configurable: true});
    return Reflect.get(t, name, r);
}});
var key = { toString: function () { conversions++; return 'x'; } };
check(proxy[key] === 19 && gets === 1 && conversions === 1, 'proxy/key reentry');
var foreign = Function('return {x: 20, y: 0}')();
check(foreign.x === 20 && eval('foreign.x') === 20, 'foreign code');
for (var n = 0; n < 1000; n++) { foreign['temp' + n] = n; delete foreign['temp' + n]; }
check(Object.keys(foreign).join() === 'x,y', 'churn keys');
var boxed = new String('x'.repeat(2048));
check(Object.keys(boxed).length === 2048 && boxed[2047] === 'x', 'large lazy string indices');
check(!Reflect.set(boxed, '0', 'changed'), 'string index write rejection');
Object.freeze(boxed);
check(Object.isFrozen(boxed), 'large string freeze');
var sparse = [];
for (var s = 0; s < 80; s++) Object.defineProperty(sparse, String(1000000 + s), {value: s, configurable: true});
sparse.b = 1; sparse.a = 2; sparse.length = 0;
check(sparse[1000000] === undefined && Object.keys(sparse).join() === 'b,a', 'sparse deletion/moved named slots');
return 'layout mutations ok';
})()
