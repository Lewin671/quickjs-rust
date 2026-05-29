(function () {
  return (null == undefined) + ":" +
    (null != undefined) + ":" +
    ("1" == 1) + ":" +
    (1 == "1") + ":" +
    (true == 1) + ":" +
    (false == 0) + ":" +
    (false == "") + ":" +
    (NaN == NaN) + ":" +
    ("x" == 1) + ":" +
    ("1" === 1);
})()
