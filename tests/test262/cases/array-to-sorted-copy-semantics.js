// Derived from: test/built-ins/Array/prototype/toSorted/comparefn-called-after-get-elements.js
// Derived from: test/built-ins/Array/prototype/toSorted/holes-not-preserved.js
var getCalls = [];
var arrayLike = {
  length: 3,
  get 0() {
    getCalls.push(0);
    return 2;
  },
  get 1() {
    getCalls.push(1);
    return 1;
  },
  get 2() {
    getCalls.push(2);
    return 3;
  }
};

try {
  Array.prototype.toSorted.call(arrayLike, function() {
    throw "compare failed";
  });
  throw "expected compare failure";
} catch (error) {
  if (error !== "compare failed") {
    throw error;
  }
}

if (getCalls.join(",") !== "0,1,2") {
  throw "Array.prototype.toSorted should read all values before comparefn";
}

var array = [3, , 4, , 1];
Array.prototype[3] = 2;
var sorted = array.toSorted();
delete Array.prototype[3];

if (sorted.join("|") !== "1|2|3|4|") {
  throw "Array.prototype.toSorted should sort copied values and undefined holes";
}
if (!sorted.hasOwnProperty("4")) {
  throw "Array.prototype.toSorted result should contain own properties for copied holes";
}
