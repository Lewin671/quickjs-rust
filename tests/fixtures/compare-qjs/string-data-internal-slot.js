(function () {
  var s = new String('abc'), key = '\0StringData';
  function read(o) { return o[key]; }
  for (var i = 0; i < 50; i++) read(s);
  var result = [String(read(s)), key in s, s.hasOwnProperty(key)];
  var inherited = Object.create(s);
  result.push(String(read(inherited)));
  s[key] = 'xyz';
  result.push(String(s), s.length, s[0], read(s), JSON.stringify(Object.getOwnPropertyNames(s)));
  var fake = {}; fake[key] = 'forged';
  try { String.prototype.valueOf.call(fake); result.push('forged'); }
  catch (e) { result.push(e instanceof TypeError); }
  return result.join('|');
})()
