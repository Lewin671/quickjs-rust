// Derived from: test/built-ins/Array/prototype/reverse/call-with-boolean.js
// Derived from: test/built-ins/Array/prototype/reverse/S15.4.4.8_A2_T1.js
// Derived from: test/built-ins/Array/prototype/reverse/get_if_present_with_delete.js
// Derived from: test/built-ins/Array/prototype/reverse/S15.4.4.8_A4_T1.js
if (!(Array.prototype.reverse.call(true) instanceof Boolean)) {
  throw "Array.prototype.reverse should return a boxed boolean receiver";
}

var object = { length: 4, 0: "a", 2: "c" };
var result = Array.prototype.reverse.call(object);
if (result !== object) {
  throw "Array.prototype.reverse should return the generic receiver";
}
if (object[1] !== "c" || object[3] !== "a") {
  throw "Array.prototype.reverse should move present generic properties";
}
if (object.hasOwnProperty("0") || object.hasOwnProperty("2")) {
  throw "Array.prototype.reverse should preserve holes on generic receivers";
}

var getterArray = ["first", "second"];
Object.defineProperty(getterArray, "0", {
  get: function() {
    getterArray.length = 0;
    return "first";
  },
  configurable: true
});
getterArray.reverse();
if ((0 in getterArray) || !(1 in getterArray) || getterArray[1] !== "first") {
  throw "Array.prototype.reverse should observe deletions during indexed gets";
}

Array.prototype[1] = 1;
var inheritedArray = [0];
inheritedArray.length = 2;
inheritedArray.reverse();
var inheritedOk = inheritedArray[0] === 1
  && inheritedArray[1] === 0
  && inheritedArray.hasOwnProperty("0")
  && inheritedArray.hasOwnProperty("1");
delete Array.prototype[1];
if (!inheritedOk) {
  throw "Array.prototype.reverse should read inherited indexed properties";
}
