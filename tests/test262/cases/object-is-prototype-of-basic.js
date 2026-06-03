// Derived from: test/built-ins/Object/prototype/isPrototypeOf/this-value-is-in-prototype-chain-of-arg.js
var proto = { marker: 1 };
var object = Object.create(proto);
if (typeof Object.prototype.isPrototypeOf !== "function") { throw; }
if (!proto.isPrototypeOf(object)) { throw; }
if (!Object.prototype.isPrototypeOf(object)) { throw; }
if (object.isPrototypeOf(proto)) { throw; }

function USER_FACTORY(name) {
  this.name = name;
}

function FORCEDUSER_FACTORY(name, grade) {
  this.name = name;
  this.grade = grade;
}

var userProto = new USER_FACTORY("noname");
FORCEDUSER_FACTORY.prototype = userProto;
var luke = new FORCEDUSER_FACTORY("Luke Skywalker", 12);

if (!userProto.isPrototypeOf(luke)) { throw; }
if (!USER_FACTORY.prototype.isPrototypeOf(luke)) { throw; }
if (Number.isPrototypeOf(luke)) { throw; }
