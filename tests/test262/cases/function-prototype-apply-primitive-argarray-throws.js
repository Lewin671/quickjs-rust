// Derived from: test/built-ins/Function/prototype/apply/argarray-not-object.js
function fn() {}

function assertApplyThrows(value) {
  var caught = false;
  try {
    fn.apply(null, value);
  } catch (error) {
    caught = error instanceof TypeError;
  }

  if (!caught) { throw; }
}

assertApplyThrows(true);
assertApplyThrows(NaN);
assertApplyThrows("1,2,3");
assertApplyThrows(Symbol());
