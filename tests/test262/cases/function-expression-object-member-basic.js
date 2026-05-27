// Derived from: test/language/expressions/call/with-base-obj.js
var object = {
  value: 7,
  method: function() {
    return this.value;
  }
};

if (object.method() !== 7) { throw; }
