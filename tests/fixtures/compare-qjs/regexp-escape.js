(function () {
  function codes(value) {
    var result = "";
    for (var i = 0; i < value.length; i = i + 1) {
      result = result + (i === 0 ? "" : ",") + value.charCodeAt(i);
    }
    return result;
  }

  return typeof RegExp.escape + ":" +
    RegExp.escape.length + ":" +
    codes(RegExp.escape("abc123")) + ":" +
    codes(RegExp.escape("^$\\.*+?()[]{}|/")) + ":" +
    codes(RegExp.escape(",-=<>#&!%:;@~'`\"")) + ":" +
    codes(RegExp.escape("\t\n\v\f\r ")) + ":" +
    codes(RegExp.escape(String.fromCharCode(0x00a0, 0x2028, 0xfeff))) + ":" +
    codes(RegExp.escape("\ud800\udc00")) + ":" +
    codes(RegExp.escape(String.fromCharCode(0x100)));
})()
