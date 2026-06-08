(function() {
  var xs = [1, 2, 3, 4, 5];
  var result = xs.copyWithin(0, 3);
  var ys = [1, 2, 3, 4].copyWithin(1, 0, 3);
  var zs = [1, 2, 3, 4, 5].copyWithin(-2, 0, 2);

  var shortened = [0, 1, 2, 3];
  shortened.copyWithin(0, {
    valueOf: function() {
      shortened.length = 2;
      return 3;
    }
  });

  var proto = { 3: 9 };
  var inherited = [0, 1, 2, 3];
  Object.setPrototypeOf(inherited, proto);
  Array.prototype.copyWithin.call(inherited, 0, {
    valueOf: function() {
      inherited.length = 2;
      return 3;
    }
  });

  var arrayProto = [0, 1, 2, 3, 4];
  var arrayInherited = [0, 1, 2, 3, 4];
  Object.setPrototypeOf(arrayInherited, arrayProto);
  arrayInherited.copyWithin(0, {
    valueOf: function() {
      arrayInherited.length = 2;
      return 3;
    }
  });

  return (result === xs)
    + ":" + xs.join()
    + ":" + ys.join()
    + ":" + zs.join()
    + ":" + shortened.length
    + ":" + shortened.hasOwnProperty("0")
    + ":" + shortened[1]
    + ":" + inherited.length
    + ":" + inherited[0]
    + ":" + inherited[1]
    + ":" + arrayInherited.length
    + ":" + arrayInherited[0]
    + ":" + arrayInherited[1];
})()
