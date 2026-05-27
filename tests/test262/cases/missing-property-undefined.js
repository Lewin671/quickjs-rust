// Derived from: test/language/expressions/property-accessors/S8.12.3_A2.js
var obj = {};
if (obj.propFoo !== undefined) { throw; }
if (obj["propFoo"] !== undefined) { throw; }
